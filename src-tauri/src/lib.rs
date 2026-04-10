mod scanner;
mod types;

use types::{ScanConfig, ScanResult};

#[tauri::command]
async fn scan_ports(app: tauri::AppHandle, config: ScanConfig) -> Result<ScanResult, String> {
    scanner::run_scan(app, config).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![scan_ports])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
