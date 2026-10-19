import { ref } from "vue";
import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { useEditorStore } from "@/stores/editor";
import { useLogsStore } from "@/stores/logs";
import type { CompileResult, FileContentBytes } from "@/types/models";

export type SaveChoice = "save" | "discard" | "cancel";

export const useCompileStore = defineStore("compile", () => {
  const promptOpen = ref(false);
  const promptMessage = ref("Save changes before compiling?");
  const lastResult = ref<CompileResult | null>(null);
  const latestPdfPath = ref<string | null>(null);

  let resolver: ((choice: SaveChoice) => void) | null = null;

  function askSaveChanges(message: string): Promise<SaveChoice> {
    promptMessage.value = message;
    promptOpen.value = true;
    return new Promise((resolve) => {
      resolver = resolve;
    });
  }

  function choose(choice: SaveChoice) {
    promptOpen.value = false;
    resolver?.(choice);
    resolver = null;
  }

  /**
   * Save or discard unsaved buffers according to `choice`. Returns false when
   * the user cancelled (caller should abort).
   */
  async function resolveDirtyBuffers(choice: SaveChoice): Promise<boolean> {
    const editor = useEditorStore();
    if (choice === "cancel") return false;
    if (choice === "save") await editor.saveAllDirty();
    else await editor.discardAllDirty();
    return true;
  }

  /**
   * Prompt to save dirty buffers if needed. Returns false when cancelled.
   */
  async function ensureSaved(
    message = "Save changes before compiling?",
  ): Promise<boolean> {
    const editor = useEditorStore();
    if (editor.dirtyPaths.length === 0) return true;
    const choice = await askSaveChanges(message);
    return resolveDirtyBuffers(choice);
  }

  /**
   * Compile the current main file, prompting to resolve unsaved buffers first.
   */
  async function compile(): Promise<CompileResult | null> {
    const logs = useLogsStore();

    if (!(await ensureSaved())) return null;

    try {
      const result = await invoke<CompileResult>("run_build_pipeline");
      lastResult.value = result;
      if (result.pdf_path) {
        latestPdfPath.value = result.pdf_path;
      } else if (result.success) {
        latestPdfPath.value = null;
      }
      logs.setDiagnostics(result.diagnostics);
      return result;
    } catch (e) {
      logs.push({
        timestamp: new Date().toISOString(),
        level: "error",
        message: `compile failed: ${String(e)}`,
      });
      return null;
    }
  }

  async function openLatestPdf() {
    const logs = useLogsStore();
    const editor = useEditorStore();
    const path = latestPdfPath.value;
    if (!path) return;
    try {
      const res = await invoke<FileContentBytes>("read_file_bytes", { relPath: path });
      editor.openPreview(path, Uint8Array.from(res.content).buffer);
    } catch (e) {
      logs.push({
        timestamp: new Date().toISOString(),
        level: "error",
        message: `open pdf failed: ${String(e)}`,
      });
    }
  }

  function reset() {
    lastResult.value = null;
    latestPdfPath.value = null;
  }

  return {
    promptOpen,
    promptMessage,
    lastResult,
    latestPdfPath,
    choose,
    compile,
    openLatestPdf,
    reset,
    ensureSaved,
    resolveDirtyBuffers,
  };
});
