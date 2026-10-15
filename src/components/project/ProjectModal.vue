<script setup lang="ts">
import { computed, ref } from "vue";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useProjectStore } from "@/stores/project";
import { useCompileStore } from "@/stores/compile";
import { FolderOpen, Plus, Clock } from "@lucide/vue";

const open = defineModel<boolean>("open", { required: true });
const props = defineProps<{ dismissible: boolean }>();

const project = useProjectStore();
const compile = useCompileStore();
const mode = ref<"choose" | "create">("choose");
const parentDir = ref<string | null>(null);
const projectName = ref("");
const createError = ref<string | null>(null);
const busy = ref(false);

const canCreate = computed(
  () => parentDir.value !== null && projectName.value.trim().length > 0 && !busy.value,
);

/** Prompt to save unsaved work before switching projects. */
async function guardDirty(): Promise<boolean> {
  return compile.ensureSaved("Save changes before switching projects?");
}

function onEscape(e: Event) {
  if (!props.dismissible) e.preventDefault();
}
function onInteractOutside(e: Event) {
  if (!props.dismissible) e.preventDefault();
}

async function pickFolder(target: "open" | "create") {
  const { open: openDialog } = await import("@tauri-apps/plugin-dialog");
  const dir = await openDialog({ directory: true, multiple: false });
  if (!dir) return;
  if (target === "open") {
    if (!(await guardDirty())) return;
    busy.value = true;
    try {
      await project.openProject(dir);
    } finally {
      busy.value = false;
    }
  } else {
    parentDir.value = dir;
    mode.value = "create";
  }
}

async function doCreate() {
  if (!canCreate.value || !parentDir.value) return;
  if (!(await guardDirty())) return;
  createError.value = null;
  busy.value = true;
  try {
    await project.createProject(parentDir.value, projectName.value.trim());
  } catch (e) {
    createError.value = String(e);
  } finally {
    busy.value = false;
  }
}

async function openRecent(path: string) {
  if (!(await guardDirty())) return;
  busy.value = true;
  project
    .openProject(path)
    .catch(async () => {
      // stale recent — remove it and let the user pick another
      await project.removeRecent(path);
    })
    .finally(() => {
      busy.value = false;
    });
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent
      class="sm:max-w-lg"
      :show-close-button="dismissible"
      @escape-key-down="onEscape"
      @interact-outside="onInteractOutside"
    >
      <DialogHeader>
        <DialogTitle>Typst IDE</DialogTitle>
        <DialogDescription>
          Create a new Typst project or open an existing folder.
        </DialogDescription>
      </DialogHeader>

      <div v-if="mode === 'choose'" class="grid gap-3">
        <div class="grid grid-cols-2 gap-3">
          <Button
            variant="outline"
            class="h-24 flex-col gap-2"
            :disabled="busy"
            @click="mode = 'create'"
          >
            <Plus class="size-5" />
            New project
          </Button>
          <Button
            variant="outline"
            class="h-24 flex-col gap-2"
            :disabled="busy"
            @click="pickFolder('open')"
          >
            <FolderOpen class="size-5" />
            Open folder
          </Button>
        </div>

        <div v-if="project.recents.length > 0" class="border-t pt-3">
          <p class="mb-2 text-xs font-medium text-muted-foreground">
            Recent projects
          </p>
          <ul class="grid gap-1">
            <li v-for="r in project.recents" :key="r.path">
              <button
                class="flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-left text-sm hover:bg-accent"
                :disabled="busy"
                @click="openRecent(r.path)"
              >
                <Clock class="size-3.5 shrink-0 text-muted-foreground" />
                <span class="truncate">{{ r.name }}</span>
                <span class="ml-auto truncate text-xs text-muted-foreground">
                  {{ r.path }}
                </span>
              </button>
            </li>
          </ul>
        </div>
      </div>

      <div v-else class="grid gap-4">
        <div class="grid gap-2">
          <label class="text-sm font-medium" for="project-parent">
            Parent folder
          </label>
          <div class="flex gap-2">
            <Input
              id="project-parent"
              :model-value="parentDir ?? ''"
              readonly
              placeholder="Pick a parent folder…"
            />
            <Button variant="outline" @click="pickFolder('create')">
              Browse
            </Button>
          </div>
        </div>
        <div class="grid gap-2">
          <label class="text-sm font-medium" for="project-name">
            Project name
          </label>
          <Input
            id="project-name"
            v-model="projectName"
            placeholder="my-typst-doc"
          />
          <p
            v-if="parentDir && projectName.trim()"
            class="text-xs text-muted-foreground"
          >
            {{ parentDir }}/{{ projectName.trim() }}
          </p>
        </div>
        <p v-if="createError" class="text-sm text-destructive">
          {{ createError }}
        </p>
        <div class="flex justify-end gap-2">
          <Button variant="ghost" @click="mode = 'choose'">Back</Button>
          <Button :disabled="!canCreate" @click="doCreate">
            Create project
          </Button>
        </div>
      </div>
    </DialogContent>
  </Dialog>
</template>
