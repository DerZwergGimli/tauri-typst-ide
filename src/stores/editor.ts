import { computed, reactive, ref } from "vue";
import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import type { FileContent, FileMeta } from "@/types/models";

export interface OpenBuffer {
  path: string;
  content: string;
  savedContent: string;
  mtimeMs: number;
}

export type ExternalChangeKind = "modified" | "deleted";

export const useEditorStore = defineStore("editor", () => {
  const openPath = ref<string | null>(null);
  const buffers = reactive(new Map<string, OpenBuffer>());
  const jumpRequest = ref<{ offset: number; length: number; nonce: number } | null>(
    null,
  );
  /** Paths that changed on disk while open in the editor. */
  const externalChanges = reactive(new Map<string, ExternalChangeKind>());

  const openBuffer = computed(() =>
    openPath.value ? buffers.get(openPath.value) ?? null : null,
  );
  const isDirty = computed(
    () => openBuffer.value !== null && openBuffer.value.content !== openBuffer.value.savedContent,
  );
  const dirtyPaths = computed(() =>
    [...buffers.values()]
      .filter((b) => b.content !== b.savedContent)
      .map((b) => b.path),
  );
  const openExternalChange = computed(() =>
    openPath.value ? externalChanges.get(openPath.value) ?? null : null,
  );

  function openFile(path: string, content: string, mtimeMs: number) {
    if (!buffers.has(path)) {
      buffers.set(path, { path, content, savedContent: content, mtimeMs });
    }
    openPath.value = path;
  }

  function updateContent(path: string, content: string) {
    const buf = buffers.get(path);
    if (buf) buf.content = content;
  }

  function markSaved(path: string, mtimeMs: number) {
    const buf = buffers.get(path);
    if (buf) {
      buf.savedContent = buf.content;
      buf.mtimeMs = mtimeMs;
    }
    // Our own write means the disk and buffer agree again.
    externalChanges.delete(path);
  }

  function reloadBuffer(path: string, content: string, mtimeMs: number) {
    buffers.set(path, { path, content, savedContent: content, mtimeMs });
    externalChanges.delete(path);
  }

  function closeFile(path: string) {
    buffers.delete(path);
    externalChanges.delete(path);
    if (openPath.value === path) {
      openPath.value = buffers.size > 0 ? [...buffers.keys()][0]! : null;
    }
  }

  /** Persist one buffer to disk. Throws on failure. */
  async function saveBuffer(path: string): Promise<void> {
    const buf = buffers.get(path);
    if (!buf) return;
    const meta = await invoke<FileMeta>("write_file", {
      relPath: path,
      content: buf.content,
    });
    markSaved(path, meta.mtime_ms);
  }

  /** Persist every dirty buffer. Throws on the first failure. */
  async function saveAllDirty(): Promise<void> {
    for (const path of dirtyPaths.value) {
      await saveBuffer(path);
    }
  }

  /**
   * Reload every dirty buffer from disk, discarding unsaved edits.
   */
  async function discardAllDirty(): Promise<void> {
    for (const path of dirtyPaths.value) {
      try {
        const file = await invoke<FileContent>("read_file", { relPath: path });
        reloadBuffer(path, file.content, file.mtime_ms);
      } catch {
        // File may have been deleted externally; leave the buffer as-is.
      }
    }
  }

  function flagExternalChange(path: string, kind: ExternalChangeKind) {
    externalChanges.set(path, kind);
  }

  function clearExternalChange(path: string) {
    externalChanges.delete(path);
  }

  /** Drop all open buffers and transient state (e.g. on project switch). */
  function reset() {
    buffers.clear();
    externalChanges.clear();
    openPath.value = null;
    jumpRequest.value = null;
  }

  function jumpTo(offset: number, length: number) {
    jumpRequest.value = { offset, length, nonce: Date.now() };
  }

  return {
    openPath,
    buffers,
    jumpRequest,
    externalChanges,
    openBuffer,
    isDirty,
    dirtyPaths,
    openExternalChange,
    openFile,
    updateContent,
    markSaved,
    reloadBuffer,
    closeFile,
    saveBuffer,
    saveAllDirty,
    discardAllDirty,
    flagExternalChange,
    clearExternalChange,
    reset,
    jumpTo,
  };
});
