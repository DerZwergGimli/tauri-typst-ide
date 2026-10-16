<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import { storeToRefs } from "pinia";
import { useLogsStore } from "@/stores/logs";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import type { Diagnostic } from "@/types/models";
import { AlertTriangle, CircleX } from "@lucide/vue";

const logs = useLogsStore();
const { entries, diagnostics, panelExpanded } = storeToRefs(logs);
const scrollEl = ref<HTMLElement | null>(null);

const emit = defineEmits<{ openDiagnostic: [d: Diagnostic] }>();

const levelClasses: Record<string, string> = {
  info: "text-muted-foreground",
  warn: "text-amber-600 dark:text-amber-400",
  error: "text-destructive",
};

watch(
  () => entries.value.length,
  async () => {
    if (!panelExpanded.value) return;
    await nextTick();
    scrollEl.value?.scrollTo({ top: scrollEl.value.scrollHeight });
  },
);

function formatTime(ts: string): string {
  const d = new Date(ts);
  return d.toLocaleTimeString(undefined, { hour12: false });
}
</script>

<template>
  <div
    v-if="panelExpanded"
    class="flex h-48 shrink-0 flex-col border-t bg-background"
  >
    <div v-if="diagnostics.length > 0" class="shrink-0 border-b px-3 py-1.5">
      <button
        v-for="(d, i) in diagnostics"
        :key="i"
        class="flex w-full items-start gap-2 rounded-sm px-1 py-0.5 text-left text-xs hover:bg-accent/50"
        @click="emit('openDiagnostic', d)"
      >
        <CircleX
          v-if="d.severity === 'error'"
          class="mt-0.5 size-3.5 shrink-0 text-destructive"
        />
        <AlertTriangle v-else class="mt-0.5 size-3.5 shrink-0 text-amber-500" />
        <span class="font-mono text-muted-foreground">
          {{ d.path ?? "?"
          }}<template v-if="d.line !== null"
            >:{{ d.line }}:{{ d.column }}</template
          >
        </span>

        <span class="truncate">{{ d.message }}</span>
      </button>
    </div>
    <ScrollArea class="flex-1">
      <div ref="scrollEl" class="px-3 py-1 font-mono text-xs">
        <div
          v-for="(e, i) in entries"
          :key="i"
          class="flex gap-2 py-0.5 leading-5"
        >
          <span class="shrink-0 text-muted-foreground/60">
            {{ formatTime(e.timestamp) }}
          </span>
          <Badge
            variant="outline"
            class="h-4 shrink-0 px-1 text-[10px] uppercase"
            :class="levelClasses[e.level]"
          >
            {{ e.level }}
          </Badge>
          <span :class="cn('truncate', levelClasses[e.level])">
            [{{ e.context }}]
          </span>
          <span :class="cn('truncate', levelClasses[e.level])">
            {{ e.message }}
          </span>
        </div>
        <div v-if="entries.length === 0" class="py-2 text-muted-foreground">
          No log entries yet
        </div>
      </div>
    </ScrollArea>
  </div>
</template>
