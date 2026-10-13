// Tauri 命令封装层：前端只调这些函数，Rust 命令名保持一致
import { invoke } from "@tauri-apps/api/core";
import type {
  LedgerEntry,
  Ledger,
  Filters,
  EntriesSummary,
  ReceiptInfo,
  ImportResult,
  StatsBreakdown,
  CategoryBudget,
  EntryStatus,
} from "@/types";

// ---- 记账 ----
export const addEntry = (payload: Record<string, unknown>) =>
  invoke<number>("add_entry", { payload });
export const updateEntry = (id: number, payload: Record<string, unknown>) =>
  invoke<void>("update_entry", { id, payload });
export const deleteEntry = (id: number) => invoke<void>("delete_entry", { id });

// ---- 列表 / 统计 ----
export const queryEntries = (ledgerId: number, filters: Filters) =>
  invoke<LedgerEntry[]>("query_entries", { ledgerId, filters });
export const entriesSummary = (ledgerId: number, filters: Filters) =>
  invoke<EntriesSummary>("entries_summary", { ledgerId, filters });
/** 统计看板聚合（汇总 + 分类 + 月度 + 支付方式 + 预算） */
export const statsBreakdown = (ledgerId: number, filters: Filters) =>
  invoke<StatsBreakdown>("stats_breakdown", { ledgerId, filters });
/** 账套内已使用过的全部分类（不受分页限制） */
export const listCategories = (ledgerId: number) =>
  invoke<string[]>("list_categories", { ledgerId });
export const ledgerBudget = (ledgerId: number) =>
  invoke<number | null>("ledger_budget", { ledgerId });

// ---- 报销状态 ----
export const setEntryStatus = (
  id: number,
  status: EntryStatus,
  reimbursedAt: string | null = null,
) => invoke<void>("set_entry_status", { id, status, reimbursedAt });

// ---- 分类预算 ----
export const listCategoryBudgets = (ledgerId: number) =>
  invoke<CategoryBudget[]>("list_category_budgets", { ledgerId });
export const setCategoryBudget = (ledgerId: number, category: string, amount: number) =>
  invoke<void>("set_category_budget", { ledgerId, category, amount });
export const deleteCategoryBudget = (ledgerId: number, category: string) =>
  invoke<void>("delete_category_budget", { ledgerId, category });

// ---- 账套 ----
export const listLedgers = () => invoke<Ledger[]>("list_ledgers");
export const createLedger = (name: string, budget: number | null, note: string | null) =>
  invoke<number>("create_ledger", { name, budget, note });
export const updateLedger = (
  id: number,
  name: string | null,
  budget: number | null,
  note: string | null,
) => invoke<void>("update_ledger", { id, name, budget, note });
export const deleteLedger = (id: number) => invoke<void>("delete_ledger", { id });

// ---- 凭证 ----
export const importReceipt = (entryId: number, srcPath: string) =>
  invoke<string>("import_receipt", { entryId, srcPath });
export const listReceipts = (entryId: number) =>
  invoke<ReceiptInfo[]>("list_receipts", { entryId });
export const deleteReceipt = (id: number) => invoke<void>("delete_receipt", { id });
export const readFileBase64 = (path: string) =>
  invoke<string>("read_file_base64", { path });

// ---- 口令锁 ----
export const setVaultPassword = (password: string) =>
  invoke<void>("set_vault_password", { password });
export const hasVaultPassword = () => invoke<boolean>("has_vault_password");
export const verifyVaultPassword = (password: string) =>
  invoke<boolean>("verify_vault_password", { password });
export const clearVaultPassword = () => invoke<void>("clear_vault_password");

// ---- 导出 / 备份 ----
export const exportExcel = (ledgerId: number, path: string, filters: Filters) =>
  invoke<void>("export_excel", { ledgerId, path, filters });
export const exportCsv = (ledgerId: number, path: string, filters: Filters) =>
  invoke<void>("export_csv", { ledgerId, path, filters });
export const exportExcelTemplate = (path: string) =>
  invoke<void>("export_excel_template", { path });
export const importExcel = (ledgerId: number, path: string) =>
  invoke<ImportResult>("import_excel", { ledgerId, path });
export const backupDb = (path: string) => invoke<void>("backup_db", { path });
export const restoreDb = (path: string) => invoke<void>("restore_db", { path });
