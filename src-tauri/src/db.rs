use rusqlite::{Connection, Result};
use tauri::Manager;

/// 数据库文件路径：<AppData>/ledger.db
pub fn db_path(app: &tauri::AppHandle) -> std::path::PathBuf {
    let dir = app
        .path()
        .app_data_dir()
        .expect("无法获取 AppData 目录");
    std::fs::create_dir_all(&dir).ok();
    dir.join("ledger.db")
}

pub fn connect(app: &tauri::AppHandle) -> Result<Connection, Box<dyn std::error::Error>> {
    Ok(Connection::open(db_path(app))?)
}

/// 凭证图片存储目录：<AppData>/receipts/
pub fn receipts_dir(app: &tauri::AppHandle) -> std::path::PathBuf {
    let dir = app
        .path()
        .app_data_dir()
        .expect("无法获取 AppData 目录")
        .join("receipts");
    std::fs::create_dir_all(&dir).ok();
    dir
}

/// 初始化表结构 + 默认台账
pub fn init_db(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let conn = connect(app)?;
    receipts_dir(app); // 确保凭证目录存在
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS ledgers (
            id      INTEGER PRIMARY KEY AUTOINCREMENT,
            name    TEXT NOT NULL,
            budget  REAL,
            note    TEXT
        );
        CREATE TABLE IF NOT EXISTS categories (
            id       INTEGER PRIMARY KEY AUTOINCREMENT,
            name     TEXT NOT NULL,
            type     TEXT NOT NULL,
            parent_id INTEGER
        );
        CREATE TABLE IF NOT EXISTS entries (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            ledger_id   INTEGER NOT NULL DEFAULT 1,
            date        TEXT NOT NULL,
            type        TEXT NOT NULL,
            category    TEXT NOT NULL,
            amount      REAL NOT NULL,
            party       TEXT,
            payment     TEXT,
            note        TEXT,
            receipt_ref TEXT,
            created_at  TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS receipts (
            id        INTEGER PRIMARY KEY AUTOINCREMENT,
            entry_id  INTEGER NOT NULL,
            path      TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS app_settings (
            key   TEXT PRIMARY KEY,
            value TEXT
        );
        CREATE TABLE IF NOT EXISTS category_budgets (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            ledger_id  INTEGER NOT NULL,
            category   TEXT NOT NULL,
            amount     REAL NOT NULL,
            UNIQUE(ledger_id, category)
        );",
    )?;

    // 平滑升级：老库没有后加的列，按需补上（CREATE TABLE IF NOT EXISTS 不会加列）
    if !has_column(&conn, "entries", "status")? {
        conn.execute(
            "ALTER TABLE entries ADD COLUMN status TEXT NOT NULL DEFAULT 'none'",
            [],
        )?;
        // 老数据迁移：升级前的支出一律视作「待报销」，
        // 否则看板的在途金额永远是 0，用户会以为功能没生效；不需要的可自行改为「不涉及」
        conn.execute("UPDATE entries SET status='pending' WHERE type='expense'", [])?;
    }
    if !has_column(&conn, "entries", "reimbursed_at")? {
        conn.execute("ALTER TABLE entries ADD COLUMN reimbursed_at TEXT", [])?;
    }

    conn.execute(
        "INSERT OR IGNORE INTO ledgers (id, name, budget, note) VALUES (1, '默认台账', NULL, '初始台账')",
        [],
    )?;
    Ok(())
}

/// 判断表里是否已存在某列（用于老版本数据库平滑升级）
fn has_column(conn: &Connection, table: &str, column: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table))?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(1))?;
    for name in rows {
        if name? == column {
            return Ok(true);
        }
    }
    Ok(false)
}
