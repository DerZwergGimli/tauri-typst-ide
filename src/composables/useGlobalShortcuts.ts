import { useEventListener } from "@vueuse/core";
import { useCompileStore } from "@/stores/compile";
import { useProjectStore } from "@/stores/project";

/**
 * App-wide keyboard shortcuts. `Ctrl+S` inside the editor is handled by the
 * CodeMirror keymap; here we cover actions that should work regardless of
 * focus.
 */
export function useGlobalShortcuts() {
  const compile = useCompileStore();
  const project = useProjectStore();

  useEventListener(window, "keydown", (event: KeyboardEvent) => {
    if (!(event.ctrlKey || event.metaKey) || event.altKey) return;

    switch (event.key.toLowerCase()) {
      case "enter":
        event.preventDefault();
        void compile.compile();
        break;
      case "o":
        event.preventDefault();
        project.modalOpen = true;
        break;
    }
  });
}
