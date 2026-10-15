# AGENTS.md

Desktop **Typst IDE**: a Tauri v2 shell with a Rust backend that compiles Typst
via the `typst` crates, and a Vue 3 + CodeMirror 6 frontend.

## Commands

Run from the repo root unless noted.

```bash
yarn dev            # Vite dev server only (no Tauri shell)
yarn tauri dev      # full app (Vite + Rust); use this for real testing
yarn build          # REQUIRED gate: vue-tsc --noEmit && vite build
yarn tauri build    # production bundle

# Rust checks (workdir = src-tauri)
cargo check
cargo clippy -- -D warnings -A dead-code   # treat warnings as errors
cargo test
```

**Always run both gates after changes:** `yarn build` (frontend + typecheck) and
`cargo clippy -- -D warnings -A dead-code` (from `src-tauri`). There is no test
suite yet.

## Toolchain / environment

- **Yarn 4.12 with PnP** (`nodeLinker: pnp`, configured in `~/.yarnrc`). Extra
  empty `node_modules/` may exist but is unused — do NOT run `npm install`.
  Use `yarn add <pkg>` and `yarn dlx <tool>`. Vite prints a PnP warning; ignore it.
- Node 24, Rust edition **2024** (note: `gen` is a reserved keyword — use
  `generation`).
- Add shadcn-vue components with
  `yarn dlx shadcn-vue@latest add <name>` (config: `components.json`,
  new-york style, Tailwind 4, base color neutral, icons `@lucide/vue`).

## Architecture

Frontend (`src/`) — Vue 3 `<script setup>`, Pinia setup-stores, `@/` → `src/`:

- `stores/project.ts` — project root/name, main file, file tree, recents
  (persisted via `tauri-plugin-store` in `app-settings.json`), expanded-dir
  persistence (`expanded:<root>`), `refreshTree()`.
- `stores/editor.ts` — open buffers, dirty tracking, save/reload/reset,
  external-change flags, `jumpTo`.
- `stores/compile.ts` — compile flow + the shared Save/Discard/Cancel prompt
  (`ensureSaved`, `promptMessage`).
- `stores/logs.ts` — log entries, diagnostics, `isCompiling`.
- `composables/` — `useCodeMirror`, `useTauriEvents` (bridges Rust events),
  `useGlobalShortcuts`.
- `components/layout/*` — `TopBar`, `FileTreePane`, `EditorPane` shell, `StatusBar`.
- `components/ui/*` — shadcn-vue components (generated; avoid hand-editing beyond
  project-specific ones like `file-tree/`).
- `types/models.ts` — TS mirrors of Rust serde types. **Keep in sync.**
- `lib/typstLanguage.ts` — lazy Typst language support. Uses
  `codemirror-lang-typst/lezer` (`typst_lezer()`), the WASM-free subpath.

Backend (`src-tauri/src/`) — commands registered in `lib.rs`:

- `commands/project.rs` — `open_project`, `create_project`, `close_project`,
  `get_project_info`, `set_main_file`, `path_exists`. Opens/closes the FS watcher.
- `commands/files.rs` — `read_dir_tree`, `read_file`, `write_file`, `file_mtime`,
  `create_entry`, `rename_entry`, `delete_entry`.
- `commands/compile.rs` — `compile`: runs the Typst engine, maps diagnostics,
  writes `<main-stem>.pdf` next to the main `.typ`, opens it in the system viewer.
- `typst_engine/world.rs` — `ProjectWorld` impl of `typst::World`; shared font
  store via `shared_fonts()`.
- `state.rs` — `AppState` (project + compile generation guard).
- `watcher.rs` — debounced recursive watcher; emits `fs-changed`.
- `events.rs` — `EmitExt` (`log`, `log_ctx`, `process_status`).
- `error.rs` — `AppError` (serde-tagged, camelCase) + `AppResult`; `rel_from_root`.

### IPC / events

- JS → Rust: `invoke("<snake_case_command>", { camelCaseArgs })`. Tauri v2
  converts camelCase JS args to snake_case Rust params.
- Rust → JS events: `log` (`LogPayload`), `process-status`
  (`ProcessStatusPayload`), `fs-changed` (`FsChangedPayload`:
  `{ kind: "created"|"removed"|"modified", paths: string[] }`). Paths in
  `fs-changed` are **project-relative**.
- `tauri::Emitter` must be in scope to call `app.emit`.
- Calling the Rust opener API bypasses capability permission checks; the
  `opener:default` capability does not grant `allow-open-path`, but that's fine
  because it's invoked from Rust.

## Domain gotchas

**Typst 0.15.1 (pinned `=0.15.1`; 0.14 ↔ 0.15 APIs are incompatible):**

- `typst::compile::<T>(&dyn World) -> Warned<SourceResult<T>>`.
  `Warned` is at `typst::diag::Warned` (NOT `typst::Warned`), fields
  `output` / `warnings`.
- `typst_pdf::pdf(&PagedDocument, &PdfOptions::default()) -> Result<Vec<u8>, EcoVec<SourceDiagnostic>>`.
- `Library::default()` comes from the `LibraryExt` trait — import it.
- `SystemDownloader` is in `typst_kit::downloader` (not `packages`);
  `SystemPackages` is in `typst_kit::packages`.
- Diagnostics: `SourceDiagnostic { severity, span: DiagSpan, message, trace, hints }`;
  hints are `Spanned<EcoString, DiagSpan>`, value via `h.v`. `DiagSpan::id()`,
  `WorldExt::range(span)`, `src.lines().byte_to_line_column(offset)` (0-based →
  +1 for the UI).
- **Severity strings:** Rust emits `"warning"` (not `"warn"`) to match the TS
  union `"error" | "warning"`.
- Long compiles run in `tauri::async_runtime::spawn_blocking`; a generation guard
  (`AppState::begin_compile`) discards superseded results.

**Watcher:** `notify` `EventKind` is exhaustive; ignore `EventKind::Access`
(fires on every read and would spam events) and filter dotfiles / `target` /
`node_modules`.

**Self-write detection:** after saving, the buffer's `mtimeMs` is updated to the
file's new mtime, so an incoming `fs-changed` whose mtime is `<= buf.mtimeMs` is
our own write and is skipped.

## Conventions

- No comments unless asked.
- Rust: keep clippy clean under `-D warnings`; the project runs `-A dead-code`
  to allow not-yet-wired helpers.
- Prefer editing existing files; keep new docs minimal.
- Only commit when explicitly asked.
