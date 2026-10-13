<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useLedgerStore } from "@/stores/ledger";
import StatCard from "@/components/StatCard.vue";
import { fmtMoney, payLabel } from "@/lib/format";

const store = useLedgerStore();
const dateFrom = ref("");
const dateTo = ref("");

watch([dateFrom, dateTo], ([a, b]) => {
  store.setFilters({ date_from: a || null, date_to: b || null });
});

// 统计口径统一走后端 stats_breakdown：基于筛选后的**全量**数据聚合。
// 此前由当前分页（100 条）派生，记录超过一页时 KPI 与图表都会偏小。
const stats = computed(() => store.stats);

const categoryAgg = computed(() => stats.value?.by_category ?? []);
const monthAgg = computed(() => stats.value?.by_month ?? []);
const paymentAgg = computed(() => stats.value?.by_payment ?? []);

const maxCat = computed(() => Math.max(1, ...categoryAgg.value.map((c) => c.amount)));
const maxMonth = computed(() =>
  Math.max(1, ...monthAgg.value.map((m) => Math.max(m.expense, m.income))),
);
const maxPay = computed(() => Math.max(1, ...paymentAgg.value.map((p) => p.amount)));

const expenseTotal = computed(() => stats.value?.total_expense ?? 0);
const incomeTotal = computed(() => stats.value?.total_income ?? 0);
const net = computed(() => stats.value?.net ?? 0);
const totalCount = computed(() => stats.value?.count ?? 0);
/** 在途：已垫付但还没回款的部分 */
const pendingAmount = computed(() => stats.value?.pending_amount ?? 0);
const pendingCount = computed(() => stats.value?.pending_count ?? 0);

const budget = computed(() => stats.value?.budget ?? null);

/** 分类预算：category → 额度 */
const budgetMap = computed(
  () => new Map(store.categoryBudgets.map((b) => [b.category, b.amount])),
);

/** 分类维度的预算使用率（无预算返回 null） */
function catRatio(category: string, amount: number): number | null {
  const b = budgetMap.value.get(category);
  if (!b || b <= 0) return null;
  return amount / b;
}
const budgetRatio = computed(() =>
  budget.value && budget.value > 0 ? expenseTotal.value / budget.value : 0,
);
const budgetWarn = computed(() => budgetRatio.value >= 0.85);
</script>

