<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useLedgerStore } from "@/stores/ledger";
import {
  createLedger,
  updateLedger,
  deleteLedger,
  hasVaultPassword,
  setVaultPassword,
  verifyVaultPassword,
  clearVaultPassword,
  backupDb,
  restoreDb,
  setCategoryBudget,
} from "@/lib/commands";
import { saveAs, pickAnyFile } from "@/lib/dialog";
import { requireVault } from "@/lib/vault";
import { fmtMoney } from "@/lib/format";
import { EXPENSE_CATEGORIES } from "@/constants/categories";
import type { Ledger } from "@/types";

const store = useLedgerStore();
const msg = ref("");

// 账套
const newName = ref("");
const newBudget = ref<number | null>(null);

async function addLedger() {
  if (!newName.value.trim()) {
    msg.value = "账套名称不能为空";
    return;
  }
  await createLedger(newName.value.trim(), newBudget.value, null);
  await store.loadLedgers();
  newName.value = "";
  newBudget.value = null;
  msg.value = "已新建账套";
}

async function saveLedger(l: Ledger) {
  const b = l.budget == null ? null : Number(l.budget);
  await updateLedger(l.id, l.name, b, l.note);
  msg.value = "账套已保存";
}

async function delLedger(l: Ledger) {
  if (l.id === 1) {
    msg.value = "默认台账不可删除";
    return;
  }
  if (!confirm(`确认删除账套「${l.name}」及其全部记录？`)) return;
  await deleteLedger(l.id);
  await store.loadLedgers();
  if (store.currentLedgerId === l.id) await store.setLedger(store.ledgers[0]?.id ?? 1);
  msg.value = "账套已删除";
}

function onBudgetInput(l: Ledger, e: Event) {
  const v = (e.target as HTMLInputElement).value;
  l.budget = v === "" ? null : Number(v);
}

// 分类预算
const newCat = ref("");
const newCatAmount = ref<number | null>(null);
// 下拉可选：常用支出分类 + 该账套已用过的分类
const catChoices = computed(() => {
  const set = new Set<string>([...EXPENSE_CATEGORIES, ...store.categoryList]);
  return [...set];
});

async function addCatBudget() {
  const c = newCat.value.trim();
  if (!c) {
    msg.value = "请选择或填写分类";
    return;
  }
  if (!newCatAmount.value || newCatAmount.value <= 0) {
    msg.value = "预算金额需大于 0";
    return;
  }
  await setCategoryBudget(store.currentLedgerId, c, newCatAmount.value);
  newCat.value = "";
  newCatAmount.value = null;
  await store.refreshBudgets();
  msg.value = `已设置「${c}」的预算`;
}

async function saveCatBudget(category: string, amount: number | null) {
  await setCategoryBudget(store.currentLedgerId, category, amount ?? 0);
  await store.refreshBudgets();
  msg.value = amount && amount > 0 ? `已更新「${category}」预算` : `已删除「${category}」预算`;
}

function onCatAmountInput(e: Event, category: string) {
  const v = Number((e.target as HTMLInputElement).value);
  void saveCatBudget(category, Number.isFinite(v) && v > 0 ? v : 0);
}

// 口令锁
const vaultOn = ref(false);
const oldPwd = ref("");
const newPwd = ref("");
const newPwd2 = ref("");

async function setVault() {
  if (newPwd.value.length < 4) {
    msg.value = "口令至少 4 位";
    return;
  }
  if (newPwd.value !== newPwd2.value) {
    msg.value = "两次输入不一致";
    return;
  }
  await setVaultPassword(newPwd.value);
  vaultOn.value = true;
  oldPwd.value = newPwd.value = newPwd2.value = "";
  msg.value = "口令锁已启用";
}

async function changeVault() {
  if (!(await verifyVaultPassword(oldPwd.value))) {
    msg.value = "原口令不正确";
    return;
  }
  if (newPwd.value.length < 4 || newPwd.value !== newPwd2.value) {
    msg.value = "新口令至少 4 位且两次一致";
    return;
  }
  await setVaultPassword(newPwd.value);
  oldPwd.value = newPwd.value = newPwd2.value = "";
  msg.value = "口令已更新";
}

async function turnOffVault() {
  if (!(await verifyVaultPassword(oldPwd.value))) {
    msg.value = "原口令不正确";
    return;
  }
  await clearVaultPassword();
  vaultOn.value = false;
  oldPwd.value = "";
  msg.value = "已关闭口令锁";
}

