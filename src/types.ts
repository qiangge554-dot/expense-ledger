// 与 Rust 侧结构对齐的前端类型

export type EntryType = "expense" | "income";
export type PaymentMethod = "cash" | "bank" | "alipay" | "wechat" | "card";
/** 报销状态：none=不涉及 / pending=待报销 / submitted=已提交 / reimbursed=已回款 */
export type EntryStatus = "none" | "pending" | "submitted" | "reimbursed";

export interface LedgerEntry {
  id: number;
  ledger_id: number;
  date: string;
  type: EntryType;
  category: string;
  amount: number;
  party: string | null;
  payment: string | null;
  note: string | null;
  receipt_ref: string | null;
  /** 关联凭证张数（由后端 receipts 表统计） */
  receipt_count: number;
  status: EntryStatus;
  reimbursed_at: string | null;
  created_at: string;
}

/** 分类预算（按账套 + 分类设置额度） */
export interface CategoryBudget {
  category: string;
  amount: number;
}

export interface Ledger {
  id: number;
  name: string;
  budget: number | null;
  note: string | null;
}

export interface ReceiptInfo {
  id: number;
  path: string;
}

export interface Filters {
  date_from?: string | null;
  date_to?: string | null;
  type_filter?: string | null; // "expense" | "income" | null
  category?: string | null;
  payment?: string | null;
  /** 报销状态筛选 */
  status?: EntryStatus | null;
  keyword?: string | null;
  sort?: "date_desc" | "date_asc" | "amount_desc";
  offset?: number;
  limit?: number;
}

export interface EntriesSummary {
  total_expense: number;
  total_income: number;
  net: number;
  count: number;
}

export interface MonthlyPoint {
  month: string;
  expense: number;
  income: number;
}

export interface PaymentShare {
  payment: string;
  amount: number;
  count: number;
}

export interface CategoryAmount {
  category: string;
  amount: number;
}

/** 统计看板聚合结果（后端按筛选后的全量数据计算，不受分页影响） */
export interface StatsBreakdown {
  total_expense: number;
  total_income: number;
  net: number;
  count: number;
  /** 在途金额（待报销 + 已提交）：自己垫付、公司还没回款的部分 */
  pending_amount: number;
  pending_count: number;
  budget: number | null;
  by_category: CategoryAmount[];
  by_month: MonthlyPoint[];
  by_payment: PaymentShare[];
}

export interface ImportResult {
  inserted: number;
  skipped: number;
  errors: string[];
}
