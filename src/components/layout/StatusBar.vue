<script setup lang="ts">
import { storeToRefs } from "pinia";
import { useLogsStore } from "@/stores/logs";
import { ChevronDown, ChevronUp } from "@lucide/vue";
import { cn } from "@/lib/utils";

const logs = useLogsStore();
const { panelExpanded, latestEntry, isCompiling } = storeToRefs(logs);

const levelClasses: Record<string, string> = {
  info: "text-muted-foreground",
  warn: "text-amber-500",
  error: "text-destructive",
};
</script>

<template>
  <div class="flex shrink-0 flex-col border-t bg-background">
    <button
      class="flex h-7 w-full items-center gap-2 px-3 text-left text-xs hover:bg-accent/50"
      @click="panelExpanded = !panelExpanded"
    >
      <span
        class="size-2 shrink-0 rounded-full"
        :class="
          isCompiling
            ? 'animate-pulse bg-amber-500'
            : latestEntry?.level === 'error'
              ? 'bg-destructive'
              : 'bg-emerald-500'
        "
      />
      <span
        v-if="latestEntry"
        :class="cn('truncate font-mono', levelClasses[latestEntry.level])"
      >
        [{{ latestEntry.context }}]
      </span>
      <span
        v-if="latestEntry"
        :class="cn('truncate font-mono', levelClasses[latestEntry.level])"
      >
        {{ latestEntry.message }}
      </span>
      <span v-else class="truncate font-mono text-muted-foreground">
        Ready
      </span>
      <component
        :is="panelExpanded ? ChevronDown : ChevronUp"
        class="ml-auto size-3.5 shrink-0 text-muted-foreground"
      />
    </button>
  </div>
</template>