<template>
  <section>
    <div class="flex items-center justify-between mb-4">
      <h1 class="text-xl font-medium text-deep">统计看板</h1>
      <div class="flex gap-2 items-end">
        <label class="text-xs text-ink-400">
          起
          <input v-model="dateFrom" type="date" class="mt-1 rounded-lg border border-line bg-card-alt px-2 py-1.5 text-ink outline-none focus:border-deep-500" />
        </label>
        <label class="text-xs text-ink-400">
          止
          <input v-model="dateTo" type="date" class="mt-1 rounded-lg border border-line bg-card-alt px-2 py-1.5 text-ink outline-none focus:border-deep-500" />
        </label>
      </div>
    </div>

    <!-- KPI -->
    <div class="grid grid-cols-2 md:grid-cols-5 gap-4">
      <StatCard label="支出" :value="fmtMoney(expenseTotal)" tone="expense" />
      <StatCard label="收入" :value="fmtMoney(incomeTotal)" tone="income" />
      <StatCard
        :label="'待报销 · ' + pendingCount + ' 笔'"
        :value="fmtMoney(pendingAmount)"
        tone="warn"
      />
      <StatCard label="净额" :value="fmtMoney(net)" />
      <StatCard label="笔数" :value="String(totalCount)" />
    </div>

    <!-- 在途提示 -->
    <div
      v-if="pendingAmount > 0"
      class="mt-4 rounded-xl border border-warn-fg bg-warn-bg p-4 text-sm text-warn-deep"
    >
      有 {{ pendingCount }} 笔支出共 {{ fmtMoney(pendingAmount) }} 尚未回款，可在台账页逐条标记状态。
    </div>

    <!-- 预算预警 -->
    <div
      v-if="budget && budget > 0"
      :class="[
        'mt-4 rounded-xl border p-4 text-sm',
        budgetWarn ? 'bg-warn-bg border-warn-fg text-warn-deep' : 'bg-card border-line text-ink-400',
      ]"
    >
      <div class="flex items-center justify-between">
        <span>预算 {{ fmtMoney(budget) }} · 已用 {{ (budgetRatio * 100).toFixed(1) }}%</span>
        <span v-if="budgetWarn">⚠ 支出已达预算 85%，注意控制</span>
      </div>
      <div class="mt-2 h-2 rounded-full bg-fog-300 overflow-hidden">
        <div
          class="h-full rounded-full"
          :class="budgetWarn ? 'bg-warn-fg' : 'bg-deep-500'"
          :style="{ width: Math.min(100, budgetRatio * 100) + '%' }"
        ></div>
      </div>
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-2 gap-4 mt-4">
      <!-- 分类占比 -->
      <div class="bg-card rounded-xl border border-line shadow-card p-5">
        <h2 class="text-sm font-medium text-ink-400 mb-3">分类支出与预算</h2>
        <div v-if="categoryAgg.length" class="space-y-2.5">
          <div v-for="c in categoryAgg" :key="c.category" class="text-sm">
            <div class="flex items-center gap-3">
              <span class="w-20 text-ink truncate">{{ c.category }}</span>
              <div class="flex-1 h-3 rounded-full bg-fog-300 overflow-hidden">
                <div
                  class="h-full rounded-full"
                  :class="[
                    catRatio(c.category, c.amount) == null
                      ? 'bg-deep-500'
                      : catRatio(c.category, c.amount)! >= 1
                        ? 'bg-expense'
                        : catRatio(c.category, c.amount)! >= 0.85
                          ? 'bg-warn-fg'
                          : 'bg-deep-500',
                  ]"
                  :style="{
                    width:
                      (catRatio(c.category, c.amount) == null
                        ? (c.amount / maxCat) * 100
                        : Math.min(100, catRatio(c.category, c.amount)! * 100)) + '%',
                  }"
                ></div>
              </div>
              <span class="w-24 text-right text-ink-400 tabular-nums">{{ fmtMoney(c.amount) }}</span>
            </div>
            <div
              v-if="catRatio(c.category, c.amount) != null"
              class="mt-0.5 flex items-center gap-2 text-xs"
              :class="
                catRatio(c.category, c.amount)! >= 1
                  ? 'text-expense'
                  : catRatio(c.category, c.amount)! >= 0.85
                    ? 'text-warn-fg'
                    : 'text-ink-300'
              "
            >
              <span class="w-20"></span>
              <span>
                预算 {{ fmtMoney(budgetMap.get(c.category) ?? 0) }} · 已用
                {{ (catRatio(c.category, c.amount)! * 100).toFixed(0) }}%
                <template v-if="catRatio(c.category, c.amount)! >= 1"> · 已超支</template>
              </span>
            </div>
          </div>
        </div>
        <p v-else class="text-ink-300 text-sm">暂无支出数据</p>
      </div>

      <!-- 月度趋势 -->
      <div class="bg-card rounded-xl border border-line shadow-card p-5">
        <h2 class="text-sm font-medium text-ink-400 mb-3">月度趋势</h2>
        <div v-if="monthAgg.length" class="space-y-3">
          <div v-for="m in monthAgg" :key="m.month" class="text-sm">
            <div class="flex justify-between text-xs text-ink-400 mb-1">
              <span>{{ m.month }}</span>
              <span>支 {{ fmtMoney(m.expense) }} / 收 {{ fmtMoney(m.income) }}</span>
            </div>
            <div class="flex h-3 rounded-full bg-fog-300 overflow-hidden">
              <div class="h-full bg-warn-deep" :style="{ width: (m.expense / maxMonth) * 100 + '%' }"></div>
              <div class="h-full bg-deep-500" :style="{ width: (m.income / maxMonth) * 100 + '%' }"></div>
            </div>
          </div>
        </div>
        <p v-else class="text-ink-300 text-sm">暂无数据</p>
      </div>

      <!-- 支付方式 -->
      <div class="bg-card rounded-xl border border-line shadow-card p-5 lg:col-span-2">
        <h2 class="text-sm font-medium text-ink-400 mb-3">支付方式构成（支出）</h2>
        <div v-if="paymentAgg.length" class="grid grid-cols-2 md:grid-cols-3 gap-3">
          <div v-for="p in paymentAgg" :key="p.payment" class="flex items-center gap-3 text-sm">
            <span class="w-20 text-ink truncate">{{ payLabel(p.payment) }}</span>
            <div class="flex-1 h-3 rounded-full bg-fog-300 overflow-hidden">
              <div class="h-full bg-deep-600 rounded-full" :style="{ width: (p.amount / maxPay) * 100 + '%' }"></div>
            </div>
            <span class="w-24 text-right text-ink-400 tabular-nums">{{ fmtMoney(p.amount) }}</span>
          </div>
        </div>
        <p v-else class="text-ink-300 text-sm">暂无支出数据</p>
      </div>
    </div>
  </section>
</template>
