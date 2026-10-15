<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import type { FileNode } from "@/types/models";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import { iconForFile, Folder, FolderOpen } from "@/lib/fileIcons";
import { ChevronRight } from "@lucide/vue";

defineOptions({ inheritAttrs: false });

export interface PendingCreate {
  parentPath: string;
  kind: "file" | "dir";
}

const props = defineProps<{
  node: FileNode;
  depth: number;
  selectedPath: string | null;
  focusedPath: string | null;
  expandedSet: ReadonlySet<string>;
  renamingPath: string | null;
  pendingCreate: PendingCreate | null;
  disabled?: boolean;
}>();

const emit = defineEmits<{
  focus: [path: string];
  select: [node: FileNode];
  open: [node: FileNode];
  toggle: [path: string];
  "create-submit": [parentPath: string, kind: "file" | "dir", name: string];
  "create-cancel": [];
  "rename-submit": [path: string, newName: string];
  "rename-cancel": [];
  contextAction: [
    action: "new-file" | "new-folder" | "rename" | "delete",
    node: FileNode,
  ];
}>();

const renameInput = ref<HTMLInputElement | null>(null);
const createInput = ref<HTMLInputElement | null>(null);
const renameValue = ref("");
const createValue = ref("");
const renameDone = ref(false);
const createDone = ref(false);

const isDir = computed(() => props.node.kind === "dir");
const isExpanded = computed(
  () => isDir.value && props.expandedSet.has(props.node.path),
);
const isFocused = computed(() => props.focusedPath === props.node.path);
const isSelected = computed(
  () => !isDir.value && props.selectedPath === props.node.path,
);
const isRenaming = computed(() => props.renamingPath === props.node.path);
const showCreateInput = computed(
  () =>
    isDir.value &&
    isExpanded.value &&
    props.pendingCreate?.parentPath === props.node.path,
);
const FileIcon = computed(() => iconForFile(props.node.name));

watch(
  () => props.renamingPath,
  async (p) => {
    if (p === props.node.path) {
      renameValue.value = props.node.name;
      renameDone.value = false;
      await nextTick();
      renameInput.value?.focus();
      const dot = props.node.name.lastIndexOf(".");
      renameInput.value?.setSelectionRange(0, dot > 0 ? dot : props.node.name.length);
    }
  },
);

watch(
  () => props.pendingCreate?.parentPath,
  async (p) => {
    if (p === props.node.path && isExpanded.value) {
      createValue.value = "";
      createDone.value = false;
      await nextTick();
      createInput.value?.focus();
    }
  },
);

function onClick() {
  emit("focus", props.node.path);
  if (isDir.value) emit("toggle", props.node.path);
  else emit("select", props.node);
}

function onDoubleClick() {
  if (!isDir.value) emit("open", props.node);
}

function submitRename() {
  if (renameDone.value) return;
  renameDone.value = true;
  const name = renameValue.value.trim();
  if (name && name !== props.node.name) {
    emit("rename-submit", props.node.path, name);
  } else {
    emit("rename-cancel");
  }
}

function submitCreate() {
  if (createDone.value) return;
  createDone.value = true;
  const name = createValue.value.trim();
  if (name && props.pendingCreate) {
    emit("create-submit", props.pendingCreate.parentPath, props.pendingCreate.kind, name);
  } else {
    emit("create-cancel");
  }
}

function onContextAction(action: "new-file" | "new-folder" | "rename" | "delete") {
  emit("focus", props.node.path);
  emit("contextAction", action, props.node);
}
</script>

<template>
  <ContextMenu>
    <ContextMenuTrigger as-child disabled>
      <div
        role="treeitem"
        :aria-level="depth + 1"
        :aria-selected="isSelected"
        :aria-expanded="isDir ? isExpanded : undefined"
        :tabindex="isFocused ? 0 : -1"
        :data-path="node.path"
        :style="{ paddingLeft: `${depth * 12 + 8}px` }"
        :class="
          cn(
            'group flex h-6 w-full select-none items-center gap-1.5 pr-2 text-sm',
            'outline-none',
            isSelected && 'bg-accent text-accent-foreground',
            isFocused && !isSelected && 'bg-accent/50',
            !isSelected && 'hover:bg-accent/40',
          )
        "
        @click="onClick"
        @dblclick="onDoubleClick"
        @focus="emit('focus', node.path)"
      >
        <template v-if="isRenaming">
          <Input
            ref="renameInput"
            v-model="renameValue"
            class="h-5 flex-1 px-1 text-xs"
            @keydown.enter.prevent="submitRename"
            @keydown.esc.prevent="renameValue = node.name; emit('rename-cancel')"
            @blur="submitRename"
            @click.stop
            @dblclick.stop
          />
        </template>
        <template v-else>
          <ChevronRight
            v-if="isDir"
            :class="cn('size-3.5 shrink-0 text-muted-foreground transition-transform', isExpanded && 'rotate-90')"
          />
          <span v-else class="w-3.5 shrink-0" />
          <component
            :is="isDir ? (isExpanded ? FolderOpen : Folder) : FileIcon"
            class="size-4 shrink-0 text-muted-foreground"
            :class="node.name.endsWith('.typ') && 'text-blue-500/80'"
          />
          <span class="truncate">{{ node.name }}</span>
        </template>
      </div>
    </ContextMenuTrigger>
    <ContextMenuContent class="w-44">
      <ContextMenuItem
        :disabled="isDir && !isExpanded"
        @select="onContextAction('new-file')"
      >
        New file
      </ContextMenuItem>
      <ContextMenuItem
        :disabled="isDir && !isExpanded"
        @select="onContextAction('new-folder')"
      >
        New folder
      </ContextMenuItem>
      <ContextMenuSeparator />
      <ContextMenuItem @select="onContextAction('rename')">
        Rename
      </ContextMenuItem>
      <ContextMenuItem
        variant="destructive"
        @select="onContextAction('delete')"
      >
        Delete
      </ContextMenuItem>
    </ContextMenuContent>
  </ContextMenu>

  <!-- inline create row -->
  <div
    v-if="showCreateInput"
    role="treeitem"
    :aria-level="depth + 2"
    :style="{ paddingLeft: `${(depth + 1) * 12 + 8}px` }"
    class="flex h-6 w-full items-center pr-2"
    @click.stop
  >
    <span class="w-3.5 shrink-0" />
    <Input
      ref="createInput"
      v-model="createValue"
      class="h-5 flex-1 px-1 text-xs"
      :placeholder="pendingCreate?.kind === 'dir' ? 'folder name' : 'file name'"
      @keydown.enter.prevent="submitCreate"
      @keydown.esc.prevent="createValue = ''; emit('create-cancel')"
      @blur="submitCreate"
      @click.stop
    />
  </div>

  <!-- children -->
  <template v-if="isDir && isExpanded && node.children">
    <FileTreeItem
      v-for="child in node.children"
      :key="child.path"
      :node="child"
      :depth="depth + 1"
      :selected-path="selectedPath"
      :focused-path="focusedPath"
      :expanded-set="expandedSet"
      :renaming-path="renamingPath"
      :pending-create="pendingCreate"
      :disabled="disabled"
      @focus="emit('focus', $event)"
      @select="emit('select', $event)"
      @open="emit('open', $event)"
      @toggle="emit('toggle', $event)"
      @create-submit="(p, k, n) => emit('create-submit', p, k, n)"
      @create-cancel="emit('create-cancel')"
      @rename-submit="(p, n) => emit('rename-submit', p, n)"
      @rename-cancel="emit('rename-cancel')"
      @context-action="(a, n) => emit('contextAction', a, n)"
    />
  </template>
</template>
