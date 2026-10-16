<script setup lang="ts">
import { computed, defineAsyncComponent } from "vue";
import { storeToRefs } from "pinia";
import { useEditorStore } from "@/stores/editor";
import { useLogsStore } from "@/stores/logs";
import CodeEditor from "@/components/editor/CodeEditor.vue";
import EditorEmptyState from "@/components/editor/EditorEmptyState.vue";
import { Button } from "@/components/ui/button";
import { invoke } from "@tauri-apps/api/core";
import type { FileContent } from "@/types/models";
import { Dot, TriangleAlert } from "@lucide/vue";

// pdf.js (~1MB) is only fetched the first time a PDF is previewed.
const PdfPreview = defineAsyncComponent(
  () => import("@/components/editor/PdfPreview.vue"),
);

const editor = useEditorStore();
const logs = useLogsStore();
const {
  openPath,
  openBuffer,
  isDirty,
  jumpRequest,
  openExternalChange,
  previewPath,
  previewBytes,
} = storeToRefs(editor);

const showPreview = computed(
  () =>
    previewBytes.value !== null &&
    previewPath.value !== null &&
    openPath.value === previewPath.value,
);

const displayName = computed(() => {
  if (!openPath.value) return "";
  const parts = openPath.value.split("/");
  return parts[parts.length - 1];
});

function onChange(value: string) {
  if (openPath.value) editor.updateContent(openPath.value, value);
}

async function save() {
  const path = openPath.value;
  if (!path) return;
  try {
    await editor.saveBuffer(path);
    logs.push({
      timestamp: new Date().toISOString(),
      level: "info",
      message: `saved ${path}`,
    });
  } catch (e) {
    logs.push({
      timestamp: new Date().toISOString(),
      level: "error",
      message: `save failed: ${String(e)}`,
    });
  }
}

async function reloadFromDisk() {
  const path = openPath.value;
  if (!path) return;
  try {
    const file = await invoke<FileContent>("read_file", { relPath: path });
    editor.reloadBuffer(path, file.content, file.mtime_ms);
  } catch (e) {
    logs.push({
      timestamp: new Date().toISOString(),
      level: "error",
      message: `reload failed: ${String(e)}`,
    });
  }
}

function keepMine() {
  if (openPath.value) editor.clearExternalChange(openPath.value);
}
</script>

<template>
  <div class="flex h-full min-h-0 flex-col">
    <EditorEmptyState v-if="!openPath || (!openBuffer && !showPreview)" />
    <template v-else>
      <div
        class="flex h-8 shrink-0 items-center gap-1 border-b px-3 text-xs text-muted-foreground"
      >
        <span class="font-medium">{{ displayName }}</span>
        <span v-if="openPath?.includes('/')" class="truncate opacity-70">
          {{ openPath.split("/").slice(0, -1).join(" / ") }}
        </span>
        <Dot v-if="isDirty" class="size-5 shrink-0 text-amber-500" />
      </div>

      <div
        v-if="openExternalChange"
        class="flex shrink-0 items-center gap-2 border-b border-amber-500/40 bg-amber-500/10 px-3 py-1.5 text-xs"
      >
        <TriangleAlert class="size-3.5 shrink-0 text-amber-500" />
        <span class="truncate">
          {{
            openExternalChange === "deleted"
              ? "This file was deleted on disk."
              : "This file changed on disk."
          }}
        </span>
        <div class="ml-auto flex shrink-0 gap-1">
          <Button
            v-if="openExternalChange === 'modified'"
            variant="ghost"
            size="sm"
            class="h-6 px-2 text-xs"
            @click="reloadFromDisk"
          >
            Reload
          </Button>
          <Button
            variant="ghost"
            size="sm"
            class="h-6 px-2 text-xs"
            @click="keepMine"
          >
            Keep mine
          </Button>
        </div>
      </div>

      <div class="min-h-0 flex-1">
        <PdfPreview
          v-if="showPreview"
          :key="openPath!"
          :bytes="previewBytes!"
        />
        <CodeEditor
          v-else
          :key="openPath"
          :model-value="openBuffer!.content"
          @update:model-value="onChange"
          @save="save"
          :jump-to="jumpRequest"
        />
      </div>
    </template>
  </div>
</template>
