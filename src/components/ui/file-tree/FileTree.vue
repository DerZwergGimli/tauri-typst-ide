<script setup lang="ts">
import { ref, toRef } from "vue";
import type { FileNode } from "@/types/models";
import FileTreeItem, { type PendingCreate } from "./FileTreeItem.vue";
import { useFileTree } from "./useFileTree";
import { FolderOpen } from "@/lib/fileIcons";

const props = defineProps<{
  nodes: FileNode[];
  expandedSet: Set<string>;
  selectedPath?: string | null;
  loading?: boolean;
  disabled?: boolean;
}>();

const emit = defineEmits<{
  select: [node: FileNode];
  open: [node: FileNode];
  create: [parentPath: string, kind: "file" | "dir", name: string];
  rename: [path: string, newName: string];
  delete: [node: FileNode];
  refresh: [];
}>();

const expanded = toRef(props, "expandedSet");
const focusedPath = ref<string | null>(null);
const renamingPath = ref<string | null>(null);
const pendingCreate = ref<PendingCreate | null>(null);
const rootEl = ref<HTMLElement | null>(null);

const { visible, toggle, moveFocus, focusFirst, focusLast, moveRight, moveLeft } =
  useFileTree(toRef(props, "nodes"), expanded, focusedPath);

function expandParentsOf(path: string) {
  const parts = path.split("/");
  parts.pop();
  let acc = "";
  for (const part of parts) {
    acc = acc ? `${acc}/${part}` : part;
    expanded.value.add(acc);
  }
}

function onContextAction(
  action: "new-file" | "new-folder" | "rename" | "delete",
  node: FileNode,
) {
  if (props.disabled) return;
  if (action === "rename") {
    renamingPath.value = node.path;
    return;
  }
  if (action === "delete") {
    emit("delete", node);
    return;
  }
  // new file/folder: target the node itself if it's an expanded dir, else its parent
  const parentPath =
    node.kind === "dir" && expanded.value.has(node.path)
      ? node.path
      : node.path.split("/").slice(0, -1).join("/");
  pendingCreate.value = {
    parentPath,
    kind: action === "new-file" ? "file" : "dir",
  };
  expanded.value.add(parentPath);
}

function onCreateSubmit(parentPath: string, kind: "file" | "dir", name: string) {
  pendingCreate.value = null;
  emit("create", parentPath, kind, name);
}

function onRenameSubmit(path: string, newName: string) {
  renamingPath.value = null;
  emit("rename", path, newName);
}

function scrollFocusedIntoView() {
  if (!focusedPath.value || !rootEl.value) return;
  const el = rootEl.value.querySelector<HTMLElement>(
    `[data-path="${CSS.escape(focusedPath.value)}"]`,
  );
  el?.scrollIntoView({ block: "nearest" });
}

function focusItem() {
  if (!focusedPath.value || !rootEl.value) return;
  const el = rootEl.value.querySelector<HTMLElement>(
    `[data-path="${CSS.escape(focusedPath.value)}"]`,
  );
  el?.focus();
}

async function moveAndFocus(delta: number) {
  moveFocus(delta);
  scrollFocusedIntoView();
  focusItem();
}

async function focusEdge(which: "first" | "last") {
  if (which === "first") focusFirst();
  else focusLast();
  scrollFocusedIntoView();
  focusItem();
}

function onKeyDown(e: KeyboardEvent) {
  if (props.disabled) return;
  if (renamingPath.value || pendingCreate.value) return;
  const focused = focusedPath.value;
  const node = focused ? visible.value.find((f) => f.node.path === focused)?.node : null;

  switch (e.key) {
    case "ArrowDown":
      e.preventDefault();
      void moveAndFocus(1);
      break;
    case "ArrowUp":
      e.preventDefault();
      void moveAndFocus(-1);
      break;
    case "ArrowRight":
      e.preventDefault();
      if (node) {
        moveRight(node.path, node.kind);
        focusItem();
      }
      break;
    case "ArrowLeft":
      e.preventDefault();
      if (node) {
        moveLeft(node.path);
        focusItem();
      }
      break;
    case "Home":
      e.preventDefault();
      void focusEdge("first");
      break;
    case "End":
      e.preventDefault();
      void focusEdge("last");
      break;
    case "Enter":
      e.preventDefault();
      if (node) {
        if (node.kind === "dir") toggle(node.path);
        else emit("open", node);
      }
      break;
    case "F2":
      e.preventDefault();
      if (node && !props.disabled) renamingPath.value = node.path;
      break;
    case "Delete":
      e.preventDefault();
      if (node && !props.disabled) emit("delete", node);
      break;
  }
}

function onSelect(node: FileNode) {
  emit("select", node);
}

function onOpen(node: FileNode) {
  emit("open", node);
}

defineExpose({ expandParentsOf });
</script>

<template>
  <div
    ref="rootEl"
    role="tree"
    aria-label="Project files"
    class="select-none py-1"
    @keydown="onKeyDown"
  >
    <template v-if="loading">
      <div
        v-for="i in 6"
        :key="i"
        class="mx-2 my-1 h-4 animate-pulse rounded bg-muted"
        :style="{ width: `${40 + (i % 3) * 15}%` }"
      />
    </template>
    <template v-else-if="nodes.length === 0">
      <div class="flex flex-col items-center gap-2 px-4 py-8 text-center">
        <FolderOpen class="size-8 text-muted-foreground/40" />
        <p class="text-xs text-muted-foreground">Empty project</p>
        <button
          class="text-xs text-primary underline-offset-2 hover:underline"
          :disabled="disabled"
          @click="pendingCreate = { parentPath: '', kind: 'file' }; expanded.add('')"
        >
          Create your first file
        </button>
      </div>
    </template>
    <template v-else>
      <FileTreeItem
        v-for="entry in visible.filter((f) => f.depth === 0)"
        :key="entry.node.path"
        :node="entry.node"
        :depth="0"
        :selected-path="selectedPath ?? null"
        :focused-path="focusedPath"
        :expanded-set="expanded"
        :renaming-path="renamingPath"
        :pending-create="pendingCreate"
        :disabled="disabled"
        @focus="focusedPath = $event"
        @select="onSelect"
        @open="onOpen"
        @toggle="toggle"
        @create-submit="onCreateSubmit"
        @create-cancel="pendingCreate = null"
        @rename-submit="onRenameSubmit"
        @rename-cancel="renamingPath = null"
        @context-action="onContextAction"
      />
    </template>
  </div>
</template>
