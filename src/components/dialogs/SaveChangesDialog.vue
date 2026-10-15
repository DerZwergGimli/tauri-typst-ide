<script setup lang="ts">
import { computed } from "vue";
import { storeToRefs } from "pinia";
import { useCompileStore } from "@/stores/compile";
import { useEditorStore } from "@/stores/editor";
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

const compile = useCompileStore();
const editor = useEditorStore();
const { promptOpen, promptMessage } = storeToRefs(compile);

const open = computed({
  get: () => promptOpen.value,
  set: (v: boolean) => {
    // Dismissing (Escape / outside click) cancels the action.
    if (!v) compile.choose("cancel");
  },
});
</script>

<template>
  <AlertDialog v-model:open="open">
    <AlertDialogContent>
      <AlertDialogHeader>
        <AlertDialogTitle>{{ promptMessage }}</AlertDialogTitle>
        <AlertDialogDescription>
          {{ editor.dirtyPaths.length }} file{{
            editor.dirtyPaths.length === 1 ? "" : "s"
          }}
          {{ editor.dirtyPaths.length === 1 ? "has" : "have" }} unsaved changes.
          Save them, continue without saving, or cancel.
        </AlertDialogDescription>
      </AlertDialogHeader>
      <AlertDialogFooter>
        <AlertDialogCancel @click="compile.choose('cancel')">
          Cancel
        </AlertDialogCancel>
        <AlertDialogAction
          class="bg-destructive text-white hover:bg-destructive/90"
          @click="compile.choose('discard')"
        >
          Discard all
        </AlertDialogAction>
        <AlertDialogAction @click="compile.choose('save')">
          Save all
        </AlertDialogAction>
      </AlertDialogFooter>
    </AlertDialogContent>
  </AlertDialog>
</template>
