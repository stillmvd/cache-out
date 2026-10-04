use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

const FIRST_CHECK: Duration = Duration::from_secs(10);
const EVERY: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Default)]
pub struct Updates(Mutex<Option<(Update, Vec<u8>)>>);

fn ready(app: &AppHandle) -> bool {
    app.state::<Updates>().0.lock().unwrap_or_else(|e| e.into_inner()).is_some()
}

pub fn spawn(app: &AppHandle) {
    if cfg!(debug_assertions) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(FIRST_CHECK);
        while !ready(&app) {
            if let Err(e) = tauri::async_runtime::block_on(fetch(&app)) {
                eprintln!("update: {e}");
            }
            if ready(&app) {
                break;
            }
            std::thread::sleep(EVERY);
        }
    });
}

async fn fetch(app: &AppHandle) -> tauri_plugin_updater::Result<()> {
    let Some(update) = app.updater_builder().restart_after_install(false).build()?.check().await? else {
        return Ok(());
    };
    let bytes = update.download(|_, _| {}, || {}).await?;
    *app.state::<Updates>().0.lock().unwrap_or_else(|e| e.into_inner()) = Some((update, bytes));
    let _ = app.emit("update://ready", ());
    Ok(())
}

pub fn install(app: &AppHandle, restart: bool) -> Result<(), String> {
    let pending = app.state::<Updates>().0.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let Some((update, bytes)) = pending else {
        return Ok(());
    };
    update.restart_after_install(restart).install(bytes).map_err(|e| format!("Не удалось обновить: {e}"))
}

#[tauri::command]
pub fn update_ready(app: AppHandle) -> bool {
    ready(&app)
}

#[tauri::command]
pub fn update_install(app: AppHandle) -> Result<(), String> {
    install(&app, true)
}
