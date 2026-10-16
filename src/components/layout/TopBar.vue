<script setup lang="ts">
import { storeToRefs } from "pinia";
import { useProjectStore } from "@/stores/project";
import { useLogsStore } from "@/stores/logs";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { FolderOpen, Play } from "@lucide/vue";
import ModeToggle from "@/components/layout/ModeToggle.vue";

const project = useProjectStore();
const logs = useLogsStore();
const { mainFile, typFiles, name } = storeToRefs(project);
const { isCompiling } = storeToRefs(logs);

const emit = defineEmits<{ compile: [] }>();

function onSelectMainFile(value: unknown) {
  if (typeof value === "string") {
    project.setMainFile(value);
  }
}

function openProjectModal() {
  project.modalOpen = true;
}
</script>

<template>
  <header
    class="flex h-11 shrink-0 items-center gap-2 border-b bg-background px-3"
  >
    <Button variant="outline" size="sm" @click="openProjectModal">
      <FolderOpen class="mr-1.5 size-4" />
      {{ name ?? "Open project…" }}
    </Button>

    <div class="ml-2 flex items-center gap-2">
      <Select
        :model-value="mainFile ?? undefined"
        :disabled="typFiles.length === 0"
        @update:model-value="onSelectMainFile"
      >
        <SelectTrigger class="w-56" size="sm">
          <SelectValue placeholder="Main file" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem v-for="f in typFiles" :key="f" :value="f">
            {{ f }}
          </SelectItem>
        </SelectContent>
      </Select>

      <Button
        size="sm"
        :disabled="isCompiling || !mainFile"
        @click="emit('compile')"
      >
        <Play v-if="!isCompiling" class="mr-1.5 size-4" />
        <span
          v-else
          class="mr-1.5 size-4 animate-spin rounded-full border-2 border-current border-t-transparent"
        />
        Compile
      </Button>
    </div>
    <div class="flex w-full"></div>
    <ModeToggle />
  </header>
</template>
