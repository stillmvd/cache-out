pub mod browsers;
pub mod chromium;
pub mod model;
pub mod procs;
pub mod site;
pub mod snapshot;
pub mod vss;

use model::{Browser, Family, ProfileScan};

#[tauri::command]
fn list_browsers() -> Vec<Browser> {
    browsers::detect()
}

#[tauri::command]
async fn running_processes() -> Vec<String> {
    tauri::async_runtime::spawn_blocking(procs::running).await.unwrap_or_default()
}

pub fn scan_profile_blocking(browser_id: &str, profile_id: &str) -> Result<ProfileScan, String> {
    let (browser, profile) = browsers::find(browser_id, profile_id).ok_or("Профиль не найден")?;
    match browser.family {
        Family::Chromium => chromium::scan(&profile.path).map_err(|e| e.to_string()),
        Family::Firefox => Err("Firefox пока не поддерживается".into()),
    }
}

#[tauri::command]
async fn scan_profile(browser_id: String, profile_id: String) -> Result<ProfileScan, String> {
    tauri::async_runtime::spawn_blocking(move || scan_profile_blocking(&browser_id, &profile_id))
        .await
        .map_err(|e| e.to_string())?
}

pub fn run() {
    snapshot::sweep_stale();
    vss::sweep_stale();
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![list_browsers, scan_profile, running_processes])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
