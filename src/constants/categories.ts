// 常用分类预设：覆盖小微 / 自由职业者的高频记账场景
// 记一笔时可直接点选，也保留输入框支持完全自定义的分类
export const EXPENSE_CATEGORIES = [
  "餐饮",
  "交通",
  "差旅",
  "住宿",
  "办公用品",
  "通讯网络",
  "快递物流",
  "招待应酬",
  "会议培训",
  "设备采购",
  "软件订阅",
  "房租水电",
  "维修保养",
  "医疗",
  "其他",
];

export const INCOME_CATEGORIES = [
  "报销回款",
  "工资薪酬",
  "项目奖金",
  "兼职收入",
  "利息理财",
  "退款",
  "其他",
];

/** 按类型返回对应预设分类 */
export function categoriesFor(type: string | null | undefined): string[] {
  return type === "income" ? INCOME_CATEGORIES : EXPENSE_CATEGORIES;
}
