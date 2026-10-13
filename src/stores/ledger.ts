import { defineStore } from "pinia";
import { ref } from "vue";
import * as api from "@/lib/commands";
import type {
  LedgerEntry,
  Ledger,
  Filters,
  EntriesSummary,
  StatsBreakdown,
  CategoryBudget,
  EntryStatus,
} from "@/types";

const PAGE = 100;

export const useLedgerStore = defineStore("ledger", () => {
  const ledgers = ref<Ledger[]>([]);
  const currentLedgerId = ref<number>(1);
  const filters = ref<Filters>({ sort: "date_desc", offset: 0, limit: PAGE });

  const entries = ref<LedgerEntry[]>([]);
  const summary = ref<EntriesSummary | null>(null);
  /** 统计看板聚合结果（后端按筛选后的全量数据计算，不受分页影响） */
  const stats = ref<StatsBreakdown | null>(null);
  const budget = ref<number | null>(null);
  const hasMore = ref(false);
  const loading = ref(false);

  /** 该账套用过的全部分类（用于录入页自动补全与筛选下拉，后端全量返回） */
  const categoryList = ref<string[]>([]);
  /** 分类预算（category → 额度） */
  const categoryBudgets = ref<CategoryBudget[]>([]);

  async function loadLedgers() {
    ledgers.value = await api.listLedgers();
    if (!ledgers.value.find((l) => l.id === currentLedgerId.value)) {
      currentLedgerId.value = ledgers.value[0]?.id ?? 1;
    }
  }

  async function refreshList() {
    const [rows, sum] = await Promise.all([
      api.queryEntries(currentLedgerId.value, { ...filters.value }),
      api.entriesSummary(currentLedgerId.value, {
        ...filters.value,
        offset: 0,
        limit: 1e9,
      }),
    ]);
    entries.value = rows;
    summary.value = sum;
    hasMore.value = rows.length >= (filters.value.limit ?? PAGE);
  }

  async function loadMore() {
    if (!hasMore.value || loading.value) return;
    loading.value = true;
    filters.value.offset = (filters.value.offset ?? 0) + (filters.value.limit ?? PAGE);
    const rows = await api.queryEntries(currentLedgerId.value, { ...filters.value });
    entries.value.push(...rows);
    hasMore.value = rows.length >= (filters.value.limit ?? PAGE);
    loading.value = false;
  }

  async function refreshStats() {
    const s = await api.statsBreakdown(currentLedgerId.value, {
      ...filters.value,
      offset: 0,
      limit: 1e9,
    });
    stats.value = s;
    budget.value = s.budget;
  }

  async function refreshCategories() {
    const list = await api.listCategories(currentLedgerId.value);
    categoryList.value = [...list].sort((a, b) => a.localeCompare(b, "zh-CN"));
  }

  async function refreshBudgets() {
    categoryBudgets.value = await api.listCategoryBudgets(currentLedgerId.value);
  }

  async function refreshAll() {
    await Promise.all([
      refreshList(),
      refreshStats(),
      refreshCategories(),
      refreshBudgets(),
    ]);
  }

  /** 一键流转报销状态 */
  async function setStatus(id: number, status: EntryStatus, reimbursedAt: string | null = null) {
    await api.setEntryStatus(id, status, reimbursedAt);
    await refreshAll();
  }

  async function setLedger(id: number) {
    currentLedgerId.value = id;
    filters.value.offset = 0;
    await refreshAll();
  }

  async function setFilters(patch: Partial<Filters>) {
    filters.value = { ...filters.value, ...patch, offset: 0 };
    // 统计看板与列表共用 filters，必须一起刷新，否则看板停留在上一组筛选的聚合结果
    await Promise.all([refreshList(), refreshStats()]);
  }

  async function add(payload: Record<string, unknown>) {
    await api.addEntry(payload);
    await refreshAll();
  }

  async function update(id: number, payload: Record<string, unknown>) {
    await api.updateEntry(id, payload);
    await refreshAll();
  }

  async function remove(id: number) {
    await api.deleteEntry(id);
    await refreshAll();
  }

  return {
    ledgers,
    currentLedgerId,
    filters,
    entries,
    summary,
    stats,
    budget,
    hasMore,
    loading,
    categoryList,
    categoryBudgets,
    loadLedgers,
    refreshList,
    loadMore,
    refreshStats,
    refreshCategories,
    refreshBudgets,
    setStatus,
    refreshAll,
    setLedger,
    setFilters,
    add,
    update,
    remove,
  };
});
