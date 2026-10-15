# Typst IDE Desktop App — Design

**Date:** 2026-10-15
**Status:** Approved

A desktop IDE for writing and compiling Typst documents: Tauri v2 shell, Rust Typst compiler backend, Vue 3 + shadcn-vue frontend, CodeMirror 6 editor.

## Locked decisions

| Topic | Decision |
|---|---|
| Editor | CodeMirror 6 + `@vedivad/codemirror-typst` (WASM). Its live preview / own diagnostics / PDF export are **disabled** — the Rust backend is the single source of truth for compilation. WASM engine used for syntax highlighting, completion, hover only. |
| Compile with dirty buffers | Frontend prompts **Save all / Discard all / Cancel** before invoking `compile`. Compile always reads from disk (one code path; the Typst world never sees in-memory buffers). |
| PDF output | `<main-stem>.pdf` written **next to the main `.typ` file** (typst CLI default). |
| PDF preview pane | **Out of scope for v1.** Post-compile action: "Open in system viewer" via `tauri-plugin-opener`. |
| Recent projects / per-project settings | `tauri-plugin-store` JSON in the app config dir (never inside the project folder). |
| Main-file deletion fallback | Auto-fall back to first `.typ` (prefer `main.typ`), log a warning. |

## Verified versions (Oct 2026)

- `tauri` 2.12, `tauri-plugin-dialog` 2.8, `tauri-plugin-store` 2.5, `tauri-plugin-opener` 2
- `typst`, `typst-layout`, `typst-pdf`, `typst-kit` **=0.15.1** (pin exactly; 0.14↔0.15 incompatible)
- `notify` 8.2 + `notify-debouncer-full` 0.7
- Vue 3.5, Pinia 4.1 (requires `@vue/devtools-api` peer), VueUse 15, shadcn-vue 2.8 (new-york-v4, Tailwind 4, reka-ui 2.11)
- `@vedivad/codemirror-typst` 0.18.8 (needs `vite-plugin-wasm` + possibly top-level-await; spike first, fallback `codemirror-lang-typst` 0.6 pure-JS Lezer)
- Rust 1.92+ (typst 0.15 MSRV); repo uses Yarn PnP

## Layout

```
┌──────────────────────────────────────────────────────┐
│ Top bar: [Main file ▾] [▶ Compile]  [project ▾]      │
├────────────┬─────────────────────────────────────────┤
│ FileTree   │ CodeEditor (CodeMirror 6)               │
├────────────┴─────────────────────────────────────────┤
│ Status bar: latest log · process dot · expand ▴      │
│ (expanded → scrollable LogPanel)                     │
└──────────────────────────────────────────────────────┘
```

Resizable panels (`ResizablePanelGroup/Panel/Handle`) between tree and editor. Project modal (`Dialog`) on launch / no-project; non-dismissible while no project is open.

## Architecture

### Rust (`src-tauri/src/`)

```
lib.rs                 builder: plugins (opener, dialog, store), AppState, invoke_handler
state.rs               AppState { project: Mutex<Option<ProjectState>>, fonts: Arc<LazyLock<FontStore>>, compile_gen: AtomicU32 }
error.rs               AppError → structured serde payload { kind, message, path? }
events.rs              typed emit helpers: log(), process_status(), fs_changed()
commands/project.rs    open_project, create_project, close_project, get_project_info, set_main_file, path_exists
commands/files.rs      read_dir_tree, read_file, write_file, create_entry, rename_entry, delete_entry
commands/compile.rs    compile (async → spawn_blocking; emits log/process-status/compile-finished)
watcher.rs             notify-debouncer-full → fs-changed events
typst_engine/world.rs  ProjectWorld: impl typst::World + DiagnosticWorld
typst_engine/compile.rs compile → PagedDocument → pdf bytes → Diagnostic[]
```

All file ops go through `resolve_in_root(root, rel)`: reject absolute paths and `..`, `canonicalize` + `starts_with(root)` to block symlink escapes. All paths exposed to the frontend are root-relative, `/`-separated.

### Typst World (0.15.1 API)

- `ProjectWorld { root, main: FileId, library: LazyHash<Library>, fonts: Arc<LazyLock<FontStore>>, files: FileStore<SystemFiles>, now: Time }`
- `main` file id: `RootedPath::new(VirtualRoot::Project, VirtualPath::virtualize(&root, &main_abs)?).intern()`
- **Files:** `typst-kit` `SystemFiles` (feature `system-files`) resolves project files + `#import "@preview/…"` via `SystemPackages::obtain` (downloads/caches to `~/.local/share/typst/packages`, `~/.cache/typst/packages`; features `system-packages`, `system-downloader`, `universe-packages`).
- **Fonts:** `FontStore` = `fonts::system()` (feature `scan-fonts`) + `fonts::embedded()` (feature `embedded-fonts`; Libertinus, New Computer Modern, DejaVu Mono). Built **once per app** behind `Arc<LazyLock<…>>` in AppState, reused across compiles (comemo cache + no re-scan).
- `typst-kit` has **empty default features** — all features listed explicitly. `typst-layout` is a direct dep (provides `PagedDocument`; not re-exported by `typst`).
- Compile: `typst::compile::<PagedDocument>(&world) -> Warned<SourceResult<PagedDocument>>`; PDF: `typst_pdf::pdf(&doc, &PdfOptions::default())`.
- Diagnostics: `diag.span.id()` → file; `WorldExt::range(span)` → byte range; `Source::lines().byte_to_line_column` → 1-based line/col.

