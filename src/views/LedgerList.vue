<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { useLedgerStore } from "@/stores/ledger";
import EntryForm from "@/views/EntryForm.vue";
import { exportExcel, exportCsv, queryEntries, exportExcelTemplate, importExcel } from "@/lib/commands";
import { saveAs, pickExcelFile } from "@/lib/dialog";
import { requireVault } from "@/lib/vault";
import { categoriesFor, EXPENSE_CATEGORIES, INCOME_CATEGORIES } from "@/constants/categories";
import { fmtMoney, payLabel, typeLabel, statusLabel, STATUS_OPTIONS } from "@/lib/format";
import type { EntryStatus, LedgerEntry } from "@/types";

const store = useLedgerStore();
const editing = ref<LedgerEntry | null>(null);
const showModal = ref(false);
const busy = ref(false);
const tip = ref("");

// 用 computed 跟随 store.filters（setFilters 会整体替换对象）
const f = computed(() => store.filters);
const printRows = ref<LedgerEntry[]>([]);
const importErrors = ref<string[]>([]);

// 筛选分类下拉：预设分类 + 已使用过的分类合并去重（跟随类型筛选联动）
const categoryOptions = computed(() => {
  const tf = f.value.type_filter;
  const base = tf ? categoriesFor(tf) : [...EXPENSE_CATEGORIES, ...INCOME_CATEGORIES];
  const set = new Set<string>(base);
  // 后端返回该账套用过的全部分类（不受分页限制）
  for (const c of store.categoryList) set.add(c);
  return [...set];
});

// 关键词：本地暂存 + 300ms 防抖，避免每敲一个字就跑一次全量查询
const keywordInput = ref(store.filters.keyword ?? "");
let kwTimer: number | undefined;
watch(keywordInput, (v) => {
  window.clearTimeout(kwTimer);
  kwTimer = window.setTimeout(() => {
    store.setFilters({ keyword: v.trim() || null });
  }, 300);
});

async function doExport(kind: "xlsx" | "csv") {
  // 已开启口令锁时，导出属于敏感操作，需先验证
  if (!(await requireVault(kind === "xlsx" ? "导出 Excel" : "导出 CSV"))) return;
  const name = `台账_${new Date().toISOString().slice(0, 10)}.${kind}`;
  const path = await saveAs(kind, name);
  if (!path) return;
  busy.value = true;
  try {
    const filters = { ...store.filters, offset: 0, limit: 1e9 };
    if (kind === "xlsx") await exportExcel(store.currentLedgerId, path, filters);
    else await exportCsv(store.currentLedgerId, path, filters);
    tip.value = "已导出：" + path;
  } catch (e) {
    tip.value = "导出失败：" + String(e);
  } finally {
    busy.value = false;
    setTimeout(() => (tip.value = ""), 4000);
  }
}

const printedAt = ref("");

/** 打印区合计（用筛选后的全量数据，与列表 tfoot 口径一致） */
const printExpense = computed(() =>
  printRows.value.filter((e) => e.type === "expense").reduce((s, e) => s + e.amount, 0),
);
const printIncome = computed(() =>
  printRows.value.filter((e) => e.type === "income").reduce((s, e) => s + e.amount, 0),
);
/** 打印区：还没回款的部分 */
const printPending = computed(() =>
  printRows.value
    .filter((e) => e.type === "expense" && (e.status === "pending" || e.status === "submitted"))
    .reduce((s, e) => s + e.amount, 0),
);

async function printPdf() {
  // 打印全部筛选结果（而非仅当前分页）
  printRows.value = await queryEntries(store.currentLedgerId, {
    ...store.filters,
    offset: 0,
    limit: 1e9,
  });
  printedAt.value = new Date().toLocaleString("zh-CN");
  await new Promise((r) => setTimeout(r, 50));
  window.print();
}

/** 下载空白导入模板（含示例行与填写说明） */
async function downloadTemplate() {
  const path = await saveAs("xlsx", "报销台账导入模板.xlsx");
  if (!path) return;
  busy.value = true;
  try {
    await exportExcelTemplate(path);
    tip.value = "模板已保存：" + path + "（在「台账数据」表填写，删除示例行后导入）";
  } catch (e) {
    tip.value = "保存模板失败：" + String(e);
  } finally {
    busy.value = false;
    setTimeout(() => (tip.value = ""), 6000);
  }
}

