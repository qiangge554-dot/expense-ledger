<script setup lang="ts">
import { computed } from "vue";
import { useLedgerStore } from "@/stores/ledger";
import { fmtMoney } from "@/lib/format";

const store = useLedgerStore();
const current = computed(
  () => store.ledgers.find((l) => l.id === store.currentLedgerId) ?? null,
);
const budgetHint = computed(() => {
  if (!current.value || current.value.budget == null) return "";
  return "预算 " + fmtMoney(current.value.budget);
});
</script>

<template>
  <div class="flex items-center gap-3">
    <span class="text-deep font-medium">报销台账</span>
    <div class="relative">
      <select
        :value="store.currentLedgerId"
        @change="store.setLedger(Number(($event.target as HTMLSelectElement).value))"
        class="appearance-none bg-card-alt border border-line rounded-lg pl-3 pr-8 py-1.5 text-sm text-ink outline-none focus:border-deep-500"
      >
        <option v-for="l in store.ledgers" :key="l.id" :value="l.id">
          {{ l.name }}
        </option>
      </select>
      <span class="pointer-events-none absolute right-2.5 top-1/2 -translate-y-1/2 text-ink-300 text-xs">▾</span>
    </div>
    <span v-if="budgetHint" class="text-xs text-ink-400">{{ budgetHint }}</span>
  </div>
</template>
