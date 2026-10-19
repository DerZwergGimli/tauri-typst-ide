<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useCodeMirror } from "@/composables/useCodeMirror";

const props = withDefaults(
  defineProps<{
    modelValue: string;
    fileName: string;
    readOnly?: boolean;
    jumpTo?: { offset: number; length: number; nonce: number } | null;
  }>(),
  { readOnly: false, jumpTo: null },
);

const emit = defineEmits<{
  "update:modelValue": [value: string];
  save: [];
}>();

const host = ref<HTMLElement | null>(null);
let api: ReturnType<typeof useCodeMirror> | null = null;
let internalChange = false;

onMounted(async () => {
  if (!host.value) return;
  api = useCodeMirror({
    parent: host.value,
    doc: props.modelValue,
    readOnly: props.readOnly,
    onChange: (value) => {
      internalChange = true;
      emit("update:modelValue", value);
      internalChange = false;
    },
    onSave: () => emit("save"),
  });

  //const language = await loadTypstLanguage();
  await api.setLanguageForFile(props.fileName);
});

onBeforeUnmount(() => {
  api?.destroy();
});

watch(
  () => props.modelValue,
  (v) => {
    if (internalChange || !api) return;
    api.setValue(v);
  },
);

watch(
  () => props.readOnly,
  (v) => api?.setReadOnly(v),
);

watch(
  () => props.jumpTo,
  (j) => {
    if (j) api?.jumpTo(j.offset, j.length);
  },
);
</script>

<template>
  <div ref="host" class="h-full min-h-0 overflow-hidden" />
</template>
