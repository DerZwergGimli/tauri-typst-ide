<script setup lang="ts">
import { FileText } from "@lucide/vue";
</script>

<template>
  <div class="flex h-full flex-col items-center justify-center gap-3 p-6 text-center">
    <FileText class="size-10 text-muted-foreground/40" />
    <p class="text-sm text-muted-foreground">
      Select a file to start editing
    </p>
  </div>
</template>
