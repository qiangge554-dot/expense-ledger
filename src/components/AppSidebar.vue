<script setup lang="ts">
defineProps<{ active: string }>();
const emit = defineEmits<{
  (e: "navigate", key: "entry" | "list" | "stats" | "settings"): void;
}>();

const items = [
  { key: "entry" as const, label: "记一笔", icon: "＋" },
  { key: "list" as const, label: "台账", icon: "▤" },
  { key: "stats" as const, label: "看板", icon: "◔" },
  { key: "settings" as const, label: "设置", icon: "⚙" },
];
</script>

<template>
  <aside
    class="w-56 bg-gradient-to-b from-sidebar-from to-sidebar-to text-sidebar-text flex flex-col shrink-0"
  >
    <div
      class="h-14 flex items-center px-5 text-white font-medium text-lg tracking-wide border-b border-white/10"
    >
      报销台账
    </div>
    <nav class="flex-1 py-4 space-y-1">
      <button
        v-for="it in items"
        :key="it.key"
        @click="emit('navigate', it.key)"
        :class="[
          'relative w-full flex items-center gap-3 px-5 py-2.5 text-left text-sm transition',
          active === it.key ? 'bg-sidebar-active text-white' : 'hover:bg-white/10 text-sidebar-text',
        ]"
      >
        <span
          v-if="active === it.key"
          class="absolute left-0 top-2 bottom-2 w-[3px] rounded-r-full bg-accent"
        ></span>
        <span class="text-base leading-none">{{ it.icon }}</span>
        {{ it.label }}
      </button>
    </nav>
    <div class="p-4 text-xs text-sidebar-text/70 border-t border-white/10 leading-relaxed">
      v0.4.0
    </div>
  </aside>
</template>
