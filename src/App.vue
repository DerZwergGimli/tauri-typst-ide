<script setup lang="ts">
import { onMounted } from "vue";
import { storeToRefs } from "pinia";
import TopBar from "@/components/layout/TopBar.vue";
import StatusBar from "@/components/layout/StatusBar.vue";
import FileTreePane from "@/components/layout/FileTreePane.vue";
import LogPanel from "@/components/logs/LogPanel.vue";
import EditorPane from "@/components/editor/EditorPane.vue";
import ProjectModal from "@/components/project/ProjectModal.vue";
import SaveChangesDialog from "@/components/dialogs/SaveChangesDialog.vue";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import { useProjectStore } from "@/stores/project";
import { useEditorStore } from "@/stores/editor";
import { useCompileStore } from "@/stores/compile";
import { useTauriEvents } from "@/composables/useTauriEvents";
import { useGlobalShortcuts } from "@/composables/useGlobalShortcuts";
import { invoke } from "@tauri-apps/api/core";
import type { Diagnostic, FileContent } from "@/types/models";

const project = useProjectStore();
const editor = useEditorStore();
const compile = useCompileStore();
const { modalOpen, hasProject } = storeToRefs(project);

useTauriEvents();
useGlobalShortcuts();

onMounted(() => {
  void project.loadRecents();
  // Phase 2: always show the modal on launch; later: restore last project
  modalOpen.value = true;
});

function onCompile() {
  void compile.compile();
}

async function openDiagnostic(d: Diagnostic) {
  if (!d.path) return;
  if (d.offset !== null) {
    // Ensure the file is open before jumping.
    if (!editor.buffers.has(d.path)) {
      try {
        const file = await invoke<FileContent>("read_file", {
          relPath: d.path,
        });
        editor.openFile(d.path, file.content, file.mtime_ms);
      } catch {
        return;
      }
    } else {
      editor.openPath = d.path;
    }
    editor.jumpTo(d.offset, d.length ?? 0);
  } else if (!editor.buffers.has(d.path)) {
    const file = await invoke<FileContent>("read_file", { relPath: d.path });
    editor.openFile(d.path, file.content, file.mtime_ms);
  } else {
    editor.openPath = d.path;
  }
}
</script>

<template>
  <div class="flex h-screen flex-col overflow-hidden ">
    <TopBar @compile="onCompile" />

    <ResizablePanelGroup direction="horizontal" class="min-h-0 flex-1">
      <ResizablePanel :default-size="20" :min-size="12" :max-size="40">
        <FileTreePane />
      </ResizablePanel>
      <ResizableHandle with-handle />
      <ResizablePanel :default-size="80">
        <main class="h-full min-h-0 overflow-hidden">
          <EditorPane />
        </main>
      </ResizablePanel>
    </ResizablePanelGroup>
    <LogPanel @open-diagnostic="openDiagnostic" />
    <StatusBar />
    <ProjectModal v-model:open="modalOpen" :dismissible="hasProject" />
    <SaveChangesDialog />
  </div>
</template>
