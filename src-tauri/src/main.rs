#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod db;
mod crypto;
mod commands;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // 首次启动初始化本地 SQLite（含默认台账、凭证表、设置表）
            db::init_db(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::add_entry,
            commands::update_entry,
            commands::delete_entry,
            commands::query_entries,
            commands::entries_summary,
            commands::stats_breakdown,
            commands::list_categories,
            commands::set_entry_status,
            commands::list_category_budgets,
            commands::set_category_budget,
            commands::delete_category_budget,
            commands::ledger_budget,
            commands::list_ledgers,
            commands::create_ledger,
            commands::update_ledger,
            commands::delete_ledger,
            commands::import_receipt,
            commands::list_receipts,
            commands::delete_receipt,
            commands::set_vault_password,
            commands::has_vault_password,
            commands::verify_vault_password,
            commands::clear_vault_password,
            commands::read_file_base64,
            commands::export_excel,
            commands::export_csv,
            commands::export_excel_template,
            commands::import_excel,
            commands::backup_db,
            commands::restore_db,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
