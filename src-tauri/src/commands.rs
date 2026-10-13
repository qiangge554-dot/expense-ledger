use crate::db;
use crate::crypto;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use tauri::AppHandle;

// ============ 通用数据结构 ============

#[derive(Serialize, Clone)]
pub struct EntryRow {
    pub id: i64,
    pub ledger_id: i64,
    pub date: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub category: String,
    pub amount: f64,
    pub party: Option<String>,
    pub payment: Option<String>,
    pub note: Option<String>,
    pub receipt_ref: Option<String>,
    /// 关联凭证张数（导出时用于「凭证」列）
    pub receipt_count: i64,
    /// 报销状态：none=不涉及 / pending=待报销 / submitted=已提交 / reimbursed=已回款
    pub status: String,
    /// 回款日期（status = reimbursed 时有效）
    pub reimbursed_at: Option<String>,
    pub created_at: String,
}

/// 分类预算（按账套 + 分类设置额度）
#[derive(Serialize, Clone)]
pub struct CategoryBudget {
    pub category: String,
    pub amount: f64,
}

/// 报销状态常量与归一化（避免各处硬编码字符串）
pub const STATUS_NONE: &str = "none";
pub const STATUS_PENDING: &str = "pending";
pub const STATUS_SUBMITTED: &str = "submitted";
pub const STATUS_REIMBURSED: &str = "reimbursed";

/// 处于「钱还没回来」的状态：待报销 + 已提交
fn is_in_transit(status: &str) -> bool {
    status == STATUS_PENDING || status == STATUS_SUBMITTED
}

fn normalize_status(s: &str) -> String {
    match s.trim() {
        "待报销" | "pending" | "未报销" => STATUS_PENDING.to_string(),
        "已提交" | "submitted" | "审批中" => STATUS_SUBMITTED.to_string(),
        "已回款" | "已报销" | "reimbursed" | "已到账" => STATUS_REIMBURSED.to_string(),
        _ => STATUS_NONE.to_string(),
    }
}

/// 分类金额聚合项（统计看板「分类支出占比」用）
#[derive(Serialize, Clone)]
pub struct CategoryAmount {
    pub category: String,
    pub amount: f64,
}

/// 统计看板一次调用所需的全部聚合数据（均按当前筛选条件在全量数据上计算）
#[derive(Serialize)]
pub struct StatsBreakdown {
    pub total_expense: f64,
    pub total_income: f64,
    pub net: f64,
    pub count: i64,
    /// 在途（待报销 + 已提交）金额与笔数：自己垫付、公司还没回款的部分
    pub pending_amount: f64,
    pub pending_count: i64,
    pub budget: Option<f64>,
    pub by_category: Vec<CategoryAmount>,
    pub by_month: Vec<MonthlyPoint>,
    pub by_payment: Vec<PaymentShare>,
}

#[derive(Serialize)]
pub struct EntriesSummary {
    pub total_expense: f64,
    pub total_income: f64,
    pub net: f64,
    pub count: i64,
}

#[derive(Serialize)]
pub struct MonthlyPoint {
    pub month: String,
    pub expense: f64,
    pub income: f64,
}

#[derive(Serialize)]
pub struct PaymentShare {
    pub payment: String,
    pub amount: f64,
    pub count: i64,
}

#[derive(Serialize)]
pub struct Ledger {
    pub id: i64,
    pub name: String,
    pub budget: Option<f64>,
    pub note: Option<String>,
}

#[derive(Serialize)]
pub struct ReceiptInfo {
    pub id: i64,
    pub path: String,
}

/// 台账筛选条件（前端传入，缺失字段按默认处理）
#[derive(Deserialize)]
#[serde(default)]
pub struct Filters {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub type_filter: Option<String>, // "expense" | "income" | null
    pub category: Option<String>,
    pub payment: Option<String>,
    /// 报销状态筛选：none / pending / submitted / reimbursed
    pub status: Option<String>,
    pub keyword: Option<String>,
    pub sort: String, // "date_desc" | "date_asc" | "amount_desc"
    pub offset: i64,
    pub limit: i64,
}

impl Default for Filters {
    fn default() -> Self {
        Filters {
            date_from: None,
            date_to: None,
            type_filter: None,
            category: None,
            payment: None,
            status: None,
            keyword: None,
            sort: "date_desc".into(),
            offset: 0,
            limit: 200,
        }
    }
}

// ============ 内部工具 ============

fn now_secs() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string()
}

