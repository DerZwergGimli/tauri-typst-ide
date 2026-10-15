import { onBeforeUnmount, onMounted } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { useLogsStore } from "@/stores/logs";
import { useEditorStore } from "@/stores/editor";
import { useProjectStore } from "@/stores/project";
import type {
  FileContent,
  FileMeta,
  FsChangedEvent,
  LogPayload,
  ProcessStatusEvent,
} from "@/types/models";

/**
 * Bridges Rust-side events (`log`, `process-status`, `fs-changed`) into the
 * frontend stores. Must be called from a component's setup so listeners are
 * cleaned up on unmount.
 */
export function useTauriEvents() {
  const logs = useLogsStore();
  const editor = useEditorStore();
  const project = useProjectStore();
  const unlisteners: UnlistenFn[] = [];

  let refreshTimer: ReturnType<typeof setTimeout> | null = null;
  function scheduleTreeRefresh() {
    if (refreshTimer !== null) clearTimeout(refreshTimer);
    refreshTimer = setTimeout(() => {
      refreshTimer = null;
      if (project.hasProject) void project.refreshTree();
    }, 150);
  }

  /**
   * Decide whether a change on disk came from us or from an external editor.
   * Self-writes are detected by mtime: after saving, the buffer's mtimeMs is
   * updated to the file's new mtime, so an equal mtime means "our own write".
   */
  async function handleFsChange(path: string, kind: FsChangedEvent["kind"]) {
    const buf = editor.buffers.get(path);
    if (!buf) return;

    if (kind === "removed") {
      editor.flagExternalChange(path, "deleted");
      return;
    }

    let meta: FileMeta | null = null;
    try {
      meta = await invoke<FileMeta | null>("file_mtime", { relPath: path });
    } catch {
      return;
    }
    if (!meta) {
      editor.flagExternalChange(path, "deleted");
      return;
    }
    // Our own write (mtime unchanged since we saved) — nothing to do.
    if (meta.mtime_ms <= buf.mtimeMs) return;

    const isClean = buf.content === buf.savedContent;
    if (isClean) {
      try {
        const file = await invoke<FileContent>("read_file", { relPath: path });
        editor.reloadBuffer(path, file.content, file.mtime_ms);
      } catch {
        // ignore
      }
    } else {
      editor.flagExternalChange(path, "modified");
    }
  }

  onMounted(async () => {
    unlisteners.push(
      await listen<LogPayload>("log", (event) => {
        logs.push({
          timestamp: new Date().toISOString(),
          level: event.payload.level,
          message: event.payload.message,
          context: event.payload.context,
        });
      }),
    );

    unlisteners.push(
      await listen<ProcessStatusEvent>("process-status", (event) => {
        logs.setProcessStatus(event.payload.process, event.payload.status);
      }),
    );

    unlisteners.push(
      await listen<FsChangedEvent>("fs-changed", (event) => {
        scheduleTreeRefresh();
        for (const path of event.payload.paths) {
          void handleFsChange(path, event.payload.kind);
        }
      }),
    );
  });

  onBeforeUnmount(() => {
    if (refreshTimer !== null) clearTimeout(refreshTimer);
    for (const unlisten of unlisteners) unlisten();
    unlisteners.length = 0;
  });
}
