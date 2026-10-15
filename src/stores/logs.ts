import { computed, reactive, ref } from "vue";
import { defineStore } from "pinia";
import type { Diagnostic, LogEntry, ProcessName, ProcessStatusKind } from "@/types/models";

const MAX_LOG_ENTRIES = 2000;

export const useLogsStore = defineStore("logs", () => {
  const entries = ref<LogEntry[]>([]);
  const diagnostics = ref<Diagnostic[]>([]);
  const processes = reactive(new Map<ProcessName, ProcessStatusKind>());
  const panelExpanded = ref(false);

  const isCompiling = computed(() => processes.get("compile") === "started");
  const latestEntry = computed(() => entries.value[entries.value.length - 1] ?? null);

  function push(entry: LogEntry) {
    entries.value.push(entry);
    if (entries.value.length > MAX_LOG_ENTRIES) {
      entries.value.splice(0, entries.value.length - MAX_LOG_ENTRIES);
    }
  }

  function setProcessStatus(process: ProcessName, status: ProcessStatusKind) {
    processes.set(process, status);
  }

  function setDiagnostics(diags: Diagnostic[]) {
    diagnostics.value = diags;
  }

  function clear() {
    entries.value = [];
    diagnostics.value = [];
    processes.clear();
  }

  return {
    entries,
    diagnostics,
    processes,
    panelExpanded,
    isCompiling,
    latestEntry,
    push,
    setProcessStatus,
    setDiagnostics,
    clear,
  };
});
