export const PAYMENT_LABELS: Record<string, string> = {
  cash: "现金",
  bank: "银行转账",
  alipay: "支付宝",
  wechat: "微信",
  card: "刷卡",
};

export const TYPE_LABELS: Record<string, string> = {
  expense: "支出",
  income: "收入",
};

export function payLabel(p?: string | null): string {
  if (!p) return "—";
  return PAYMENT_LABELS[p] ?? p;
}

export function typeLabel(t: string): string {
  return TYPE_LABELS[t] ?? t;
}

export const STATUS_LABELS: Record<string, string> = {
  none: "不涉及",
  pending: "待报销",
  submitted: "已提交",
  reimbursed: "已回款",
};

/** 状态下拉选项（录入页与筛选条共用） */
export const STATUS_OPTIONS: { value: string; label: string }[] = [
  { value: "none", label: "不涉及" },
  { value: "pending", label: "待报销" },
  { value: "submitted", label: "已提交" },
  { value: "reimbursed", label: "已回款" },
];

export function statusLabel(s?: string | null): string {
  if (!s) return "—";
  return STATUS_LABELS[s] ?? s;
}

/** 状态徽标配色：在途用琥珀色，已回款用青绿，不涉及弱化 */
export function statusTone(s: string): string {
  switch (s) {
    case "pending":
      return "bg-warn-bg text-warn-fg";
    case "submitted":
      return "bg-fog-300 text-deep";
    case "reimbursed":
      return "bg-income/10 text-income";
    default:
      return "text-ink-300";
  }
}

export function fmtMoney(n: number): string {
  return (
    "¥" +
    n.toLocaleString("zh-CN", {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    })
  );
}
