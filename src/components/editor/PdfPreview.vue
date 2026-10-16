<script setup lang="ts">
import {
  computed,
  nextTick,
  onBeforeUnmount,
  onMounted,
  ref,
  watch,
} from "vue";
import * as pdfjs from "pdfjs-dist/legacy/build/pdf.mjs";
import PdfWorker from "pdfjs-dist/legacy/build/pdf.worker.min.mjs?url";
import type {
  PDFDocumentLoadingTask,
  PDFDocumentProxy,
  RenderTask,
} from "pdfjs-dist";
import { Minus, Plus, Scan } from "@lucide/vue";
import { Button } from "@/components/ui/button";

// Legacy build + local worker: the modern build needs `Promise.withResolvers`,
// which is unreliable on the WebKitGTK webview Tauri uses on Linux.
pdfjs.GlobalWorkerOptions.workerSrc = PdfWorker;

const props = defineProps<{ bytes: ArrayBuffer }>();

const MIN_SCALE = 0.5;
const MAX_SCALE = 3;
const ZOOM_STEP = 1.2;
const PAGE_GAP = 16;
const H_PADDING = 32;

const container = ref<HTMLDivElement | null>(null);
const pageSizes = ref<{ width: number; height: number }[]>([]);
const loading = ref(true);
const error = ref<string | null>(null);
const fitWidth = ref(true);
const fitScale = ref(1);
const customScale = ref(1);

const scale = computed(() => (fitWidth.value ? fitScale.value : customScale.value));
const zoomPercent = computed(() => Math.round(scale.value * 100));

let doc: PDFDocumentProxy | null = null;
let loadingTask: PDFDocumentLoadingTask | null = null;
let observer: IntersectionObserver | null = null;
let resizeObserver: ResizeObserver | null = null;
let fitTimer: ReturnType<typeof setTimeout> | null = null;
const renderedTasks = new Map<number, RenderTask>();
const renderTokens = new Map<number, number>();
const renderedPages = new Set<number>();
let tokenSeq = 0;
let loadSeq = 0;

function cancelAllRenders() {
  for (const task of renderedTasks.values()) task.cancel();
}

function cleanupDoc() {
  cancelAllRenders();
  renderedTasks.clear();
  renderTokens.clear();
  renderedPages.clear();
  observer?.disconnect();
  observer = null;
  void loadingTask?.destroy();
  loadingTask = null;
  doc = null;
}

async function renderPage(index: number) {
  const document = doc;
  if (!document) return;
  const canvas = container.value?.querySelector(
    `[data-index="${index}"] canvas`,
  ) as HTMLCanvasElement | null;
  if (!canvas) return;

  renderedPages.add(index);
  renderedTasks.get(index)?.cancel();
  renderedTasks.delete(index);
  const token = ++tokenSeq;
  renderTokens.set(index, token);

  try {
    const page = await document.getPage(index + 1);
    if (document !== doc) return; // a different document was loaded meanwhile
    const viewport = page.getViewport({ scale: scale.value });
    const dpr = window.devicePixelRatio || 1;
    canvas.width = Math.floor(viewport.width * dpr);
    canvas.height = Math.floor(viewport.height * dpr);
    canvas.style.width = `${viewport.width}px`;
    canvas.style.height = `${viewport.height}px`;
    const task = page.render({
      canvasContext: canvas.getContext("2d")!,
      canvas,
      viewport,
      transform: [dpr, 0, 0, dpr, 0, 0],
    });
    renderedTasks.set(index, task);
    await task.promise;
  } catch {
    // RenderingCancelledException / render errors — canvas keeps its last frame.
  } finally {
    if (renderTokens.get(index) === token) {
      renderTokens.delete(index);
      renderedTasks.delete(index);
    }
  }
}

function rerenderVisible() {
  cancelAllRenders();
  for (const index of [...renderedPages]) void renderPage(index);
}

function measureFit() {
  const el = container.value;
  if (!el || pageSizes.value.length === 0) return;
  const maxWidth = Math.max(...pageSizes.value.map((p) => p.width));
  const available = el.clientWidth - H_PADDING;
  if (maxWidth > 0 && available > 0) {
    fitScale.value = Math.min(
      Math.max(available / maxWidth, MIN_SCALE),
      MAX_SCALE,
    );
  }
}

function setupObserver() {
  const el = container.value;
  if (!el) return;
  observer?.disconnect();
  observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (!entry.isIntersecting) continue;
        const index = Number((entry.target as HTMLElement).dataset.index);
        if (!Number.isNaN(index)) void renderPage(index);
      }
    },
    { root: el, rootMargin: "600px 0px" },
  );
  for (const page of el.querySelectorAll<HTMLElement>("[data-index]")) {
    observer.observe(page);
  }
}