### IPC contract

Commands: `open_project(path) -> ProjectInfo`, `create_project(parent, name) -> ProjectInfo`, `close_project()`, `get_project_info() -> ProjectInfo?`, `set_main_file(rel)`, `path_exists(path) -> bool`, `read_dir_tree() -> FileNode`, `read_file(rel) -> FileContent`, `write_file(rel, content) -> FileMeta`, `create_entry(rel, kind)`, `rename_entry(old, new)`, `delete_entry(rel)`, `compile() -> CompileResult`.

Events: `log { timestamp, level, message, context? }`, `process-status { process: "compile", status, duration_ms? }`, `fs-changed { kind, paths[] }`, `compile-finished: CompileResult`.

Types (mirrored in `src/types/ipc.ts`): `FileNode { name, path, kind, children? }`, `ProjectInfo { root, name, main_file, typ_files }`, `FileContent { path, content, mtime_ms }`, `Diagnostic { severity, message, path?, line?, column?, offset?, length?, hints }`, `CompileResult { success, pdf_path?, duration_ms, diagnostics }`.

`compile()` guards against overlap with a generation counter (stale results discarded). Not cancellable mid-run (Typst limitation).

### Frontend (`src/`)

```
components/ui/…                shadcn-vue primitives (dialog, alert-dialog, input, select,
                               context-menu, dropdown-menu, resizable, tooltip, scroll-area,
                               separator, badge, skeleton, button)
components/ui/file-tree/       custom shadcn-style component:
  FileTree.vue                 props { nodes, selectedPath?, loading?, disabled? }
                               emits select/open/create/rename/delete/refresh
                               slots #node-icon, #empty
  FileTreeItem.vue             recursive row
  useFileTree.ts               expansion Set, roving tabindex, type-ahead
  ARIA: tree/treeitem, aria-expanded/selected/level
  Keyboard: ↑↓ move, → expand, ← collapse, Enter open, F2 rename, Delete delete
components/layout/TopBar.vue   main-file Select, ▶ Compile (spinner while compiling), project switcher
components/layout/StatusBar.vue latest log line, process dot, expand toggle
components/logs/LogPanel.vue   timestamps, level badges, diagnostics clickable → jump
components/project/ProjectModal.vue  create/open/recents; non-dismissible without project
components/editor/CodeEditor.vue     CM6 wrapper: v-model, readOnly, jumpTo {offset,length,nonce}
composables/useCodeMirror.ts   CM6 lifecycle, keymap (Ctrl+S)
composables/useTypstLanguage.ts wires @vedivad language features only
stores/project.ts              root, mainFile, typFiles, tree, recents, modalOpen
stores/editor.ts               open file, per-path buffers { content, dirty, mtime }, jump requests
stores/logs.ts                 entries[], processes, panelExpanded
types/ipc.ts, types/models.ts
```

Global shortcuts: Ctrl+S save, Ctrl+Enter compile, Ctrl+O open project.

## Implementation phases

1. **Scaffold + static layout** — Pinia, shadcn components, App shell with resizable panels, placeholder panes. Runnable: `tauri dev` shows layout.
2. **Project modal + file tree** — open/create project, recents, FileTree CRUD. Testable against a real folder.
3. **Editor load/save** — CM6 wrapper, buffers, dirty tracking, Ctrl+S, empty state.
4. **Main file + compile** — typst deps, World, compile command, TopBar select + ▶, Save/Discard/Cancel dialog, WASM editor language spike.
5. **Logs + diagnostics** — events, StatusBar/LogPanel, clickable diagnostics → editor jump, generation guard.
6. **Polish** — notify watcher → fs-changed → tree refresh (skip self-writes via mtime), external-change banner, per-project persistence (main file, expanded dirs), shortcuts, unsaved-changes guard on project switch.

## Risks

- `@vedivad/codemirror-typst` WASM + Vite 8 bundling may fight; fallback `codemirror-lang-typst`. Spike on day 1 of Phase 4.
- typst 0.15 API churn — pin `=0.15.1`, mirror `typst-cli/src/world.rs` at tag v0.15.1.
- Compile latency: font scan once per app; package downloads cached on disk; all off main thread.
- Symlink/path traversal: canonicalize + prefix check on every command.
- Windows path separators: normalize to `/` at the Rust boundary.