/** 按模板导入 Excel 台账数据 */
async function doImport() {
  const path = await pickExcelFile();
  if (!path) return;
  busy.value = true;
  importErrors.value = [];
  try {
    const r = await importExcel(store.currentLedgerId, path);
    if (r.inserted > 0) await store.refreshAll();
    const parts = [`成功导入 ${r.inserted} 条`];
    if (r.skipped) parts.push(`自动跳过示例行 ${r.skipped} 条`);
    if (r.errors.length) parts.push(`失败 ${r.errors.length} 条`);
    tip.value = "导入完成：" + parts.join("，");
    importErrors.value = r.errors;
  } catch (e) {
    tip.value = "导入失败：" + String(e);
  } finally {
    busy.value = false;
    setTimeout(() => (tip.value = ""), 6000);
  }
}

/** 报销状态下拉：支出可流转，收入不涉及 */
const statusChoices = STATUS_OPTIONS.filter((o) => o.value !== "none");

async function changeStatus(e: LedgerEntry, s: EntryStatus) {
  await store.setStatus(e.id, s);
}

function openEdit(e: LedgerEntry) {
  editing.value = e;
  showModal.value = true;
}
function closeModal() {
  showModal.value = false;
  editing.value = null;
}

async function removeRow(e: LedgerEntry) {
  if (!confirm(`确认删除「${e.date} ${e.category} ${fmtMoney(e.amount)}」？`)) return;
  await store.remove(e.id);
}
</script>

