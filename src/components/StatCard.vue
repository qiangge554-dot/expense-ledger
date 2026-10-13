<script setup lang="ts">
defineProps<{
  label: string;
  value: string;
  accent?: boolean;
  tone?: "expense" | "income" | "warn";
}>();
</script>

<template>
  <div
    class="bg-card rounded-xl border border-line shadow-card hover:shadow-card-hover transition-shadow p-4"
  >
    <div class="text-ink-400 text-xs">{{ label }}</div>
    <div
      :class="[
        'mt-1.5 text-2xl font-medium tabular-nums',
        tone === 'expense'
          ? 'text-expense'
          : tone === 'income'
            ? 'text-income'
            : tone === 'warn'
              ? 'text-warn-fg'
              : accent
                ? 'text-deep'
                : 'text-ink',
      ]"
    >
      {{ value }}
    </div>
  </div>
</template>