// 备份 / 恢复
async function doBackup() {
  if (!(await requireVault("备份数据库"))) return;
  const path = await saveAs("db", `报销台账备份_${new Date().toISOString().slice(0, 10)}.db`);
  if (!path) return;
  await backupDb(path);
  msg.value = "已备份到：" + path;
}

async function doRestore() {
  if (!(await requireVault("从备份恢复"))) return;
  const path = await pickAnyFile();
  if (!path) return;
  if (!confirm("恢复将覆盖当前数据，建议先备份。继续？")) return;
  await restoreDb(path);
  await store.refreshAll();
  msg.value = "恢复完成，请重启应用以完全生效";
}

onMounted(async () => {
  vaultOn.value = await hasVaultPassword();
});
</script>

<template>
  <section class="max-w-3xl space-y-6">
    <h1 class="text-xl font-medium text-deep">设置</h1>
    <p v-if="msg" class="text-sm text-ink-400">{{ msg }}</p>

    <!-- 账套管理 -->
    <div class="bg-card rounded-xl border border-line shadow-card p-5">
      <h2 class="font-medium text-ink mb-3">账套管理</h2>
      <div class="space-y-3">
        <div
          v-for="l in store.ledgers"
          :key="l.id"
          class="flex flex-wrap items-center gap-3 border-t border-line pt-3 first:border-0 first:pt-0"
        >
          <input
            v-model="l.name"
            class="rounded-lg border border-line bg-card-alt px-3 py-1.5 text-sm text-ink outline-none focus:border-deep-500"
          />
          <label class="text-xs text-ink-400">
            预算
            <input
              :value="l.budget ?? ''"
              @input="onBudgetInput(l, $event)"
              type="number"
              placeholder="无"
              class="ml-1 w-28 rounded-lg border border-line bg-card-alt px-2 py-1.5 text-sm text-ink outline-none focus:border-deep-500"
            />
          </label>
          <span v-if="l.budget != null" class="text-xs text-ink-300">{{ fmtMoney(l.budget) }}</span>
          <div class="flex-1"></div>
          <button @click="saveLedger(l)" class="text-deep text-sm hover:underline">保存</button>
          <button
            v-if="l.id !== 1"
            @click="delLedger(l)"
            class="text-warn-deep text-sm hover:underline"
          >
            删除
          </button>
          <span v-else class="text-xs text-ink-300">默认</span>
        </div>
      </div>
      <div class="flex flex-wrap items-center gap-3 mt-4 border-t border-line pt-4">
        <input
          v-model="newName"
          placeholder="新账套名称"
          class="rounded-lg border border-line bg-card-alt px-3 py-1.5 text-sm text-ink outline-none focus:border-deep-500"
        />
        <input
          v-model.number="newBudget"
          type="number"
          placeholder="预算(可选)"
          class="w-32 rounded-lg border border-line bg-card-alt px-2 py-1.5 text-sm text-ink outline-none focus:border-deep-500"
        />
        <button
          @click="addLedger"
          class="rounded-lg bg-deep text-white text-sm px-3 py-1.5 hover:bg-deep-600"
        >
          新建账套
        </button>
      </div>
    </div>

    <!-- 分类预算 -->
    <div class="bg-card rounded-xl border border-line shadow-card p-5">
      <h2 class="font-medium text-ink mb-1">分类预算</h2>
      <p class="text-xs text-ink-400 mb-3">
        按分类设置额度，统计看板会逐条显示已用比例，超过 85% 变琥珀色、超支标红。清空金额即删除该分类预算。
      </p>

      <div v-if="store.categoryBudgets.length" class="space-y-3">
        <div
          v-for="b in store.categoryBudgets"
          :key="b.category"
          class="flex flex-wrap items-center gap-3 border-t border-line pt-3 first:border-0 first:pt-0"
        >
          <span class="w-24 text-sm text-ink">{{ b.category }}</span>
          <input
            :value="b.amount"
            @change="onCatAmountInput($event, b.category)"
            type="number"
            class="w-32 rounded-lg border border-line bg-card-alt px-2 py-1.5 text-sm text-ink outline-none focus:border-deep-500"
          />
          <span class="text-xs text-ink-300">{{ fmtMoney(b.amount) }}</span>
          <div class="flex-1"></div>
          <button
            @click="saveCatBudget(b.category, 0)"
            class="text-warn-deep text-sm hover:underline"
          >
            删除
          </button>
        </div>
      </div>
      <p v-else class="text-sm text-ink-300 mb-3">尚未设置分类预算</p>

      <div class="flex flex-wrap items-center gap-3 mt-4 border-t border-line pt-4">
        <input
          v-model="newCat"
          list="cat-budget-list"
          placeholder="分类"
          class="rounded-lg border border-line bg-card-alt px-3 py-1.5 text-sm text-ink outline-none focus:border-deep-500"
        />
        <datalist id="cat-budget-list">
          <option v-for="c in catChoices" :key="c" :value="c" />
        </datalist>
        <input
          v-model.number="newCatAmount"
          type="number"
          placeholder="预算金额"
          class="w-32 rounded-lg border border-line bg-card-alt px-2 py-1.5 text-sm text-ink outline-none focus:border-deep-500"
        />
        <button
          @click="addCatBudget"
          class="rounded-lg bg-deep text-white text-sm px-3 py-1.5 hover:bg-deep-600"
        >
          添加预算
        </button>
      </div>
    </div>

    <!-- 口令锁 -->
    <div class="bg-card rounded-xl border border-line shadow-card p-5">
      <h2 class="font-medium text-ink mb-1">口令锁</h2>
      <p class="text-xs text-ink-400 mb-3">开启后，敏感操作（导出/恢复）需验证口令。仅本地，不上云。</p>

      <div v-if="!vaultOn" class="space-y-2">
        <div class="flex gap-3">
          <input
            v-model="newPwd"
            type="password"
            placeholder="设置口令（≥4位）"
            class="rounded-lg border border-line bg-card-alt px-3 py-1.5 text-sm text-ink outline-none focus:border-deep-500"
          />
          <input
            v-model="newPwd2"
            type="password"
            placeholder="再次输入"
            class="rounded-lg border border-line bg-card-alt px-3 py-1.5 text-sm text-ink outline-none focus:border-deep-500"
          />
          <button @click="setVault" class="rounded-lg bg-deep text-white text-sm px-3 py-1.5 hover:bg-deep-600">
            启用
          </button>
        </div>
      </div>

      <div v-else class="space-y-2">
        <div class="flex gap-3 items-center">
          <input
            v-model="oldPwd"
            type="password"
            placeholder="原口令"
            class="rounded-lg border border-line bg-card-alt px-3 py-1.5 text-sm text-ink outline-none focus:border-deep-500"
          />
          <button @click="turnOffVault" class="text-warn-deep text-sm hover:underline">关闭口令锁</button>
        </div>
        <div class="flex gap-3">
          <input
            v-model="newPwd"
            type="password"
            placeholder="新口令"
            class="rounded-lg border border-line bg-card-alt px-3 py-1.5 text-sm text-ink outline-none focus:border-deep-500"
          />
          <input
            v-model="newPwd2"
            type="password"
            placeholder="再次输入"
            class="rounded-lg border border-line bg-card-alt px-3 py-1.5 text-sm text-ink outline-none focus:border-deep-500"
          />
          <button @click="changeVault" class="rounded-lg bg-deep text-white text-sm px-3 py-1.5 hover:bg-deep-600">
            修改口令
          </button>
        </div>
      </div>
    </div>

    <!-- 备份与恢复 -->
    <div class="bg-card rounded-xl border border-line shadow-card p-5">
      <h2 class="font-medium text-ink mb-3">数据备份与恢复</h2>
      <div class="flex gap-3">
        <button
          @click="doBackup"
          class="rounded-lg border border-deep text-deep text-sm px-3 py-1.5 hover:bg-deep/5"
        >
          备份数据库
        </button>
        <button
          @click="doRestore"
          class="rounded-lg border border-line text-ink-400 text-sm px-3 py-1.5 hover:bg-card-alt"
        >
          从备份恢复
        </button>
      </div>
      <p class="text-xs text-ink-300 mt-2">所有数据均存储于本机 AppData，备份文件请自行妥善保管。</p>
    </div>

    <!-- 关于 -->
    <div class="bg-card rounded-xl border border-line shadow-card p-5">
      <h2 class="font-medium text-ink mb-2">关于</h2>
      <p class="text-sm text-ink-400 leading-relaxed">
        报销台账 v0.4.0<br />
        数据仅存于本机，不上传任何云端；支持 Excel / CSV 导出与打印 PDF。
      </p>
    </div>
  </section>
</template>
