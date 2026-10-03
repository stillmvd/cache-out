pub mod browsers;
pub mod chromium;
pub mod clean;
pub mod model;
pub mod procs;
pub mod site;
pub mod snapshot;
pub mod vss;

use clean::{CleanError, CleanReport, CleanRequest};
use model::{Browser, Family, ProfileScan};
use std::sync::Mutex;

static CLEANING: Mutex<()> = Mutex::new(());

#[tauri::command]
fn list_browsers() -> Vec<Browser> {
    let mut list = browsers::detect();
    for p in list.iter_mut().flat_map(|b| b.profiles.iter_mut()) {
        p.avatar = p.avatar_src.as_deref().and_then(browsers::avatar_data);
    }
    list
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

pub fn clean_blocking(browser_id: &str, profile_id: &str, request: &CleanRequest, close: bool) -> Result<CleanReport, CleanError> {
    let (browser, profile) = browsers::find(browser_id, profile_id).ok_or_else(|| CleanError::before("Профиль не найден"))?;
    if browser.family != Family::Chromium {
        return Err(CleanError::before("Firefox пока не поддерживается"));
    }
    let _one = CLEANING.lock().unwrap_or_else(|e| e.into_inner());
    if procs::is_running(&browser.process) {
        if !close {
            return Err(CleanError::before(format!("{} открыт — закрой его, чтобы очистить", browser.name)));
        }
        procs::close(&browser.process, &browser.name).map_err(CleanError::before)?;
    }
    clean::clean_profile(&profile.path, request, clean::backup_dir(&browser.id, &profile.id))
}

#[tauri::command]
async fn clean_profile(browser_id: String, profile_id: String, request: CleanRequest, close: bool) -> Result<CleanReport, CleanError> {
    tauri::async_runtime::spawn_blocking(move || clean_blocking(&browser_id, &profile_id, &request, close))
        .await
        .map_err(|e| CleanError { message: e.to_string(), touched: true })?
}

pub fn run() {
    snapshot::sweep_stale();
    vss::sweep_stale();
    clean::sweep_stale();
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![list_browsers, scan_profile, running_processes, clean_profile])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