async function load(bytes: ArrayBuffer) {
  const seq = ++loadSeq;
  cleanupDoc();
  pageSizes.value = [];
  error.value = null;
  loading.value = true;
  try {
    // `data` transfers the buffer to the worker (it is detached afterwards);
    // every open fetches a fresh copy, so this is safe.
    loadingTask = pdfjs.getDocument({ data: bytes });
    doc = await loadingTask.promise;
    if (seq !== loadSeq) return;
    const sizes: { width: number; height: number }[] = [];
    for (let i = 1; i <= doc.numPages; i++) {
      const page = await doc.getPage(i);
      if (seq !== loadSeq) return;
      const viewport = page.getViewport({ scale: 1 });
      sizes.push({ width: viewport.width, height: viewport.height });
    }
    pageSizes.value = sizes;
    loading.value = false;
    await nextTick();
    if (seq !== loadSeq) return;
    setupObserver();
    measureFit();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
    loading.value = false;
  }
}

function setCustomScale(next: number) {
  customScale.value = Math.min(Math.max(next, MIN_SCALE), MAX_SCALE);
  fitWidth.value = false;
}

function zoomIn() {
  setCustomScale((fitWidth.value ? fitScale.value : customScale.value) * ZOOM_STEP);
}

function zoomOut() {
  setCustomScale((fitWidth.value ? fitScale.value : customScale.value) / ZOOM_STEP);
}

function toggleFit() {
  fitWidth.value = !fitWidth.value;
  if (fitWidth.value) measureFit();
}

watch(scale, () => rerenderVisible());

watch(
  () => props.bytes,
  (bytes) => void load(bytes),
  { immediate: true },
);

onMounted(() => {
  const el = container.value;
  if (!el) return;
  resizeObserver = new ResizeObserver(() => {
    if (!fitWidth.value) return;
    if (fitTimer !== null) clearTimeout(fitTimer);
    fitTimer = setTimeout(() => {
      fitTimer = null;
      measureFit();
    }, 120);
  });
  resizeObserver.observe(el);
});

onBeforeUnmount(() => {
  if (fitTimer !== null) clearTimeout(fitTimer);
  resizeObserver?.disconnect();
  cleanupDoc();
});
</script>

<template>
  <div class="flex h-full min-h-0 flex-col">
    <div
      class="flex h-8 shrink-0 items-center gap-1 border-b px-2 text-xs text-muted-foreground"
    >
      <Button
        variant="ghost"
        size="sm"
        class="h-6 w-6 p-0"
        title="Zoom out"
        :disabled="loading"
        @click="zoomOut"
      >
        <Minus class="size-3.5" />
      </Button>
      <span class="w-11 text-center tabular-nums">{{ zoomPercent }}%</span>
      <Button
        variant="ghost"
        size="sm"
        class="h-6 w-6 p-0"
        title="Zoom in"
        :disabled="loading"
        @click="zoomIn"
      >
        <Plus class="size-3.5" />
      </Button>
      <Button
        variant="ghost"
        size="sm"
        class="h-6 gap-1 px-2 text-xs"
        :class="fitWidth ? 'bg-accent text-accent-foreground' : ''"
        title="Fit width"
        :disabled="loading"
        @click="toggleFit"
      >
        <Scan class="size-3.5" />
        Fit width
      </Button>
      <span v-if="pageSizes.length" class="ml-auto pr-1">
        {{ pageSizes.length }}
        {{ pageSizes.length === 1 ? "page" : "pages" }}
      </span>
    </div>

    <div
      ref="container"
      class="min-h-0 flex-1 overflow-auto bg-muted/40"
      :style="{ padding: `${PAGE_GAP / 2}px ${H_PADDING / 2}px` }"
    >
      <p
        v-if="error"
        class="p-4 text-center text-xs text-destructive"
        :title="error"
      >
        Failed to render PDF: {{ error }}
      </p>
      <p
        v-else-if="loading"
        class="p-4 text-center text-xs text-muted-foreground"
      >
        Loading PDF…
      </p>
      <div v-else class="mx-auto flex w-max flex-col items-center">
        <div
          v-for="(size, index) in pageSizes"
          :key="index"
          class="pdf-page bg-white shadow-sm"
          :data-index="index"
          :style="{
            width: `${size.width * scale}px`,
            height: `${size.height * scale}px`,
            marginBottom: `${PAGE_GAP}px`,
          }"
        >
          <canvas class="block h-full w-full" />
        </div>
      </div>
    </div>
  </div>
</template>
