//! Background update checks (Settings → About → Updates): "notify" (default) checks GitHub at
//! startup and every 6 hours and puts a message on the dock; "auto" also downloads, installs and
//! restarts; "off" never checks on its own. The update is signed and verified by the updater plugin.

use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

/// Version found by the last background check, waiting to be installed.
static PENDING: Mutex<Option<String>> = Mutex::new(None);

pub fn pending() -> Option<String> {
    PENDING.lock().unwrap().clone()
}

pub fn start(app: AppHandle) {
    let _ = std::thread::Builder::new().name("updates".into()).spawn(move || {
        // Let sign-in settle first; then every 6 hours.
        std::thread::sleep(Duration::from_secs(90));
        loop {
            let mode = app.state::<crate::AppState>().store.get().update_mode;
            if mode != "off" {
                if let Err(e) = tauri::async_runtime::block_on(check(&app, mode == "auto")) {
                    log::warn!("update check failed: {e}");
                }
            }
            std::thread::sleep(Duration::from_secs(6 * 3600));
        }
    });
}

async fn check(app: &AppHandle, auto: bool) -> Result<(), String> {
    let updater = app
        .updater_builder()
        // The installer closes Strata: put the taskbar back first (the new Strata re-applies it).
        .on_before_exit(|| crate::shell::taskbar::stop())
        .build()
        .map_err(|e| e.to_string())?;
    let Some(u) = updater.check().await.map_err(|e| e.to_string())? else { return Ok(()) };
    log::info!("update available: {} (auto install: {auto})", u.version);
    if auto {
        let _ = app.emit("dock:notice", serde_json::json!({ "text": format!("Installing Strata {}… Strata restarts in a moment.", u.version) }));
        u.download_and_install(|_, _| {}, || {}).await.map_err(|e| e.to_string())?;
        app.restart();
    }
    let new = PENDING.lock().unwrap().replace(u.version.clone()).as_deref() != Some(u.version.as_str());
    if new {
        let _ = app.emit("dock:notice", serde_json::json!({
            "text": format!("Strata {} is available. Click to see what's new and install it.", u.version),
            "action": "update",
        }));
    }
    Ok(())
}
