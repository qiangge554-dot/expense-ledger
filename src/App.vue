<script setup lang="ts">
import { onMounted, ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import AppSidebar from "@/components/AppSidebar.vue";
import LedgerSelect from "@/components/LedgerSelect.vue";
import EntryForm from "@/views/EntryForm.vue";
import LedgerList from "@/views/LedgerList.vue";
import StatsDashboard from "@/views/StatsDashboard.vue";
import SettingsView from "@/views/SettingsView.vue";
import { useLedgerStore } from "@/stores/ledger";
import UpdateNotifier from "@/components/UpdateNotifier.vue";

type TabKey = "entry" | "list" | "stats" | "settings";

const store = useLedgerStore();
const active = ref<TabKey>("entry");
const toast = ref("");
const appVersion = ref("0.4.1");

// 版本号从 Tauri 运行时读，和 tauri.conf.json 永远一致
getVersion()
  .then((v) => (appVersion.value = v))
  .catch(() => {});

onMounted(async () => {
  try {
    await store.loadLedgers();
    await store.refreshAll();
  } catch (e) {
    toast.value = "初始化失败：" + String(e);
    setTimeout(() => (toast.value = ""), 4000);
  }
});

function onEntrySaved() {
  if (active.value === "entry") active.value = "list";
}
</script>

<template>
  <div class="flex h-full">
    <AppSidebar :active="active" @navigate="(k: TabKey) => (active = k)" />
    <div class="flex-1 flex flex-col min-w-0">
      <header
        class="h-14 shrink-0 flex items-center justify-between px-6 bg-card border-b border-line"
      >
        <LedgerSelect />
        <span class="text-xs text-ink-300">v{{ appVersion }}</span>
      </header>

      <main class="flex-1 overflow-auto bg-fog p-6">
        <EntryForm v-if="active === 'entry'" @saved="onEntrySaved" />
        <LedgerList v-else-if="active === 'list'" />
        <StatsDashboard v-else-if="active === 'stats'" />
        <SettingsView v-else />
      </main>
    </div>

    <UpdateNotifier />

    <transition name="fade">
      <div
        v-if="toast"
        class="fixed bottom-6 left-1/2 -translate-x-1/2 bg-deep text-white text-sm px-4 py-2 rounded-lg shadow-lg"
      >
        {{ toast }}
      </div>
    </transition>
  </div>
</template>

<style scoped>
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.25s;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
