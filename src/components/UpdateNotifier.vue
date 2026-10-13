<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import {
  dismissUpdate,
  errorText,
  info,
  percent,
  phase,
  runCheck,
  runInstall,
} from "@/lib/updater";

// 只有用户主动点过「检查更新」时，才把「已是最新 / 失败」提示出来；
// 启动时的静默检测失败不应该打扰用户。
const userInitiated = ref(false);

const visible = computed(() => {
  const p = phase.value;
  if (p === "available" || p === "downloading" || p === "installing") return true;
  if (!userInitiated.value) return false;
  return p === "uptodate" || p === "error";
});

const busy = computed(
  () => phase.value === "checking" || phase.value === "downloading" || phase.value === "installing",
);

onMounted(() => {
  // 延后 2 秒，避免拖慢首屏数据加载
  window.setTimeout(() => {
    runCheck();
  }, 2000);
});

async function manualCheck() {
  userInitiated.value = true;
  await runCheck();
}

async function install() {
  userInitiated.value = true;
  await runInstall();
}

async function later() {
  await dismissUpdate();
}
</script>

<template>
  <transition name="fade">
    <div
      v-if="visible"
      class="fixed bottom-6 right-6 z-50 w-80 rounded-2xl border border-line bg-card p-4 shadow-pop"
    >
      <div v-if="phase === 'available'" class="space-y-2">
        <p class="text-sm font-medium text-ink">发现新版本 {{ info?.version }}</p>
        <p class="text-xs text-ink-400">当前版本 {{ info?.currentVersion }}</p>
        <p
          v-if="info?.notes"
          class="max-h-24 overflow-auto whitespace-pre-line rounded-lg bg-card-alt px-2 py-1.5 text-xs leading-relaxed text-ink-400"
        >
          {{ info.notes }}
        </p>
        <div class="flex items-center gap-2 pt-1">
          <button
            class="rounded-lg bg-deep px-3 py-1.5 text-sm text-white hover:bg-deep-600 disabled:opacity-60"
            :disabled="busy"
            @click="install"
          >
            立即更新
          </button>
          <button class="text-sm text-ink-400 hover:underline" @click="later">稍后</button>
        </div>
      </div>

      <div v-else-if="phase === 'downloading'" class="space-y-2">
        <p class="text-sm font-medium text-ink">正在下载 {{ info?.version }}</p>
        <div class="h-1.5 w-full overflow-hidden rounded-full bg-fog-300">
          <div
            class="h-full rounded-full bg-deep-500 transition-all"
            :style="{ width: (percent ?? 0) + '%' }"
          />
        </div>
        <p class="text-xs text-ink-400">{{ percent === null ? "下载中…" : percent + "%" }}</p>
      </div>

      <div v-else-if="phase === 'installing'" class="space-y-1">
        <p class="text-sm font-medium text-ink">正在安装…</p>
        <p class="text-xs text-ink-400">安装完成后应用会自动重启</p>
      </div>

      <div v-else-if="phase === 'uptodate'" class="space-y-1">
        <p class="text-sm font-medium text-ink">已是最新版本</p>
        <p class="text-xs text-ink-400">当前 v{{ info?.currentVersion || "0.4.1" }}</p>
      </div>

      <div v-else-if="phase === 'error'" class="space-y-2">
        <p class="text-sm font-medium text-expense">检查更新失败</p>
        <p class="max-h-20 overflow-auto text-xs leading-relaxed text-ink-400">
          {{ errorText }}
        </p>
        <button class="text-sm text-deep hover:underline" @click="manualCheck">重试</button>
      </div>
    </div>
  </transition>
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