fn load_entries(app: &AppHandle, ledger_id: i64) -> Result<Vec<EntryRow>, String> {
    let conn = db::connect(app).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT e.id, e.ledger_id, e.date, e.type, e.category, e.amount, e.party, e.payment, \
                    e.note, e.receipt_ref, \
                    (SELECT COUNT(*) FROM receipts r WHERE r.entry_id = e.id), \
                    COALESCE(e.status, 'none'), e.reimbursed_at, e.created_at \
             FROM entries e WHERE e.ledger_id=?1",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([ledger_id], |r| {
            Ok(EntryRow {
                id: r.get(0)?,
                ledger_id: r.get(1)?,
                date: r.get(2)?,
                type_: r.get(3)?,
                category: r.get(4)?,
                amount: r.get(5)?,
                party: r.get(6)?,
                payment: r.get(7)?,
                note: r.get(8)?,
                receipt_ref: r.get(9)?,
                receipt_count: r.get(10)?,
                status: r.get(11)?,
                reimbursed_at: r.get(12)?,
                created_at: r.get(13)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

/// 在 Rust 侧完成筛选 + 排序（数据集小，性能无忧）
fn filter_entries(mut rows: Vec<EntryRow>, f: &Filters) -> Vec<EntryRow> {
    if let Some(d) = &f.date_from {
        rows.retain(|r| &r.date >= d);
    }
    if let Some(d) = &f.date_to {
        rows.retain(|r| &r.date <= d);
    }
    if let Some(t) = &f.type_filter {
        rows.retain(|r| &r.type_ == t);
    }
    if let Some(c) = &f.category {
        rows.retain(|r| &r.category == c);
    }
    if let Some(p) = &f.payment {
        rows.retain(|r| r.payment.as_deref() == Some(p.as_str()));
    }
    if let Some(s) = &f.status {
        rows.retain(|r| &r.status == s);
    }
    if let Some(kw) = &f.keyword {
        let k = kw.to_lowercase();
        rows.retain(|r| {
            r.category.to_lowercase().contains(&k)
                || r.party.as_deref().map_or(false, |p| p.to_lowercase().contains(&k))
                || r.note.as_deref().map_or(false, |n| n.to_lowercase().contains(&k))
        });
    }
    match f.sort.as_str() {
        "date_asc" => rows.sort_by(|a, b| a.date.cmp(&b.date).then(a.id.cmp(&b.id))),
        "amount_desc" => rows.sort_by(|a, b| {
            b.amount
                .partial_cmp(&a.amount)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(b.id.cmp(&a.id))
        }),
        _ => rows.sort_by(|a, b| b.date.cmp(&a.date).then(b.id.cmp(&a.id))),
    }
    rows
}

// ============ 记账相关 ============

#[tauri::command]
pub fn add_entry(app: AppHandle, payload: serde_json::Value) -> Result<i64, String> {
    let conn = db::connect(&app).map_err(|e| e.to_string())?;
    let date = payload.get("date").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let etype = payload
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("expense")
        .to_string();
    let category = payload
        .get("category")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let amount = payload.get("amount").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let party = payload.get("party").and_then(|v| v.as_str()).map(|s| s.to_string());
    let payment = payload.get("payment").and_then(|v| v.as_str()).map(|s| s.to_string());
    let note = payload.get("note").and_then(|v| v.as_str()).map(|s| s.to_string());
    let ledger_id = payload.get("ledger_id").and_then(|v| v.as_i64()).unwrap_or(1);
    // 支出默认记为「待报销」（报销台账的核心诉求），收入不涉及
    let status = match payload.get("status").and_then(|v| v.as_str()) {
        Some(s) => normalize_status(s),
        None if etype == "expense" => STATUS_PENDING.to_string(),
        None => STATUS_NONE.to_string(),
    };
    let reimbursed_at = payload
        .get("reimbursed_at")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let created = now_secs();

    let id: i64 = conn
        .query_row(
            "INSERT INTO entries (ledger_id, date, type, category, amount, party, payment, note, receipt_ref, created_at, status, reimbursed_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8, NULL, ?9, ?10, ?11) RETURNING id",
            rusqlite::params![
                ledger_id, date, etype, category, amount, party, payment, note, created,
                status, reimbursed_at
            ],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(id)
}

/// 单独更新报销状态（列表页一键流转用）
#[tauri::command]
pub fn set_entry_status(
    app: AppHandle,
    id: i64,
    status: String,
    reimbursed_at: Option<String>,
) -> Result<(), String> {
    let conn = db::connect(&app).map_err(|e| e.to_string())?;
    let st = normalize_status(&status);
    // 回款日期：已回款时若未传则记为今天，其它状态清空
    let paid = if st == STATUS_REIMBURSED {
        match reimbursed_at {
            Some(s) if !s.trim().is_empty() => Some(s),
            _ => Some(today_str()),
        }
    } else {
        None
    };
    conn.execute(
        "UPDATE entries SET status=?1, reimbursed_at=?2 WHERE id=?3",
        rusqlite::params![st, paid, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// 今天（YYYY-MM-DD），用系统秒数换算，避免引入 chrono
fn today_str() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let days = secs / 86_400;
    let (y, m, d) = days_to_ymd(days);
    format!("{:04}-{:02}-{:02}", y, m, d)
}

#[tauri::command]
pub fn update_entry(app: AppHandle, id: i64, payload: serde_json::Value) -> Result<(), String> {
    let conn = db::connect(&app).map_err(|e| e.to_string())?;
    if let Some(v) = payload.get("date").and_then(|v| v.as_str()) {
        conn.execute("UPDATE entries SET date=?1 WHERE id=?2", rusqlite::params![v, id])
            .map_err(|e| e.to_string())?;
    }
    if let Some(v) = payload.get("type").and_then(|v| v.as_str()) {
        conn.execute("UPDATE entries SET type=?1 WHERE id=?2", rusqlite::params![v, id])
            .map_err(|e| e.to_string())?;
    }
    if let Some(v) = payload.get("category").and_then(|v| v.as_str()) {
        conn.execute("UPDATE entries SET category=?1 WHERE id=?2", rusqlite::params![v, id])
            .map_err(|e| e.to_string())?;
    }
    if let Some(v) = payload.get("amount").and_then(|v| v.as_f64()) {
        conn.execute("UPDATE entries SET amount=?1 WHERE id=?2", rusqlite::params![v, id])
            .map_err(|e| e.to_string())?;
    }
    if let Some(v) = payload.get("party").and_then(|v| v.as_str()) {
        conn.execute("UPDATE entries SET party=?1 WHERE id=?2", rusqlite::params![v, id])
            .map_err(|e| e.to_string())?;
    }
    if let Some(v) = payload.get("payment").and_then(|v| v.as_str()) {
        conn.execute("UPDATE entries SET payment=?1 WHERE id=?2", rusqlite::params![v, id])
            .map_err(|e| e.to_string())?;
    }
    if let Some(v) = payload.get("note").and_then(|v| v.as_str()) {
        conn.execute("UPDATE entries SET note=?1 WHERE id=?2", rusqlite::params![v, id])
            .map_err(|e| e.to_string())?;
    }
    if let Some(v) = payload.get("status").and_then(|v| v.as_str()) {
        let st = normalize_status(v);
        let paid: Option<String> = if st == STATUS_REIMBURSED {
            match payload.get("reimbursed_at").and_then(|x| x.as_str()) {
                Some(s) if !s.trim().is_empty() => Some(s.to_string()),
                _ => Some(today_str()),
            }
        } else {
            None
        };
        conn.execute(
            "UPDATE entries SET status=?1, reimbursed_at=?2 WHERE id=?3",
            rusqlite::params![st, paid, id],
        )
        .map_err(|e| e.to_string())?;
    } else if let Some(v) = payload.get("reimbursed_at").and_then(|v| v.as_str()) {
        conn.execute(
            "UPDATE entries SET reimbursed_at=?1 WHERE id=?2",
            rusqlite::params![v, id],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn delete_entry(app: AppHandle, id: i64) -> Result<(), String> {
    let conn = db::connect(&app).map_err(|e| e.to_string())?;
    let paths: Vec<String> = {
        let mut st = conn
            .prepare("SELECT path FROM receipts WHERE entry_id=?1")
            .map_err(|e| e.to_string())?;
        let rows = st
            .query_map([id], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect()
    };
    for p in &paths {
        std::fs::remove_file(p).ok();
    }
    conn.execute("DELETE FROM receipts WHERE entry_id=?1", [id])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM entries WHERE id=?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ============ 列表 / 统计 ============

#[tauri::command]
pub fn query_entries(
    app: AppHandle,
    ledger_id: i64,
    filters: Filters,
) -> Result<Vec<EntryRow>, String> {
    let rows = load_entries(&app, ledger_id)?;
    let filtered = filter_entries(rows, &filters);
    let page: Vec<EntryRow> = filtered
        .into_iter()
        .skip(filters.offset as usize)
        .take(filters.limit as usize)
        .collect();
    Ok(page)
}

#[tauri::command]
pub fn entries_summary(
    app: AppHandle,
    ledger_id: i64,
    filters: Filters,
) -> Result<EntriesSummary, String> {
    let rows = load_entries(&app, ledger_id)?;
    let filtered = filter_entries(rows, &filters);
    let total_expense = filtered
        .iter()
        .filter(|r| r.type_ == "expense")
        .map(|r| r.amount)
        .sum();
    let total_income = filtered
        .iter()
        .filter(|r| r.type_ == "income")
        .map(|r| r.amount)
        .sum();
    Ok(EntriesSummary {
        total_expense,
        total_income,
        net: total_income - total_expense,
        count: filtered.len() as i64,
    })
}

/// 统计看板聚合：一次返回「汇总 + 分类 + 月度 + 支付方式 + 预算」。
/// 全部基于**筛选后的全量数据**计算——此前由前端按当前分页（100 条）派生，
/// 记录超过一页时看板数字会明显偏小。
#[tauri::command]
pub fn stats_breakdown(
    app: AppHandle,
    ledger_id: i64,
    filters: Filters,
) -> Result<StatsBreakdown, String> {
    let rows = load_entries(&app, ledger_id)?;
    let filtered = filter_entries(rows, &filters);

    let mut total_expense = 0.0f64;
    let mut total_income = 0.0f64;
    let mut pending_amount = 0.0f64;
    let mut pending_count = 0i64;
    let mut cat: BTreeMap<String, f64> = BTreeMap::new();
    let mut month: BTreeMap<String, (f64, f64)> = BTreeMap::new();
    let mut pay: HashMap<String, (f64, i64)> = HashMap::new();

    for r in &filtered {
        if r.type_ == "expense" {
            total_expense += r.amount;
        } else {
            total_income += r.amount;
        }
        if let Some(m) = r.date.get(0..7) {
            let e = month.entry(m.to_string()).or_insert((0.0, 0.0));
            if r.type_ == "expense" {
                e.0 += r.amount;
            } else {
                e.1 += r.amount;
            }
        }
        if r.type_ == "expense" {
            if is_in_transit(&r.status) {
                pending_amount += r.amount;
                pending_count += 1;
            }
            *cat.entry(r.category.clone()).or_insert(0.0) += r.amount;
            let k = r.payment.clone().unwrap_or_else(|| "未填".into());
            let e = pay.entry(k).or_insert((0.0, 0));
            e.0 += r.amount;
            e.1 += 1;
        }
    }

    let mut by_category: Vec<CategoryAmount> = cat
        .into_iter()
        .map(|(category, amount)| CategoryAmount { category, amount })
        .collect();
    by_category.sort_by(|a, b| {
        b.amount
            .partial_cmp(&a.amount)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut by_payment: Vec<PaymentShare> = pay
        .into_iter()
        .map(|(payment, (amount, count))| PaymentShare {
            payment,
            amount,
            count,
        })
        .collect();
    by_payment.sort_by(|a, b| {
        b.amount
            .partial_cmp(&a.amount)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let by_month = month
        .into_iter()
        .map(|(month, (expense, income))| MonthlyPoint {
            month,
            expense,
            income,
        })
        .collect();

    let budget: Option<f64> = {
        let conn = db::connect(&app).map_err(|e| e.to_string())?;
        conn.query_row("SELECT budget FROM ledgers WHERE id=?1", [ledger_id], |r| {
            r.get(0)
        })
        .ok()
    };

    Ok(StatsBreakdown {
        total_expense,
        total_income,
        net: total_income - total_expense,
        count: filtered.len() as i64,
        pending_amount,
        pending_count,
        budget,
        by_category,
        by_month,
        by_payment,
    })
}

/// 读取分类预算
#[tauri::command]
pub fn list_category_budgets(
    app: AppHandle,
    ledger_id: i64,
) -> Result<Vec<CategoryBudget>, String> {
    let conn = db::connect(&app).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT category, amount FROM category_budgets WHERE ledger_id=?1 ORDER BY category",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([ledger_id], |r| {
            Ok(CategoryBudget {
                category: r.get(0)?,
                amount: r.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

/// 设置分类预算；amount <= 0 视为删除该分类的预算
#[tauri::command]
pub fn set_category_budget(
    app: AppHandle,
    ledger_id: i64,
    category: String,
    amount: f64,
) -> Result<(), String> {
    let cat = category.trim();
    if cat.is_empty() {
        return Err("分类不能为空".into());
    }
    let conn = db::connect(&app).map_err(|e| e.to_string())?;
    if amount <= 0.0 {
        conn.execute(
            "DELETE FROM category_budgets WHERE ledger_id=?1 AND category=?2",
            rusqlite::params![ledger_id, cat],
        )
        .map_err(|e| e.to_string())?;
        return Ok(());
    }
    conn.execute(
        "INSERT INTO category_budgets (ledger_id, category, amount) VALUES (?1,?2,?3) \
         ON CONFLICT(ledger_id, category) DO UPDATE SET amount=?3",
        rusqlite::params![ledger_id, cat, amount],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// 删除某分类的预算
#[tauri::command]
pub fn delete_category_budget(
    app: AppHandle,
    ledger_id: i64,
    category: String,
) -> Result<(), String> {
    let conn = db::connect(&app).map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM category_budgets WHERE ledger_id=?1 AND category=?2",
        rusqlite::params![ledger_id, category.trim()],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// 账套内已使用过的全部分类（供录入页自动补全与筛选下拉使用，不受分页限制）
#[tauri::command]
pub fn list_categories(app: AppHandle, ledger_id: i64) -> Result<Vec<String>, String> {
    let conn = db::connect(&app).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT category FROM entries WHERE ledger_id=?1 AND TRIM(category)<>''",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([ledger_id], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

#[tauri::command]
pub fn ledger_budget(app: AppHandle, ledger_id: i64) -> Result<Option<f64>, String> {
    let conn = db::connect(&app).map_err(|e| e.to_string())?;
    let b: Option<f64> = conn
        .query_row("SELECT budget FROM ledgers WHERE id=?1", [ledger_id], |r| r.get(0))
        .ok();
    Ok(b)
}

// ============ 账套管理 ============

#[tauri::command]
pub fn list_ledgers(app: AppHandle) -> Result<Vec<Ledger>, String> {
    let conn = db::connect(&app).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, name, budget, note FROM ledgers ORDER BY id")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(Ledger {
                id: r.get(0)?,
                name: r.get(1)?,
                budget: r.get(2)?,
                note: r.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

#[tauri::command]
pub fn create_ledger(
    app: AppHandle,
    name: String,
    budget: Option<f64>,
    note: Option<String>,
) -> Result<i64, String> {
    let conn = db::connect(&app).map_err(|e| e.to_string())?;
    let id: i64 = conn
        .query_row(
            "INSERT INTO ledgers (name, budget, note) VALUES (?1,?2,?3) RETURNING id",
            rusqlite::params![name, budget, note],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub fn update_ledger(
    app: AppHandle,
    id: i64,
    name: Option<String>,
    budget: Option<f64>,
    note: Option<String>,
) -> Result<(), String> {
    let conn = db::connect(&app).map_err(|e| e.to_string())?;
    if let Some(n) = name {
        conn.execute("UPDATE ledgers SET name=?1 WHERE id=?2", rusqlite::params![n, id])
            .map_err(|e| e.to_string())?;
    }
    if let Some(b) = budget {
        conn.execute("UPDATE ledgers SET budget=?1 WHERE id=?2", rusqlite::params![b, id])
            .map_err(|e| e.to_string())?;
    }
    if let Some(n) = note {
        conn.execute("UPDATE ledgers SET note=?1 WHERE id=?2", rusqlite::params![n, id])
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn delete_ledger(app: AppHandle, id: i64) -> Result<(), String> {
    if id == 1 {
        return Err("默认台账不可删除".into());
    }
    let conn = db::connect(&app).map_err(|e| e.to_string())?;
    // 先取该账套下所有凭证文件路径，删记录前把图片一起清掉（否则残留孤儿文件）
    let files: Vec<String> = {
        let mut st = conn
            .prepare(
                "SELECT r.path FROM receipts r \
                 JOIN entries e ON r.entry_id = e.id WHERE e.ledger_id=?1",
            )
            .map_err(|e| e.to_string())?;
        let rows = st
            .query_map([id], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect()
    };
    conn.execute(
        "DELETE FROM receipts WHERE entry_id IN (SELECT id FROM entries WHERE ledger_id=?1)",
        [id],
    )
    .map_err(|e| e.to_string())?;
    for p in &files {
        std::fs::remove_file(p).ok();
    }
    conn.execute("DELETE FROM entries WHERE ledger_id=?1", [id])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM category_budgets WHERE ledger_id=?1", [id])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM ledgers WHERE id=?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ============ 凭证 ============

#[tauri::command]
pub fn import_receipt(app: AppHandle, entry_id: i64, src_path: String) -> Result<String, String> {
    let src = std::path::Path::new(&src_path);
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("bin")
        .to_lowercase();
    let stamp = now_secs();
    let rnd: u32 = rand::random();
    let fname = format!("{}_{}.{}", stamp, rnd, ext);
    let dest = db::receipts_dir(&app).join(&fname);
    std::fs::copy(src, &dest).map_err(|e| e.to_string())?;
    let conn = db::connect(&app).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO receipts (entry_id, path, created_at) VALUES (?1,?2,?3)",
        rusqlite::params![entry_id, dest.to_string_lossy().to_string(), now_secs()],
    )
    .map_err(|e| e.to_string())?;
    Ok(dest.to_string_lossy().to_string())
}

#[tauri::command]
pub fn list_receipts(app: AppHandle, entry_id: i64) -> Result<Vec<ReceiptInfo>, String> {
    let conn = db::connect(&app).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, path FROM receipts WHERE entry_id=?1 ORDER BY id")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([entry_id], |r| {
            Ok(ReceiptInfo {
                id: r.get(0)?,
                path: r.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

#[tauri::command]
pub fn delete_receipt(app: AppHandle, id: i64) -> Result<(), String> {
    let conn = db::connect(&app).map_err(|e| e.to_string())?;
    let path: Option<String> = conn
        .query_row("SELECT path FROM receipts WHERE id=?1", [id], |r| r.get(0))
        .ok();
    conn.execute("DELETE FROM receipts WHERE id=?1", [id])
        .map_err(|e| e.to_string())?;
    if let Some(p) = path {
        std::fs::remove_file(p).ok();
    }
    Ok(())
}

// ============ 口令锁（vault） ============

#[tauri::command]
pub fn set_vault_password(app: AppHandle, password: String) -> Result<(), String> {
    let hash = crypto::hash_password(&password);
    let conn = db::connect(&app).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO app_settings (key, value) VALUES ('vault_hash', ?1) \
         ON CONFLICT(key) DO UPDATE SET value=?1",
        [hash],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn has_vault_password(app: AppHandle) -> Result<bool, String> {
    let conn = db::connect(&app).map_err(|e| e.to_string())?;
    let c: i64 = conn
        .query_row("SELECT COUNT(*) FROM app_settings WHERE key='vault_hash'", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    Ok(c > 0)
}

#[tauri::command]
pub fn verify_vault_password(app: AppHandle, password: String) -> Result<bool, String> {
    let conn = db::connect(&app).map_err(|e| e.to_string())?;
    let hash: Option<String> = conn
        .query_row("SELECT value FROM app_settings WHERE key='vault_hash'", [], |r| r.get(0))
        .ok();
    match hash {
        Some(h) => Ok(crypto::verify_password(&password, &h)),
        None => Ok(false),
    }
}

#[tauri::command]
pub fn clear_vault_password(app: AppHandle) -> Result<(), String> {
    let conn = db::connect(&app).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM app_settings WHERE key='vault_hash'", [])
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 读取本地文件并返回 data URL，供前端 <img> 预览凭证
#[tauri::command]
pub fn read_file_base64(app: AppHandle, path: String) -> Result<String, String> {
    let _ = &app; // 仅需路径，保留 app 入参以便权限上下文
    let data = std::fs::read(&path).map_err(|e| e.to_string())?;
    let lower = path.to_lowercase();
    let mime = if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else if lower.ends_with(".gif") {
        "image/gif"
    } else {
        "application/octet-stream"
    };
    Ok(format!(
        "data:{};base64,{}",
        mime,
        base64::engine::general_purpose::STANDARD.encode(data)
    ))
}

// ============ 导出 / 备份 ============

/// 报销状态 → 中文标签（导出与打印统一口径）
fn status_label(s: &str) -> &'static str {
    match s {
        STATUS_PENDING => "待报销",
        STATUS_SUBMITTED => "已提交",
        STATUS_REIMBURSED => "已回款",
        _ => "不涉及",
    }
}

/// 导出的「凭证」列：显示凭证张数。
/// 注意：凭证图片存在 receipts 表，entries.receipt_ref 恒为空，
/// 早期版本直接导出 receipt_ref 导致该列永远空白。
fn receipt_cell(count: i64) -> String {
    if count > 0 {
        format!("{} 张", count)
    } else {
        String::new()
    }
}

fn csv_field(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[tauri::command]
pub fn export_csv(
    app: AppHandle,
    ledger_id: i64,
    path: String,
    filters: Filters,
) -> Result<(), String> {
    let rows = load_entries(&app, ledger_id)?;
    let rows = filter_entries(rows, &filters);
    let mut f = std::fs::File::create(&path).map_err(|e| e.to_string())?;
    use std::io::Write;
    f.write_all(b"\xef\xbb\xbf").map_err(|e| e.to_string())?; // UTF-8 BOM，Excel 中文不乱码
    writeln!(f, "日期,类型,分类,金额,往来方,支付方式,备注,凭证,报销状态,回款日期")
        .map_err(|e| e.to_string())?;
    for r in &rows {
        let ty = if r.type_ == "expense" { "支出" } else { "收入" };
        let line = format!(
            "{},{},{},{},{},{},{},{},{},{}\n",
            r.date,
            ty,
            csv_field(&r.category),
            r.amount,
            csv_field(r.party.as_deref().unwrap_or("")),
            csv_field(r.payment.as_deref().unwrap_or("")),
            csv_field(r.note.as_deref().unwrap_or("")),
            csv_field(&receipt_cell(r.receipt_count)),
            status_label(&r.status),
            r.reimbursed_at.as_deref().unwrap_or(""),
        );
        f.write_all(line.as_bytes()).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn export_excel(
    app: AppHandle,
    ledger_id: i64,
    path: String,
    filters: Filters,
) -> Result<(), String> {
    let rows = load_entries(&app, ledger_id)?;
    let rows = filter_entries(rows, &filters);

    let mut wb = rust_xlsxwriter::Workbook::new(&path);
    let ws = wb.add_worksheet();
    ws.set_name("台账").map_err(|e| e.to_string())?;
    let fmt = rust_xlsxwriter::Format::default();

    let headers = [
        "日期", "类型", "分类", "金额", "往来方", "支付方式", "备注", "凭证", "报销状态", "回款日期",
    ];
    for (c, h) in headers.iter().enumerate() {
        ws.write_string(0, c as u16, h, &fmt).map_err(|e| e.to_string())?;
    }
    for (i, r) in rows.iter().enumerate() {
        let row = (i + 1) as u32;
        let ty = if r.type_ == "expense" { "支出" } else { "收入" };
        ws.write_string(row, 0, &r.date, &fmt).map_err(|e| e.to_string())?;
        ws.write_string(row, 1, ty, &fmt).map_err(|e| e.to_string())?;
        ws.write_string(row, 2, &r.category, &fmt).map_err(|e| e.to_string())?;
        ws.write_number(row, 3, r.amount, &fmt).map_err(|e| e.to_string())?;
        ws.write_string(row, 4, r.party.as_deref().unwrap_or(""), &fmt)
            .map_err(|e| e.to_string())?;
        ws.write_string(row, 5, r.payment.as_deref().unwrap_or(""), &fmt)
            .map_err(|e| e.to_string())?;
        ws.write_string(row, 6, r.note.as_deref().unwrap_or(""), &fmt)
            .map_err(|e| e.to_string())?;
        ws.write_string(row, 7, &receipt_cell(r.receipt_count), &fmt)
            .map_err(|e| e.to_string())?;
        ws.write_string(row, 8, status_label(&r.status), &fmt)
            .map_err(|e| e.to_string())?;
        ws.write_string(row, 9, r.reimbursed_at.as_deref().unwrap_or(""), &fmt)
            .map_err(|e| e.to_string())?;
    }
    wb.close().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn backup_db(app: AppHandle, path: String) -> Result<(), String> {
    let src = db::db_path(&app);
    std::fs::copy(&src, &path).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn restore_db(app: AppHandle, path: String) -> Result<(), String> {
    let dest = db::db_path(&app);
    std::fs::copy(&path, &dest).map_err(|e| e.to_string())?;
    Ok(())
}

// ============ Excel 模板导入 ============

/// 导入模板表头（列顺序固定，与「导出 Excel」一致）
const TEMPLATE_HEADERS: [&str; 8] = [
    "日期", "类型", "分类", "金额", "往来方", "支付方式", "备注", "报销状态",
];

#[derive(Serialize)]
pub struct ImportResult {
    pub inserted: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

/// 下载空白导入模板（含表头 + 示例行 + 填写说明表）
#[tauri::command]
pub fn export_excel_template(path: String) -> Result<(), String> {
    let mut wb = rust_xlsxwriter::Workbook::new(&path);
    let ws = wb.add_worksheet();
    ws.set_name("台账数据").map_err(|e| e.to_string())?;
    let head_fmt = rust_xlsxwriter::Format::new().set_bold();
    for (c, h) in TEMPLATE_HEADERS.iter().enumerate() {
        ws.write_string(0, c as u16, h, &head_fmt)
            .map_err(|e| e.to_string())?;
    }
    for (c, w) in [16.0, 8.0, 14.0, 12.0, 26.0, 12.0, 30.0, 12.0].iter().enumerate() {
        ws.set_column_width(c as u16, *w).map_err(|e| e.to_string())?;
    }
    let rows: [(&str, &str, &str, f64, &str, &str, &str, &str); 2] = [
        (
            "2026-01-31",
            "支出",
            "差旅",
            358.20,
            "示例：北京出差高铁票",
            "支付宝",
            "示例行，导入前请删除",
            "待报销",
        ),
        (
            "2026-01-31",
            "收入",
            "报销回款",
            1200.00,
            "",
            "银行转账",
            "示例行，导入前请删除",
            "不涉及",
        ),
    ];
    for (i, (d, t, c, a, p, pay, n, st)) in rows.iter().enumerate() {
        let r = (i + 1) as u32;
        ws.write_string(r, 0, *d, &head_fmt).map_err(|e| e.to_string())?;
        ws.write_string(r, 1, *t, &head_fmt).map_err(|e| e.to_string())?;
        ws.write_string(r, 2, *c, &head_fmt).map_err(|e| e.to_string())?;
        ws.write_number(r, 3, *a, &head_fmt).map_err(|e| e.to_string())?;
        ws.write_string(r, 4, *p, &head_fmt).map_err(|e| e.to_string())?;
        ws.write_string(r, 5, *pay, &head_fmt).map_err(|e| e.to_string())?;
        ws.write_string(r, 6, *n, &head_fmt).map_err(|e| e.to_string())?;
        ws.write_string(r, 7, *st, &head_fmt).map_err(|e| e.to_string())?;
    }

    let ws2 = wb.add_worksheet();
    ws2.set_name("填写说明").map_err(|e| e.to_string())?;
    let notes = [
        "1. 「台账数据」表第 1 行为表头，请勿修改列顺序或删除表头。",
        "2. 日期：支持 2026-01-31、2026/1/31、2026.1.31、20260131 或 Excel 日期单元格。",
        "3. 类型：只能填「支出」或「收入」。",
        "4. 分类：必填，建议参考「记一笔」页的常用分类，也可自由填写。",
        "5. 金额：大于 0 的数字，可含千分位逗号。",
        "6. 往来方 / 支付方式 / 备注：选填；支付方式支持 现金/银行转账/支付宝/微信/刷卡。",
        "7. 备注中含「示例行」字样的数据，导入时会被自动跳过。",
        "8. 报销状态：选填，填「待报销 / 已提交 / 已回款 / 不涉及」；留空时支出记为待报销、收入记为不涉及。",
    ];
    for (i, n) in notes.iter().enumerate() {
        ws2.write_string(i as u32, 0, *n, &head_fmt)
            .map_err(|e| e.to_string())?;
    }
    ws2.set_column_width(0, 90.0).map_err(|e| e.to_string())?;

    wb.close().map_err(|e| e.to_string())?;
    Ok(())
}

fn cell_str(d: &calamine::Data) -> String {
    use calamine::Data;
    match d {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        Data::Int(i) => i.to_string(),
        Data::Float(f) => format_number(*f),
        Data::Bool(b) => b.to_string(),
        Data::DateTime(dt) => format_number(dt.as_f64()),
        Data::DateTimeIso(s) => s.clone(),
        Data::DurationIso(s) => s.clone(),
        _ => String::new(),
    }
}

fn format_number(f: f64) -> String {
    if f.fract() == 0.0 && f.abs() < 1e15 {
        format!("{}", f as i64)
    } else {
        format!("{}", f)
    }
}

fn parse_amount(s: &str) -> Option<f64> {
    let t = s.trim().replace(',', "").replace('¥', "").replace('￥', "");
    if t.is_empty() {
        return None;
    }
    t.parse::<f64>().ok()
}

/// 把各种常见日期写法统一为 YYYY-MM-DD
fn normalize_date(s: &str) -> Option<String> {
    // 容错 Excel 常见的带时间写法："2026-01-31T00:00:00" / "2026-01-31 00:00:00"
    let t = s.trim();
    let t = t.split(|c| c == 'T' || c == ' ').next().unwrap_or(t).trim();
    if t.is_empty() {
        return None;
    }
    if t.chars().all(|c| c.is_ascii_digit()) {
        if t.len() == 8 {
            let y: i64 = t[0..4].parse().ok()?;
            let m: u32 = t[4..6].parse().ok()?;
            let d: u32 = t[6..8].parse().ok()?;
            return valid_ymd(y, m, d);
        }
        if let Ok(serial) = t.parse::<f64>() {
            if (1_000.0..200_000.0).contains(&serial) {
                return serial_to_date(serial.floor());
            }
        }
        return None;
    }
    let parts: Vec<&str> = t.split(|c| c == '-' || c == '/' || c == '.').collect();
    if parts.len() == 3 {
        let y: i64 = parts[0].trim().parse().ok()?;
        let m: u32 = parts[1].trim().parse().ok()?;
        let d: u32 = parts[2].trim().parse().ok()?;
        return valid_ymd(y, m, d);
    }
    None
}

fn valid_ymd(y: i64, m: u32, d: u32) -> Option<String> {
    if !(1..=9999).contains(&y) || m == 0 || m > 12 || d == 0 || d > 31 {
        return None;
    }
    Some(format!("{:04}-{:02}-{:02}", y, m, d))
}

/// Excel 1900 日期系统：序列数 60 = 虚构的 1900-02-29（Excel 的闰年 bug），
/// 因此 serial >= 60 时纪元取 1899-12-30，更早的取 1899-12-31
fn serial_to_date(serial: f64) -> Option<String> {
    let days = serial as i64;
    let base = if days >= 60 {
        days_from_civil(1899, 12, 30)
    } else {
        days_from_civil(1899, 12, 31)
    };
    let (y, m, d) = days_to_ymd(base + days);
    valid_ymd(y, m, d)
}

// Howard Hinnant 的civil-date算法，避免引入 chrono 依赖
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as i64;
    let mp = if m > 2 { m - 3 } else { m + 9 } as i64;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    // 719468 = 0000-03-01 纪元到 1970-01-01 的天数偏移（Hinnant 算法约定），
    // 缺失它会把日期整体推后约 1970 年
    era * 146097 + doe - 719468
}

fn days_to_ymd(days: i64) -> (i64, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// 中文支付方式名 → 前端 PaymentMethod key（未识别的原样保留，前端展示时回退原文）
fn normalize_payment(s: &str) -> String {
    match s.trim() {
        "现金" => "cash".into(),
        "银行转账" | "银行" | "转账" | "银行卡" => "bank".into(),
        "支付宝" => "alipay".into(),
        "微信" | "微信支付" => "wechat".into(),
        "刷卡" | "信用卡" => "card".into(),
        other => other.to_string(),
    }
}

/// 生成「第 N 行：...」格式的导入错误信息（&str 与 String 均可）
fn row_err(row: usize, msg: impl Into<String>) -> String {
    format!("第 {} 行：{}", row, msg.into())
}

/// 按模板导入 Excel 台账数据（第一个工作表；跳过表头与示例行）
#[tauri::command]
pub fn import_excel(
    app: AppHandle,
    ledger_id: i64,
    path: String,
) -> Result<ImportResult, String> {
    use calamine::{open_workbook, Reader, Xlsx};

    let mut book: Xlsx<std::io::BufReader<std::fs::File>> =
        open_workbook(&path).map_err(|e| format!("无法打开文件：{}", e))?;
    let sheets = book.sheet_names();
    let first = sheets.first().ok_or("工作簿为空")?.clone();
    let range = book
        .worksheet_range(&first)
        .map_err(|e| format!("读取工作表失败：{}", e))?;

    let mut res = ImportResult {
        inserted: 0,
        skipped: 0,
        errors: Vec::new(),
    };
    // 整批写入放进一个事务：几百行以上时比逐行提交快一个数量级，
    // 且中途失败时不会留下「导入了一半」的脏数据
    let mut conn = db::connect(&app).map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    for (i, row) in range.rows().enumerate() {
        if i == 0 {
            continue; // 表头
        }
        let cells: Vec<String> = row.iter().map(cell_str).collect();
        let joined = cells.join(" ");
        if joined.trim().is_empty() {
            continue; // 整行为空
        }
        if joined.contains("示例行") {
            res.skipped += 1;
            continue; // 模板示例行防护
        }

        let date_s = match normalize_date(cells.first().map(|s| s.as_str()).unwrap_or("")) {
            Some(d) => d,
            None => {
                res.errors
                    .push(row_err(i + 1, "日期无效（支持 2026-01-31 / 2026/1/31 / 20260131 / Excel 日期）"));
                continue;
            }
        };
        let type_ = match cells.get(1).map(|s| s.trim()).unwrap_or("") {
            "支出" | "expense" => "expense",
            "收入" | "income" => "income",
            "" => {
                res.errors.push(row_err(i + 1, "类型为空（应为「支出」或「收入」）"));
                continue;
            }
            other => {
                res.errors
                    .push(row_err(i + 1, format!("类型「{}」无法识别（应为「支出」或「收入」）", other)));
                continue;
            }
        };
        let category = cells.get(2).map(|s| s.trim().to_string()).unwrap_or_default();
        if category.is_empty() {
            res.errors.push(row_err(i + 1, "分类为空"));
            continue;
        }
        let amount = match parse_amount(cells.get(3).map(|s| s.as_str()).unwrap_or("")) {
            Some(a) if a > 0.0 => a,
            Some(_) => {
                res.errors.push(row_err(i + 1, "金额应为大于 0 的数字"));
                continue;
            }
            None => {
                res.errors.push(row_err(i + 1, "金额不是有效数字"));
                continue;
            }
        };
        let party = cells.get(4).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        let payment = cells
            .get(5)
            .map(|s| normalize_payment(s))
            .filter(|s| !s.is_empty());
        let note = cells.get(6).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        // 报销状态（第 8 列，选填）：留空时支出默认「待报销」，收入「不涉及」
        let status_raw = cells.get(7).map(|s| s.trim().to_string()).unwrap_or_default();
        let status = if status_raw.is_empty() {
            if type_ == "expense" {
                STATUS_PENDING.to_string()
            } else {
                STATUS_NONE.to_string()
            }
        } else {
            normalize_status(&status_raw)
        };

        match tx.execute(
            "INSERT INTO entries (ledger_id, date, type, category, amount, party, payment, note, receipt_ref, created_at, status, reimbursed_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,NULL,?9,?10,NULL)",
            rusqlite::params![
                ledger_id, date_s, type_, category, amount, party, payment, note, now_secs(), status
            ],
        ) {
            Ok(_) => res.inserted += 1,
            Err(e) => res.errors.push(row_err(i + 1, format!("写入失败：{}", e))),
        }
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(res)
}
