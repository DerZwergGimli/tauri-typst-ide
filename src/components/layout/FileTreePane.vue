<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { storeToRefs } from "pinia";
import { useProjectStore } from "@/stores/project";
import { useEditorStore } from "@/stores/editor";
import { FileTree } from "@/components/ui/file-tree";
import type { FileNode } from "@/types/models";
import { FolderOpen, FolderOpenDot, RefreshCcw } from "@lucide/vue";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { BackendCommands } from "@/backend/backend.ts";

const project = useProjectStore();
const editor = useEditorStore();
const { tree, treeLoading, hasProject } = storeToRefs(project);

const deleteTarget = ref<FileNode | null>(null);
const deleteOpen = computed({
  get: () => deleteTarget.value !== null,
  set: (v: boolean) => {
    if (!v) deleteTarget.value = null;
  },
});
const treeRef = ref<InstanceType<typeof FileTree> | null>(null);
const error = ref<string | null>(null);

const expanded = ref(new Set<string>());
let expandedSaveTimer: ReturnType<typeof setTimeout> | null = null;

// Seed the expansion state whenever the project changes.
watch(
  () => project.root,
  async (root) => {
    if (!root) {
      expanded.value = new Set();
      return;
    }
    expanded.value = new Set(await project.loadExpandedDirs(root));
  },
  { immediate: true },
);

// Persist expansion changes (debounced).
watch(
  expanded,
  () => {
    const root = project.root;
    if (!root) return;
    if (expandedSaveTimer !== null) clearTimeout(expandedSaveTimer);
    expandedSaveTimer = setTimeout(() => {
      expandedSaveTimer = null;
      void project.saveExpandedDirs(root, [...expanded.value]);
    }, 300);
  },
  { deep: true },
);

async function onSelect(node: FileNode) {
  error.value = null;
  try {
    const existing = editor.buffers.get(node.path);
    if (existing) {
      editor.openPath = node.path;
      return;
    }

    const res = await BackendCommands.readFile(node.path);
    if (res) editor.openFile(node.path, res.content, res.mtime_ms);
  } catch (e) {
    error.value = JSON.stringify(e);
  }
}

function onOpen(node: FileNode) {
  void onSelect(node);
}

async function onCreate(
  parentPath: string,
  kind: "file" | "dir",
  name: string,
) {
  error.value = null;
  const rel = parentPath ? `${parentPath}/${name}` : name;
  try {
    await invoke("create_entry", { relPath: rel, kind });
    if (parentPath) treeRef.value?.expandParentsOf(rel);
    await project.refreshTree();
  } catch (e) {
    error.value = JSON.stringify(e);
  }
}

async function onRename(path: string, newName: string) {
  error.value = null;
  const idx = path.lastIndexOf("/");
  const parent = idx === -1 ? "" : path.slice(0, idx);
  const newPath = parent ? `${parent}/${newName}` : newName;
  try {
    await invoke("rename_entry", { oldPath: path, newPath });
    // keep editor state consistent with the rename
    const buf = editor.buffers.get(path);
    if (buf) {
      editor.buffers.delete(path);
      editor.buffers.set(newPath, { ...buf, path: newPath });
      if (editor.openPath === path) editor.openPath = newPath;
    }
    if (project.mainFile === path) {
      await project.setMainFile(newPath);
    }
    await project.refreshTree();
  } catch (e) {
    error.value = JSON.stringify(e);
  }
}

function onDelete(node: FileNode) {
  deleteTarget.value = node;
}

async function confirmDelete() {
  const node = deleteTarget.value;
  if (!node) return;
  deleteTarget.value = null;
  error.value = null;
  try {
    await invoke("delete_entry", { relPath: node.path });
    editor.closeFile(node.path);
    if (project.mainFile === node.path) {
      project.mainFile = null;
    }
    await project.refreshTree();
  } catch (e) {
    error.value = JSON.stringify(e);
  }
}
</script>

<template>
  <div class="flex h-full flex-col overflow-hidden">
    <div
      class="flex my-1 h-10 shrink-0 items-center justify-between border-b px-3 text-xs font-medium text-muted-foreground"
    >
      <span class="w-full">Files</span>
      <Button variant="ghost" size="sm">
        <FolderOpenDot />
      </Button>
      <Button
        size="sm"
        variant="ghost"
        title="Refresh"
        :disabled="!hasProject"
        @click="project.refreshTree()"
      >
        <RefreshCcw />
      </Button>
    </div>
    <p
      v-if="error"
      class="shrink-0 bg-destructive/10 px-3 py-1 text-xs text-destructive"
    >
      {{ error }}
    </p>
    <div class="min-h-0 flex-1 overflow-auto">
      <div
        v-if="!hasProject"
        class="flex flex-col items-center justify-center gap-2 p-4 text-center"
      >
        <FolderOpen class="size-8 text-muted-foreground/40" />
        <p class="text-xs text-muted-foreground">No project open</p>
      </div>
      <FileTree
        v-else
        ref="treeRef"
        :nodes="tree?.children ?? []"
        :expanded-set="expanded"
        :selected-path="editor.openPath"
        :loading="treeLoading"
        @select="onSelect"
        @open="onOpen"
        @create="onCreate"
        @rename="onRename"
        @delete="onDelete"
        @refresh="project.refreshTree()"
      />
    </div>

    <AlertDialog v-model:open="deleteOpen">
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Delete {{ deleteTarget?.kind }}?</AlertDialogTitle>
          <AlertDialogDescription>
            Permanently delete
            <code class="font-mono text-foreground">{{
              deleteTarget?.path
            }}</code
            ><template v-if="deleteTarget?.kind === 'dir'">
              and all of its contents</template
            >? This cannot be undone.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction
            class="bg-destructive text-white hover:bg-destructive/90"
            @click="confirmDelete"
          >
            Delete
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  </div>
</template>