<template>
  <section>
    <div class="no-print flex items-center justify-between mb-4">
      <h1 class="text-xl font-medium text-deep">台账明细</h1>
      <div class="flex gap-2">
        <button
          @click="doImport"
          :disabled="busy"
          class="text-sm rounded-lg border border-deep text-deep px-3 py-1.5 hover:bg-deep/5 disabled:opacity-60"
        >
          导入 Excel
        </button>
        <button
          @click="downloadTemplate"
          :disabled="busy"
          class="text-sm rounded-lg border border-line text-ink-400 px-3 py-1.5 hover:bg-card-alt disabled:opacity-60"
        >
          下载模板
        </button>
        <button
          @click="doExport('xlsx')"
          :disabled="busy"
          class="text-sm rounded-lg bg-deep text-white px-3 py-1.5 hover:bg-deep-600 disabled:opacity-60"
        >
          导出 Excel
        </button>
        <button
          @click="doExport('csv')"
          :disabled="busy"
          class="text-sm rounded-lg border border-deep text-deep px-3 py-1.5 hover:bg-deep/5 disabled:opacity-60"
        >
          导出 CSV
        </button>
        <button
          @click="printPdf"
          class="text-sm rounded-lg border border-line text-ink-400 px-3 py-1.5 hover:bg-card-alt"
        >
          打印 / PDF
        </button>
      </div>
    </div>

    <!-- 筛选条 -->
    <div class="no-print bg-card rounded-xl border border-line shadow-card p-4 mb-4 grid grid-cols-2 md:grid-cols-6 gap-3 items-end">
      <label class="text-xs text-ink-400">
        起
        <input
          :value="f.date_from ?? ''"
          @change="store.setFilters({ date_from: ($event.target as HTMLInputElement).value || null })"
          type="date"
          class="mt-1 w-full rounded-lg border border-line bg-card-alt px-2 py-1.5 text-ink outline-none focus:border-deep-500"
        />
      </label>
      <label class="text-xs text-ink-400">
        止
        <input
          :value="f.date_to ?? ''"
          @change="store.setFilters({ date_to: ($event.target as HTMLInputElement).value || null })"
          type="date"
          class="mt-1 w-full rounded-lg border border-line bg-card-alt px-2 py-1.5 text-ink outline-none focus:border-deep-500"
        />
      </label>
      <label class="text-xs text-ink-400">
        类型
        <select
          :value="f.type_filter ?? ''"
          @change="store.setFilters({ type_filter: ($event.target as HTMLSelectElement).value || null })"
          class="mt-1 w-full rounded-lg border border-line bg-card-alt px-2 py-1.5 text-ink outline-none focus:border-deep-500"
        >
          <option value="">全部</option>
          <option value="expense">支出</option>
          <option value="income">收入</option>
        </select>
      </label>
      <label class="text-xs text-ink-400">
        分类
        <select
          :value="f.category ?? ''"
          @change="store.setFilters({ category: ($event.target as HTMLSelectElement).value || null })"
          class="mt-1 w-full rounded-lg border border-line bg-card-alt px-2 py-1.5 text-ink outline-none focus:border-deep-500"
        >
          <option value="">全部</option>
          <option v-for="c in categoryOptions" :key="c" :value="c">{{ c }}</option>
        </select>
      </label>
      <label class="text-xs text-ink-400">
        支付方式
        <select
          :value="f.payment ?? ''"
          @change="store.setFilters({ payment: ($event.target as HTMLSelectElement).value || null })"
          class="mt-1 w-full rounded-lg border border-line bg-card-alt px-2 py-1.5 text-ink outline-none focus:border-deep-500"
        >
          <option value="">全部</option>
          <option value="cash">现金</option>
          <option value="bank">银行转账</option>
          <option value="alipay">支付宝</option>
          <option value="wechat">微信</option>
          <option value="card">刷卡</option>
        </select>
      </label>
      <label class="text-xs text-ink-400">
        报销状态
        <select
          :value="f.status ?? ''"
          @change="store.setFilters({ status: (($event.target as HTMLSelectElement).value || null) as EntryStatus | null })"
          class="mt-1 w-full rounded-lg border border-line bg-card-alt px-2 py-1.5 text-ink outline-none focus:border-deep-500"
        >
          <option value="">全部</option>
          <option value="pending">待报销</option>
          <option value="submitted">已提交</option>
          <option value="reimbursed">已回款</option>
          <option value="none">不涉及</option>
        </select>
      </label>
      <label class="text-xs text-ink-400">
        关键词
        <input
          v-model="keywordInput"
          placeholder="往来方/备注"
          class="mt-1 w-full rounded-lg border border-line bg-card-alt px-2 py-1.5 text-ink outline-none focus:border-deep-500"
        />
      </label>
      <label class="text-xs text-ink-400 col-span-2 md:col-span-4">
        排序
        <select
          :value="f.sort ?? 'date_desc'"
          @change="store.setFilters({ sort: ($event.target as HTMLSelectElement).value as any })"
          class="mt-1 w-full rounded-lg border border-line bg-card-alt px-2 py-1.5 text-ink outline-none focus:border-deep-500"
        >
          <option value="date_desc">日期 ↓</option>
          <option value="date_asc">日期 ↑</option>
          <option value="amount_desc">金额 ↓</option>
        </select>
      </label>
    </div>

    <p v-if="tip" class="no-print text-xs text-ink-400 mb-2">{{ tip }}</p>

    <!-- 导入失败明细 -->
    <div
      v-if="importErrors.length"
      class="no-print mb-4 rounded-xl border border-expense/30 bg-expense/5 p-3"
    >
      <div class="flex items-center justify-between">
        <span class="text-sm font-medium text-expense">
          有 {{ importErrors.length }} 条数据导入失败（其余已成功写入）
        </span>
        <button
          class="text-xs text-ink-400 hover:text-ink"
          @click="importErrors = []"
        >
          关闭
        </button>
      </div>
      <ul class="mt-1.5 max-h-40 space-y-0.5 overflow-auto text-xs text-ink-400">
        <li v-for="(e, i) in importErrors" :key="i">· {{ e }}</li>
      </ul>
    </div>

    <!-- 表格 -->
    <div class="no-print bg-card rounded-xl border border-line shadow-card overflow-hidden">
      <table class="w-full text-sm">
        <thead class="bg-fog-300 text-ink-400">
          <tr>
            <th class="text-left px-4 py-2.5 font-medium">日期</th>
            <th class="text-left px-4 py-2.5 font-medium">类型</th>
            <th class="text-left px-4 py-2.5 font-medium">分类</th>
            <th class="text-right px-4 py-2.5 font-medium">金额</th>
            <th class="text-left px-4 py-2.5 font-medium">往来方</th>
            <th class="text-left px-4 py-2.5 font-medium">方式</th>
            <th class="text-left px-4 py-2.5 font-medium">报销状态</th>
            <th class="text-right px-4 py-2.5 font-medium">操作</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="e in store.entries"
            :key="e.id"
            class="border-t border-line hover:bg-card-alt"
          >
            <td class="px-4 py-2.5 text-ink tabular-nums">{{ e.date }}</td>
            <td class="px-4 py-2.5">
              <span
                :class="e.type === 'expense' ? 'text-expense' : 'text-income'"
                class="text-xs font-medium"
                >{{ typeLabel(e.type) }}</span
              >
            </td>
            <td class="px-4 py-2.5 text-ink">{{ e.category }}</td>
            <td
              class="px-4 py-2.5 text-right font-medium tabular-nums"
              :class="e.type === 'expense' ? 'text-expense' : 'text-income'"
            >
              {{ e.type === "expense" ? "-" : "+" }}{{ e.amount.toFixed(2) }}
            </td>
            <td class="px-4 py-2.5 text-ink-400">{{ e.party || "—" }}</td>
            <td class="px-4 py-2.5 text-ink-400">{{ payLabel(e.payment) }}</td>
            <td class="px-4 py-2.5">
              <select
                v-if="e.type === 'expense'"
                :value="e.status"
                @change="changeStatus(e, ($event.target as HTMLSelectElement).value as EntryStatus)"
                class="rounded border border-line bg-card-alt px-1.5 py-1 text-xs text-ink outline-none focus:border-deep-500"
              >
                <option v-for="s in statusChoices" :key="s.value" :value="s.value">
                  {{ s.label }}
                </option>
              </select>
              <span v-else class="text-xs text-ink-300">—</span>
            </td>
            <td class="px-4 py-2.5 text-right whitespace-nowrap">
              <button @click="openEdit(e)" class="text-deep text-xs hover:underline mr-3">编辑</button>
              <button @click="removeRow(e)" class="text-warn-deep text-xs hover:underline">删除</button>
            </td>
          </tr>
          <tr v-if="!store.entries.length">
            <td colspan="8" class="px-4 py-10 text-center text-ink-300">暂无记录，去「记一笔」添加</td>
          </tr>
        </tbody>
        <tfoot v-if="store.summary" class="bg-fog-300 text-ink-400">
          <tr class="font-medium">
            <td class="px-4 py-2.5" colspan="3">
              共 {{ store.summary.count }} 笔 · 净额 {{ fmtMoney(store.summary.net) }}
            </td>
            <td class="px-4 py-2.5 text-right text-warn-deep tabular-nums">
              支 {{ fmtMoney(store.summary.total_expense) }}
            </td>
            <td></td>
            <td class="px-4 py-2.5 text-income tabular-nums">收 {{ fmtMoney(store.summary.total_income) }}</td>
            <td></td>
            <td></td>
          </tr>
        </tfoot>
      </table>
    </div>

    <div class="no-print mt-3 flex justify-center">
      <button
        v-if="store.hasMore"
        @click="store.loadMore()"
        :disabled="store.loading"
        class="text-sm text-deep border border-deep/40 rounded-lg px-4 py-1.5 hover:bg-deep/5 disabled:opacity-60"
      >
        {{ store.loading ? "加载中…" : "加载更多" }}
      </button>
    </div>

    <!-- 编辑弹窗 -->
    <div
      v-if="showModal"
      class="no-print fixed inset-0 bg-black/40 flex items-start justify-center p-6 overflow-auto z-50"
      @click.self="closeModal"
    >
      <div class="bg-fog rounded-xl shadow-2xl w-full max-w-2xl my-4">
        <EntryForm :edit-entry="editing" @saved="closeModal" @cancel="closeModal" />
      </div>
    </div>

    <!-- 打印区（仅打印时可见） -->
    <section class="print-area hidden">
      <h2 style="font-size: 15pt; margin: 0 0 4pt">
        报销台账 · {{ store.ledgers.find((l) => l.id === store.currentLedgerId)?.name }}
      </h2>
      <p style="font-size: 9pt; color: #555; margin: 0 0 8pt">
        打印时间：{{ printedAt }}
        <span v-if="f.date_from || f.date_to">
          ｜ 区间：{{ f.date_from ?? "不限" }} ~ {{ f.date_to ?? "不限" }}</span
        >
        <span v-if="f.type_filter"> ｜ 类型：{{ typeLabel(f.type_filter) }}</span>
        <span v-if="f.category"> ｜ 分类：{{ f.category }}</span>
        <span v-if="f.payment"> ｜ 方式：{{ payLabel(f.payment) }}</span>
        <span v-if="f.keyword"> ｜ 关键词：{{ f.keyword }}</span>
      </p>
      <table border="1" cellspacing="0" cellpadding="6" width="100%" style="border-collapse: collapse; font-size: 10pt">
        <thead>
          <tr>
            <th>日期</th><th>类型</th><th>分类</th><th>金额</th><th>往来方</th><th>方式</th><th>状态</th><th>备注</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="e in printRows" :key="e.id">
            <td>{{ e.date }}</td>
            <td>{{ typeLabel(e.type) }}</td>
            <td>{{ e.category }}</td>
            <td style="text-align: right">{{ e.amount.toFixed(2) }}</td>
            <td>{{ e.party || "" }}</td>
            <td>{{ payLabel(e.payment) }}</td>
            <td>{{ e.type === "expense" ? statusLabel(e.status) : "—" }}</td>
            <td>{{ e.note || "" }}</td>
          </tr>
        </tbody>
        <tfoot>
          <tr style="font-weight: 600">
            <td colspan="3">合计 {{ printRows.length }} 笔</td>
            <td style="text-align: right">
              支 {{ printExpense.toFixed(2) }} / 收 {{ printIncome.toFixed(2) }}
            </td>
            <td colspan="4">
              待报销 {{ printPending.toFixed(2) }} ｜ 净额 {{ (printIncome - printExpense).toFixed(2) }}
            </td>
          </tr>
        </tfoot>
      </table>
    </section>
  </section>
</template>

<style scoped>
@media print {
  .print-area {
    display: block !important;
  }
}
</style>
