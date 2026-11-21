//! IPC commands for the web UI. Thin wrappers: the logic lives in the modules.

use crate::cards::{self, Anchor, CardOpen};
use crate::config::{DockItem, DockStyle, Settings};
use crate::komorebi::{self, KState};
use crate::{migrate, system, AppState};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};

type R<T> = Result<T, String>;

#[tauri::command]
pub fn settings_get(state: State<AppState>) -> Settings {
    state.store.get()
}

#[tauri::command]
pub fn settings_set_items(app: AppHandle, state: State<AppState>, items: Vec<DockItem>) -> Settings {
    let s = state.store.update(|s| s.items = items);
    let _ = app.emit("settings:changed", &s);
    s
}

#[tauri::command]
pub async fn card_open(app: AppHandle, window: WebviewWindow, uid: String, widget: String, kind: Option<String>, payload: Option<serde_json::Value>, anchor: Anchor) -> R<()> {
    let state = app.state::<AppState>();
    let layout = state.docks.layout_for(window.label()).ok_or("not a dock window")?;
    let s = state.store.get();
    let req = CardOpen {
        uid,
        widget,
        kind: kind.unwrap_or_else(|| "card".into()),
        payload: payload.unwrap_or(serde_json::Value::Null),
        dock: window.label().to_string(),
        edge: layout.edge,
        light: s.style == DockStyle::White,
        style: serde_json::to_value(s.style).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default(),
    };
    state.cards.open(&app, &window, req, anchor, layout.scale).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn card_ready(app: AppHandle, state: State<AppState>, width: f64, height: f64) -> R<()> {
    state.cards.ready(&app, width, height).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn card_close(app: AppHandle, state: State<AppState>) {
    state.cards.hide(&app);
}

/// Send an action from a card back to the dock that opened it (e.g. a timer started in the card).
#[tauri::command]
pub fn card_action(app: AppHandle, dock: String, uid: String, action: String, data: Option<serde_json::Value>) {
    let _ = app.emit_to(dock.as_str(), "card:action", serde_json::json!({ "uid": uid, "action": action, "data": data }));
}

/// Runs on its own STA thread: ShellExecute can take seconds (SmartScreen, a cold Office start, a
/// network path) and must never block the main thread, which would freeze every Strata window.
#[tauri::command]
pub async fn launch(app: AppHandle, target: String, args: Option<String>) -> R<()> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        unsafe { let _ = windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_APARTMENTTHREADED); }
        let _ = tx.send(system::launch(&target, args.as_deref()));
    });
    // Still starting after 8 s: report success (the launch is in progress, not failed).
    let r = rx.recv_timeout(std::time::Duration::from_secs(8)).unwrap_or(Ok(()));
    if r.is_ok() {
        let h = app.clone();
        let _ = app.run_on_main_thread(move || {
            let st = h.state::<AppState>();
            if st.cards.is_open() {
                st.cards.hide(&h);
            }
        });
    }
    r
}

#[tauri::command]
pub fn power(action: String) -> R<()> {
    system::power(&action)
}

#[tauri::command]
pub fn start_menu() {
    system::open_start();
}

#[tauri::command]
pub fn winx_menu() {
    system::open_winx();
}

#[tauri::command]
pub fn uptime() -> u64 {
    system::uptime()
}

#[tauri::command]
pub fn komorebi_cmd(args: Vec<String>) -> R<()> {
    komorebi::run(args)
}

#[tauri::command]
pub fn komorebi_control(action: String) -> R<()> {
    komorebi::control(&action)
}

#[tauri::command]
pub fn komorebi_state(state: State<AppState>) -> KState {
    state.kstate.lock().unwrap().clone()
}

#[tauri::command]
pub fn data_read(name: String) -> R<serde_json::Value> {
    let p = migrate::data_path(&name).ok_or("unknown data file")?;
    match std::fs::read_to_string(&p) {
        Ok(t) => serde_json::from_str(&t).map_err(|e| e.to_string()),
        Err(_) => Ok(serde_json::Value::Null),
    }
}

#[tauri::command]
pub fn data_write(app: AppHandle, name: String, value: serde_json::Value) -> R<()> {
    let p = migrate::data_path(&name).ok_or("unknown data file")?;
    let tmp = p.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &p).map_err(|e| e.to_string())?;
    // Other windows showing the same data (dock tile vs card) refresh.
    let _ = app.emit("data:changed", &name);
    Ok(())
}

/// Hide the card when it loses focus (click anywhere else).
pub fn on_card_blur(app: &AppHandle) {
    let state = app.state::<AppState>();
    if state.cards.is_open() {
        state.cards.hide_on_blur(app);
    }
}

pub const CARD_LABEL: &str = cards::LABEL;
pub static CARD_FOCUSED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub static OVERLAY_FOCUSED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `http://icon.localhost/<url-encoded path>?s=<px>` → PNG.
pub fn icon_protocol(request: tauri::http::Request<Vec<u8>>, responder: tauri::UriSchemeResponder) {
    std::thread::spawn(move || {
        let uri = request.uri();
        let path = percent_decode(uri.path().trim_start_matches('/'));
        let size = uri.query().and_then(|q| q.split('&').find_map(|kv| kv.strip_prefix("s="))).and_then(|v| v.parse().ok()).unwrap_or(64);
        let resp = match system::icons::get(&path, size) {
            Some(png) => tauri::http::Response::builder().header("Content-Type", "image/png").header("Cache-Control", "max-age=86400").body(png),
            None => tauri::http::Response::builder().status(404).body(Vec::new()),
        };
        responder.respond(resp.unwrap());
    });
}

/// async: Tauri runs it off the main thread, where creating the overlay window on first use is safe.
#[tauri::command]
pub async fn overlay_open(app: AppHandle, window: WebviewWindow, kind: String) -> R<()> {
    app.state::<AppState>().cards.overlay_open(&app, &window, &kind).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn overlay_close(app: AppHandle, state: State<AppState>) {
    state.cards.overlay_close(&app);
}

pub const OVERLAY_LABEL: &str = cards::OVERLAY;
pub const POWER_LABEL: &str = cards::POWER;

/// The overlay page asks this when it loads, in case it missed the open event.
#[tauri::command]
pub fn overlay_pending(state: State<AppState>) -> Option<String> {
    state.cards.overlay_pending.lock().unwrap().clone()
}

fn json_merge(a: &mut serde_json::Value, b: serde_json::Value) {
    match (a, b) {
        (serde_json::Value::Object(a), serde_json::Value::Object(b)) => {
            for (k, v) in b {
                json_merge(a.entry(k).or_insert(serde_json::Value::Null), v);
            }
        }
        (a, b) => *a = b,
    }
}

/// Merge a partial settings object, persist it, and apply it live (docks re-placed or rebuilt).
#[tauri::command]
pub async fn settings_update(app: AppHandle, patch: serde_json::Value) -> R<Settings> {
    let state = app.state::<AppState>();
    let before = state.store.get();
    let mut v = serde_json::to_value(&before).map_err(|e| e.to_string())?;
    json_merge(&mut v, patch);
    let next: Settings = serde_json::from_value(v).map_err(|e| format!("invalid setting: {e}"))?;
    let s = state.store.update(|s| *s = next);
    let _ = app.emit("settings:changed", &s);
    // Glass device/mode changes need fresh docks; everything else is a re-place.
    let rebuild = before.live_glass != s.live_glass;
    let taskbar_changed = before.taskbar_mode != s.taskbar_mode;
    if before.power_backdrop != s.power_backdrop {
        // Recreated with the new backdrop next time it opens.
        if let Some(w) = app.get_webview_window(crate::cards::POWER) { let _ = w.destroy(); }
    }
    let h = app.clone();
    let _ = app.run_on_main_thread(move || {
        let st = h.state::<AppState>();
        if rebuild {
            st.docks.shutdown();
        }
        st.docks.sync();
        if taskbar_changed {
            crate::shell::taskbar::stop();
            *st.taskbar.lock().unwrap() = crate::shell::taskbar::start(st.store.get().taskbar_mode);
        }
    });
    Ok(s)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayInfo {
    pub id: String,
    pub name: String,
    pub gdi_name: String,
    pub primary: bool,
    pub width: i32,
    pub height: i32,
    pub enabled: bool,
}

#[tauri::command]
pub fn displays_list(state: State<AppState>) -> Vec<DisplayInfo> {
    let s = state.store.get();
    crate::shell::displays::enumerate()
        .into_iter()
        .map(|d| DisplayInfo {
            enabled: d.dock_enabled(&s),
            width: d.rect[2] - d.rect[0],
            height: d.rect[3] - d.rect[1],
            id: d.id,
            name: d.name,
            gdi_name: d.gdi_name,
            primary: d.primary,
        })
        .collect()
}

pub const SETTINGS_LABEL: &str = "settings";

/// Open (or focus) the Settings window. async: window creation off the main thread.
#[tauri::command]
pub async fn open_settings(app: AppHandle) -> R<()> {
    app.state::<AppState>().cards.hide(&app);
    if let Some(w) = app.get_webview_window(SETTINGS_LABEL) {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
        return Ok(());
    }
    let w = tauri::WebviewWindowBuilder::new(&app, SETTINGS_LABEL, tauri::WebviewUrl::App("index.html#settings".into()))
        .title("Strata Settings")
        .inner_size(860.0, 600.0)
        .min_inner_size(700.0, 460.0)
        .center()
        .build()
        .map_err(|e| e.to_string())?;
    let _ = w.set_focus();
    Ok(())
}

/// Quit cleanly: AppBars removed and the taskbar restored (RunEvent::Exit).
#[tauri::command]
pub fn quit(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub fn taskbar_state(state: State<AppState>) -> crate::shell::taskbar::TaskbarStatus {
    state.taskbar.lock().unwrap().clone()
}

/// The dock painted its first frame (shown only then, so it never flashes empty).
#[tauri::command]
pub fn dock_ready(window: WebviewWindow, state: State<AppState>) {
    state.docks.mark_ready(window.label());
}

/// The overlay painted its content: now show it (no empty or stale first frame).
#[tauri::command]
pub fn overlay_ready(app: AppHandle, state: State<AppState>) {
    state.cards.overlay_show(&app);
}

#[tauri::command]
pub fn media_state(state: State<AppState>) -> crate::system::media::MediaState {
    state.media.lock().unwrap().as_ref().map(|m| m.state()).unwrap_or_default()
}

#[tauri::command]
pub async fn media_control(app: AppHandle, action: String) -> R<()> {
    let m = app.state::<AppState>().media.lock().unwrap().clone().ok_or("media sessions unavailable")?;
    m.control(&action)
}

#[tauri::command]
pub async fn media_seek(app: AppHandle, seconds: f64) -> R<()> {
    let m = app.state::<AppState>().media.lock().unwrap().clone().ok_or("media sessions unavailable")?;
    m.seek(seconds)
}

#[tauri::command]
pub fn audio_state(state: State<AppState>) -> crate::system::audio::AudioState {
    state.audio.lock().unwrap().as_ref().map(|a| a.state()).unwrap_or_default()
}

#[tauri::command]
pub async fn audio_set_volume(app: AppHandle, volume: u32) -> R<()> {
    let a = app.state::<AppState>().audio.lock().unwrap().clone().ok_or("audio unavailable")?;
    a.set_volume(volume)
}

#[tauri::command]
pub async fn audio_set_mute(app: AppHandle, muted: bool) -> R<()> {
    let a = app.state::<AppState>().audio.lock().unwrap().clone().ok_or("audio unavailable")?;
    a.set_mute(muted)
}

#[tauri::command]
pub async fn wifi_state() -> crate::system::net::WifiState {
    crate::system::net::wifi()
}

#[tauri::command]
pub async fn net_info() -> crate::system::netinfo::NetInfo {
    crate::system::netinfo::info()
}

#[tauri::command]
pub async fn wifi_networks() -> Vec<crate::system::netinfo::WifiNetwork> {
    crate::system::netinfo::networks()
}

#[tauri::command]
pub async fn wifi_connect(ssid: String) -> R<()> {
    crate::system::netinfo::connect(&ssid)
}

/// Where the Now Playing cover art is in this dock window (CSS px), or null: the native equaliser
/// bars are drawn there (glass/eq.rs).
#[tauri::command]
pub fn eq_rect(window: WebviewWindow, state: State<AppState>, rect: Option<[f64; 4]>) {
    if let Some(g) = state.docks.glass_for(window.label()) {
        let k = window.scale_factor().unwrap_or(1.0);
        g.set_eq_rect(rect.map(|r| [(r[0] * k) as f32, (r[1] * k) as f32, (r[2] * k) as f32, (r[3] * k) as f32]));
    }
}

#[tauri::command]
pub fn windows_list() -> Vec<crate::system::winlist::AppWindow> {
    crate::system::winlist::last().lock().unwrap().clone()
}

/// Bring a window forward (or minimise it if it's already in front). If komorebi has it on another
/// workspace, that workspace is focused first.
#[tauri::command]
pub async fn window_activate(app: AppHandle, hwnd: i64, toggle: bool) {
    let loc = {
        let st = app.state::<AppState>();
        let k = st.kstate.lock().unwrap();
        k.monitors.iter().find_map(|m| {
            m.workspaces.iter().find(|w| w.windows.iter().any(|x| x.hwnd == hwnd))
                .filter(|w| w.index != m.focused_workspace)
                .map(|w| (m.index, w.index))
        })
    };
    crate::system::winlist::activate(hwnd, toggle, loc);
}

#[tauri::command]
pub fn window_close(hwnd: i64) {
    crate::system::winlist::close(hwnd);
}

#[tauri::command]
pub async fn gallery_state() -> crate::wallpaper_engine::gallery::GalleryState {
    crate::wallpaper_engine::gallery::state()
}

#[tauri::command]
pub async fn gallery_pictures(folder: String) -> Vec<String> {
    crate::wallpaper_engine::gallery::pictures(&folder)
}

/// kind: "we" (Wallpaper Engine item file) or "picture" (image path). monitor: index or null = all.
#[tauri::command]
pub async fn gallery_apply(app: AppHandle, kind: String, file: String, monitor: Option<usize>) -> R<()> {
    let r = match kind.as_str() {
        "we" => crate::wallpaper_engine::gallery::apply_we(&file, monitor),
        "picture" => crate::wallpaper_engine::gallery::apply_picture(&file, monitor),
        _ => Err("unknown wallpaper kind".into()),
    };
    // Glass shows the wallpaper behind the dock: refresh it after a change.
    let h = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(1200));
        let h2 = h.clone();
        let _ = h.run_on_main_thread(move || h2.state::<AppState>().docks.for_each_glass(|g| g.refresh_background()));
    });
    r
}

/// `http://thumb.localhost/<url-encoded path>?w=<px>` -> JPEG thumbnail.
pub fn thumb_protocol(request: tauri::http::Request<Vec<u8>>, responder: tauri::UriSchemeResponder) {
    std::thread::spawn(move || {
        let uri = request.uri();
        let path = percent_decode(uri.path().trim_start_matches('/'));
        let w = uri.query().and_then(|q| q.split('&').find_map(|kv| kv.strip_prefix("w="))).and_then(|v| v.parse().ok()).unwrap_or(320);
        // raw=1: the file as is, never cached (the region-capture still image).
        if uri.query().is_some_and(|q| q.split('&').any(|kv| kv == "raw=1")) {
            let resp = match std::fs::read(&path) { Ok(b) => tauri::http::Response::builder().header("Content-Type", "image/png").header("Cache-Control", "no-store").body(b), Err(_) => tauri::http::Response::builder().status(404).body(Vec::new()) };
            responder.respond(resp.unwrap());
            return;
        }
        let resp = match crate::system::thumbs::get(&path, w) {
            Some(jpg) => tauri::http::Response::builder().header("Content-Type", if jpg.starts_with(&[0x89, b'P']) { "image/png" } else { "image/jpeg" }).header("Cache-Control", "max-age=86400").body(jpg),
            None => tauri::http::Response::builder().status(404).body(Vec::new()),
        };
        responder.respond(resp.unwrap());
    });
}

/// The preview card placed its thumbnail slots (CSS px): show live DWM thumbnails there.
#[tauri::command]
pub fn previews_set(app: AppHandle, slots: Vec<crate::system::previews::Slot>) {
    let Some(w) = app.get_webview_window(cards::LABEL) else { return };
    let k = w.scale_factor().unwrap_or(1.0);
    let slots: Vec<_> = slots.into_iter().map(|s| crate::system::previews::Slot { hwnd: s.hwnd, x: s.x * k, y: s.y * k, w: s.w * k, h: s.h * k }).collect();
    if let Ok(h) = w.hwnd() {
        crate::system::previews::set(windows::Win32::Foundation::HWND(h.0 as _), &slots);
    }
}

/// Close the card only if it is still the given one (hover previews closing after the pointer left).
#[tauri::command]
pub fn card_close_if(app: AppHandle, state: State<AppState>, uid: String) {
    if cards::PREVIEW_HOVER.load(std::sync::atomic::Ordering::SeqCst) {
        return; // pointer is on the preview itself
    }
    if state.cards.open_uid().as_deref() == Some(uid.as_str()) {
        state.cards.hide(&app);
    }
}


#[tauri::command]
pub fn preview_hover(on: bool) {
    cards::PREVIEW_HOVER.store(on, std::sync::atomic::Ordering::SeqCst);
}

#[tauri::command]
pub async fn settings_reset(app: AppHandle) -> R<Settings> {
    let state = app.state::<AppState>();
    let before = state.store.get();
    let s = state.store.update(|s| {
        let keep_items = std::mem::take(&mut s.items);
        let keep_displays = std::mem::take(&mut s.displays);
        *s = Settings::default();
        s.items = keep_items;
        s.displays = keep_displays;
    });
    let _ = app.emit("settings:changed", &s);
    let rebuild = before.live_glass != s.live_glass;
    let h = app.clone();
    let _ = app.run_on_main_thread(move || {
        let st = h.state::<AppState>();
        if rebuild { st.docks.shutdown(); }
        st.docks.sync();
    });
    Ok(s)
}

/// Whether a secret is stored. Secrets themselves never go back to the web UI.
#[tauri::command]
pub fn secret_has(name: String) -> R<bool> {
    Ok(crate::secrets::get(&name)?.is_some())
}

#[tauri::command]
pub fn secret_set(name: String, value: String) -> R<()> {
    if name.starts_with("google-") || name == "yahoo-imap" || name == "calendars" { return Err("use the sign-in for this service".into()) }
    crate::secrets::set(&name, &value)
}

#[tauri::command]
pub fn secret_delete(name: String) -> R<()> {
    crate::secrets::delete(&name)
}

/// Download a wallpaper image (https only) into %LOCALAPPDATA%\Strata\wallpapers and return its path.
#[tauri::command]
pub async fn download_wallpaper(url: String, name: String) -> R<String> {
    if !url.starts_with("https://") {
        return Err("only https downloads are allowed".into());
    }
    let safe: String = name.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').take(64).collect();
    let dir = crate::config::local_dir().join("wallpapers");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{safe}.jpg"));
    if !path.exists() {
        let resp = ureq::get(&url).call().map_err(|e| format!("download failed: {e}"))?;
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut resp.into_body().into_reader(), &mut bytes).map_err(|e| e.to_string())?;
        std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    }
    Ok(path.to_string_lossy().into_owned())
}

/// Authenticated GET for services whose key lives in Credential Manager: the key never reaches the
/// page. `auth` = "unsplash" adds "Authorization: Client-ID <key>". Only allow-listed hosts.
#[tauri::command]
pub async fn http_get(url: String, auth: Option<String>) -> R<serde_json::Value> {
    const HOSTS: &[&str] = &["https://api.unsplash.com/", "https://api.frankfurter.dev/", "https://api.frankfurter.app/", "https://api.binance.com/"];
    if !HOSTS.iter().any(|h| url.starts_with(h)) {
        return Err("host not allowed".into());
    }
    let mut req = ureq::get(&url).header("Accept-Version", "v1");
    if auth.as_deref() == Some("unsplash") {
        let key = crate::secrets::get("unsplash")?.ok_or("Unsplash isn't connected")?;
        req = req.header("Authorization", &format!("Client-ID {key}"));
    }
    match req.call() {
        Ok(r) => r.into_body().read_json::<serde_json::Value>().map_err(|e| e.to_string()),
        Err(ureq::Error::StatusCode(401)) => Err("The key was rejected (401). Check the Access Key in your Unsplash app.".into()),
        Err(ureq::Error::StatusCode(403)) => Err("Unsplash refused the request (403): the hourly limit (50/h for new apps) may be used up.".into()),
        Err(e) => Err(format!("Couldn't reach Unsplash: {e}")),
    }
}

/// "Change now" from the gallery's schedule section.
#[tauri::command]
pub async fn wallpaper_change_now(app: AppHandle) -> R<()> {
    let s = app.state::<AppState>().store.get().wallpaper_schedule;
    crate::wallpaper_engine::gallery::change_now(&s)
}

#[tauri::command]
pub fn tray_icons() -> Vec<crate::system::tray::TrayIcon> {
    crate::system::tray::list()
}

/// button: left | right | double | middle. x, y: screen position of the icon in CSS px of the
/// calling window (converted to physical px here).
#[tauri::command]
pub fn tray_click(window: WebviewWindow, key: String, button: String, x: f64, y: f64) -> R<()> {
    let k = window.scale_factor().unwrap_or(1.0);
    crate::system::tray::click(&key, &button, (x * k) as i32, (y * k) as i32)
}

#[tauri::command]
pub fn card_sticky(state: State<AppState>, on: bool) {
    state.cards.sticky.store(on, std::sync::atomic::Ordering::SeqCst);
}

#[tauri::command]
pub async fn we_control(action: String, monitor: Option<usize>) -> R<()> {
    crate::wallpaper_engine::gallery::control(&action, monitor)
}

#[tauri::command]
pub async fn we_playlist(name: String, monitor: Option<usize>) -> R<()> {
    crate::wallpaper_engine::gallery::open_playlist(&name, monitor)
}

#[tauri::command]
pub async fn audio_set_default(id: String) -> R<()> {
    crate::system::audio_ext::set_default_output(&id)
}

#[tauri::command]
pub async fn audio_sessions() -> Vec<crate::system::audio_ext::Session> {
    crate::system::audio_ext::sessions()
}

#[tauri::command]
pub async fn audio_session_set(pid: u32, volume: Option<u32>, muted: Option<bool>) -> R<()> {
    crate::system::audio_ext::set_session(pid, volume, muted)
}

#[tauri::command]
pub async fn sys_stats(procs: Option<bool>, drives: Option<bool>) -> crate::system::stats::Stats {
    crate::system::stats::sample(procs.unwrap_or(false), drives.unwrap_or(false))
}

#[tauri::command]
pub fn recycle_bin() -> crate::system::misc::Bin { crate::system::misc::recycle_bin() }

#[tauri::command]
pub async fn recycle_bin_empty() -> R<()> { crate::system::misc::empty_recycle_bin() }

#[tauri::command]
pub fn theme_is_light() -> bool { crate::system::misc::is_light() }

#[tauri::command]
pub fn theme_set_light(light: bool) -> R<()> { crate::system::misc::set_light(light) }

#[tauri::command]
pub async fn mic_muted() -> Option<bool> { crate::system::misc::mic_muted() }

#[tauri::command]
pub async fn mic_set_muted(muted: bool) -> R<()> { crate::system::misc::set_mic_muted(muted) }

/// show-desktop | notifications | keyboard
#[tauri::command]
pub async fn sys_action(action: String) -> R<()> {
    match action.as_str() {
        "show-desktop" => crate::system::misc::show_desktop(),
        "notifications" => crate::system::misc::notification_center(),
        "keyboard" => return crate::system::launch("osk.exe", None),
        "task-manager" => return crate::system::launch("taskmgr.exe", None),
        "project" => crate::system::misc::project_menu(),
        _ => return Err("unknown action".into()),
    }
    Ok(())
}

#[tauri::command]
pub fn kb_layouts() -> Vec<crate::system::misc::Layout> { crate::system::misc::layouts() }

#[tauri::command]
pub fn kb_set_layout(id: String) -> R<()> { crate::system::misc::set_layout(&id) }

#[tauri::command]
pub fn battery() -> crate::system::misc::Battery { crate::system::misc::battery() }

#[tauri::command]
pub fn launch_admin(target: String) -> R<()> { crate::system::launch_admin(&target) }

#[tauri::command]
pub fn reveal(target: String) -> R<()> { crate::system::reveal(&target) }

#[tauri::command]
pub async fn audio_devices() -> Vec<crate::system::audio_ext::OutDevice> { crate::system::audio_ext::out_devices() }

#[tauri::command]
pub async fn audio_device_set(id: String, volume: Option<u32>, muted: Option<bool>) -> R<()> { crate::system::audio_ext::set_device(&id, volume, muted) }

#[tauri::command]
pub async fn everything_search(query: String, max: Option<u32>, sort: Option<u32>) -> R<Vec<crate::system::everything::Hit>> {
    crate::system::everything::search(query, max.unwrap_or(40).min(200), sort.unwrap_or(20))
}

#[tauri::command]
pub fn everything_running() -> bool { crate::system::everything::running() }

// ---------------------------------------------------------------- online services
#[tauri::command]
pub async fn api(service: String, url: String, method: Option<String>, body: Option<serde_json::Value>, headers: Option<std::collections::HashMap<String, String>>) -> R<crate::services::Reply> {
    crate::services::request(&service, method.as_deref().unwrap_or("GET"), &url, body, headers.unwrap_or_default())
}
#[tauri::command]
pub async fn google_connect(id: String, secret: String) -> R<()> { crate::google::connect(&id, &secret) }
#[tauri::command]
pub fn google_status() -> bool { crate::google::connected() }
/// A Google OAuth client is saved (signing in again needs no pasting).
#[tauri::command]
pub fn google_client_saved() -> bool { matches!(crate::secrets::get("google-client"), Ok(Some(_))) }
#[tauri::command]
pub fn google_disconnect() { crate::google::disconnect() }
#[tauri::command]
pub async fn yahoo_login(email: String, password: String) -> R<()> { crate::mail::save_login(&email, &password) }
#[tauri::command]
pub async fn yahoo_mail(view: String, query: Option<String>) -> R<crate::mail::Summary> { crate::mail::summary(&view, query.as_deref(), 25) }
#[tauri::command]
pub fn yahoo_disconnect() -> R<()> { crate::secrets::delete("yahoo-imap") }

// ---------------------------------------------------------------- local data
#[tauri::command]
pub async fn opencode_usage() -> R<crate::local::OpenCode> { crate::local::opencode() }
#[tauri::command]
pub async fn screen_time(days: Vec<String>) -> std::collections::HashMap<String, serde_json::Value> { crate::local::screen_time(days) }
#[tauri::command]
pub async fn whatsapp() -> R<crate::local::WhatsApp> { crate::local::whatsapp() }
#[tauri::command]
pub async fn folder_list(which: String, limit: Option<usize>) -> R<Vec<crate::local::Entry>> { crate::local::list_folder(&which, limit.unwrap_or(30)) }
#[tauri::command]
pub fn folder_path(which: String) -> R<String> { crate::local::folder_path(&which) }

// ---------------------------------------------------------------- toggles, brightness
#[tauri::command]
pub async fn toggles_get() -> Vec<crate::toggles::Toggle> { crate::toggles::states() }
#[tauri::command]
pub async fn toggle_set(id: String, on: bool) -> R<()> { crate::toggles::set(&id, on) }
#[tauri::command]
pub async fn brightness_get() -> R<u32> { crate::toggles::brightness_get() }
#[tauri::command]
pub async fn brightness_set(value: u32) -> R<()> { crate::toggles::brightness_set(value) }

// ---------------------------------------------------------------- capture
#[tauri::command]
pub async fn capture_screen(text: bool) -> R<crate::capture::Saved> { crate::capture::screen(text) }
#[tauri::command]
pub async fn capture_window(text: bool) -> R<crate::capture::Saved> { crate::capture::window(text) }
#[tauri::command]
pub async fn capture_region_begin() -> R<String> { crate::capture::region_begin() }
#[tauri::command]
pub async fn capture_region_finish(x: u32, y: u32, w: u32, h: u32, text: bool) -> R<crate::capture::Saved> { crate::capture::region_finish(x, y, w, h, text) }
#[tauri::command]
pub fn capture_region_cancel() { crate::capture::region_cancel() }
#[tauri::command]
pub fn clipboard_text(text: String) { crate::capture::clipboard_text(&text) }

#[tauri::command]
pub fn capture_temp_path() -> String { std::env::temp_dir().join("strata-capture.png").to_string_lossy().into() }

// ---------------------------------------------------------------- calendars (iCal links, read-only)
// The secret links live in Credential Manager; the UI only ever sees names and colours.
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Cal { pub id: String, pub name: String, pub color: String, #[serde(default, skip_serializing_if = "String::is_empty")] pub url: String }
fn cals() -> Vec<Cal> { crate::secrets::get("calendars").ok().flatten().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default() }
fn save_cals(v: &[Cal]) -> R<()> { crate::secrets::set("calendars", &serde_json::to_string(v).unwrap()) }
#[tauri::command]
pub fn calendars_list() -> Vec<Cal> { cals().into_iter().map(|c| Cal { url: String::new(), ..c }).collect() }
#[tauri::command]
pub async fn calendars_add(name: String, url: String, color: String) -> R<()> {
    let url = url.trim().replacen("webcal://", "https://", 1);
    if !url.starts_with("https://") { return Err("Paste the secret iCal address (it starts with https:// or webcal://)".into()) }
    // Check it really is a calendar before saving.
    let body = ureq::get(&url).call().map_err(|e| format!("Could not load it: {e}"))?.body_mut().with_config().limit(16 << 20).read_to_string().map_err(|e| e.to_string())?;
    if !body.contains("BEGIN:VCALENDAR") { return Err("That address did not return a calendar (.ics)".into()) }
    let mut v = cals();
    v.push(Cal { id: format!("{:x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()), name: name.trim().into(), color, url });
    save_cals(&v)
}
#[tauri::command]
pub fn calendars_remove(id: String) -> R<()> { let v: Vec<Cal> = cals().into_iter().filter(|c| c.id != id).collect(); save_cals(&v) }
#[derive(serde::Serialize)]
pub struct CalData { pub id: String, pub name: String, pub color: String, pub ics: String, pub error: Option<String> }
#[tauri::command]
pub async fn calendars_fetch() -> Vec<CalData> {
    cals().into_iter().map(|c| {
        let r = ureq::get(&c.url).call().map_err(|e| e.to_string()).and_then(|mut r| r.body_mut().with_config().limit(16 << 20).read_to_string().map_err(|e| e.to_string()));
        match r {
            Ok(ics) => CalData { id: c.id, name: c.name, color: c.color, ics, error: None },
            Err(e) => CalData { id: c.id, name: c.name, color: c.color, ics: String::new(), error: Some(e) },
        }
    }).collect()
}

#[tauri::command]
pub async fn flow_toggle() -> R<()> { crate::toggles::flow_toggle() }
#[tauri::command]
pub async fn gamma_set(brightness: u32, warmth: u32) -> R<()> { crate::toggles::gamma_set(brightness, warmth) }
#[tauri::command]
pub fn gamma_reset() { crate::toggles::gamma_reset() }
#[tauri::command]
pub fn autostart_get() -> bool { crate::toggles::autostart() }
#[tauri::command]
pub fn autostart_set(on: bool) -> R<()> { crate::toggles::set_autostart(on) }
#[tauri::command]
pub fn capture_save_set(on: bool) { crate::capture::SAVE.store(on, std::sync::atomic::Ordering::Relaxed) }

// ---------------------------------------------------------------- vault
#[tauri::command]
pub fn vault_list() -> Vec<crate::vault::Entry> { crate::vault::list() }
#[tauri::command]
pub fn vault_state() -> bool { crate::vault::is_unlocked() }
#[tauri::command]
pub async fn vault_unlock(window: WebviewWindow, minutes: Option<u64>, password: Option<String>) -> R<String> {
    let h = window.hwnd().map_err(|e| e.to_string())?.0 as isize;
    crate::vault::unlock(h, minutes.unwrap_or(5), password)
}
#[tauri::command]
pub fn vault_lock_info() -> crate::vault::LockInfo { crate::vault::lock_info() }
#[tauri::command]
pub async fn vault_set_master(password: Option<String>, hello_too: bool) -> R<()> { crate::vault::set_master(password, hello_too) }
#[tauri::command]
pub async fn vault_import(path: String, delete_after: bool) -> R<crate::vault::ImportResult> { crate::vault::import_csv(&path, delete_after) }
#[tauri::command]
pub async fn vault_export(path: String, format: String) -> R<usize> { crate::vault::export(&path, &format) }
#[tauri::command]
pub async fn vault_health(breaches: bool) -> R<Vec<crate::vault::Health>> { crate::vault::health(breaches) }
#[tauri::command]
pub async fn vault_attach(id: String, path: String) -> R<crate::vault::Entry> { crate::vault::attach_file(&id, &path) }
#[tauri::command]
pub async fn vault_file_save(id: String, path: String) -> R<()> { crate::vault::export_file(&id, &path) }
#[tauri::command]
pub fn vault_file_remove(id: String) -> R<()> { crate::vault::remove_file(&id) }
#[tauri::command]
pub fn vault_passphrase(words: usize) -> String { crate::vault::passphrase(words) }

/// Native Open / Save dialog owned by the calling window. The card stays open meanwhile.
#[tauri::command]
pub async fn pick_file(app: AppHandle, window: WebviewWindow, save: bool, title: String, filters: Vec<(String, String)>, default_name: Option<String>) -> Option<String> {
    let h = window.hwnd().map(|h| h.0 as isize).unwrap_or(0);
    let st = app.state::<AppState>();
    let was = st.cards.sticky.swap(true, std::sync::atomic::Ordering::SeqCst);
    let r = crate::dialog::pick(h, save, &title, filters, default_name);
    st.cards.sticky.store(was, std::sync::atomic::Ordering::SeqCst);
    r
}
#[tauri::command]
pub fn vault_lock() { crate::vault::lock() }
#[tauri::command]
pub fn vault_save(entry: crate::vault::Entry, secret: Option<String>, notes: Option<String>, fields: Option<std::collections::BTreeMap<String, String>>) -> R<crate::vault::Entry> { crate::vault::save(entry, secret, notes, fields) }
#[tauri::command]
pub fn vault_delete(id: String) -> R<()> { crate::vault::delete(&id) }
#[tauri::command]
pub fn vault_reveal(id: String) -> R<crate::vault::Secret> { crate::vault::reveal(&id) }
#[tauri::command]
pub fn vault_copy(id: String, field: String) -> R<()> { crate::vault::copy(&id, &field) }
#[tauri::command]
pub fn vault_generate(length: usize, symbols: bool) -> String { crate::vault::generate(length, symbols) }


/// The media player the idle Now Playing tile offers: Spotify (desktop or Store) when installed.
/// Returns (icon path, launch target), or None so the tile shows a neutral "Nothing playing".
#[tauri::command]
pub fn default_player() -> Option<(String, String)> {
    let appdata = std::env::var("APPDATA").unwrap_or_default();
    let exe = std::path::Path::new(&appdata).join("Spotify").join("Spotify.exe");
    if exe.is_file() {
        let p = exe.to_string_lossy().into_owned();
        return Some((p, "spotify:".into()));
    }
    let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
    let store = std::fs::read_dir(std::path::Path::new(&local).join("Packages")).ok()?
        .flatten().any(|e| e.file_name().to_string_lossy().starts_with("SpotifyAB.SpotifyMusic_"));
    store.then(|| (r"shell:AppsFolder\SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify".to_string(), "spotify:".to_string()))
}

#[tauri::command]
pub async fn office_apps() -> Vec<crate::local::SuiteApp> { crate::local::office_apps() }

/// Project Launcher (widget id "local-app"): status / start / stop of project `project` on dock
/// item `uid`. Commands come from the saved settings only (D-033).
#[tauri::command]
pub async fn project_action(state: State<'_, AppState>, uid: String, project: Option<String>, action: String) -> R<bool> {
    let p = crate::projects::find(&state.store.get(), &uid, project.as_deref().unwrap_or("main"))?;
    match action.as_str() {
        "status" => Ok(crate::projects::running(&p)),
        "start" | "stop" => { crate::projects::act(&p, &action)?; Ok(action == "start") }
        _ => Err("unknown action".into()),
    }
}
#[tauri::command]
pub async fn project_detect(folder: String) -> R<crate::projects::Project> { crate::projects::detect(&folder) }
#[tauri::command]
pub async fn project_clone(url: String, parent: String) -> R<String> { crate::projects::clone(&url, &parent) }
#[tauri::command]
pub async fn pick_folder(app: AppHandle, window: WebviewWindow, title: String) -> Option<String> {
    let h = window.hwnd().map(|h| h.0 as isize).unwrap_or(0);
    let st = app.state::<AppState>();
    let was = st.cards.sticky.swap(true, std::sync::atomic::Ordering::SeqCst);
    let r = crate::dialog::pick_folder(h, &title);
    st.cards.sticky.store(was, std::sync::atomic::Ordering::SeqCst);
    r
}

/// Save text (meeting notes as Markdown, ...) to a file the user picks in a Save dialog. The page
/// never chooses the path itself. Returns the path, or None when cancelled.
#[tauri::command]
pub async fn save_text_as(app: AppHandle, window: WebviewWindow, title: String, default_name: String, text: String) -> R<Option<String>> {
    let h = window.hwnd().map(|h| h.0 as isize).unwrap_or(0);
    let st = app.state::<AppState>();
    let was = st.cards.sticky.swap(true, std::sync::atomic::Ordering::SeqCst);
    let ext = default_name.rsplit_once('.').map(|x| x.1.to_string()).unwrap_or_else(|| "txt".into());
    let p = crate::dialog::pick(h, true, &title, vec![(format!("{} file", ext.to_uppercase()), format!("*.{ext}")), ("All files".into(), "*.*".into())], Some(default_name));
    st.cards.sticky.store(was, std::sync::atomic::Ordering::SeqCst);
    match p {
        Some(p) => { std::fs::write(&p, text).map_err(|e| e.to_string())?; Ok(Some(p)) }
        None => Ok(None),
    }
}

/// Uncaught errors from any Strata window, written to Strata.log (support: users can send the log).
#[tauri::command]
pub fn ui_error(window: WebviewWindow, message: String) {
    log::error!("ui [{}]: {}", window.label(), message.chars().take(2000).collect::<String>());
}

/// Terminal widget (runs on its own thread; an elevation prompt can take a while).
#[tauri::command]
pub async fn terminal_run(app: AppHandle, command: String, shell: String, admin: bool, cwd: Option<String>) -> R<()> {
    let r = tauri::async_runtime::spawn_blocking(move || system::terminal(&command, &shell, admin, cwd.as_deref())).await.map_err(|e| e.to_string())?;
    if r.is_ok() {
        let h = app.clone();
        let _ = app.run_on_main_thread(move || { let st = h.state::<AppState>(); if st.cards.is_open() { st.cards.hide(&h); } });
    }
    r
}
/// Which shells exist here: (pwsh, Windows Terminal).
#[tauri::command]
pub fn terminal_info() -> (bool, bool) {
    use std::os::windows::process::CommandExt;
    let wt = std::env::var("LOCALAPPDATA").map(|l| std::path::Path::new(&l).join(r"Microsoft\WindowsApps\wt.exe").exists()).unwrap_or(false);
    let pwsh = std::process::Command::new("where").arg("pwsh").creation_flags(0x0800_0000).output().map(|o| o.status.success()).unwrap_or(false);
    (pwsh, wt)
}

/// Start-style search: installed apps (shell:AppsFolder).
#[tauri::command]
pub async fn start_apps() -> Vec<crate::system::startapps::StartApp> {
    tauri::async_runtime::spawn_blocking(crate::system::startapps::apps).await.unwrap_or_default()
}
/// Start-style search: files from your main folders (when Everything isn't running).
#[tauri::command]
pub async fn quick_files(query: String, max: Option<usize>) -> Vec<crate::system::startapps::FileHit> {
    tauri::async_runtime::spawn_blocking(move || crate::system::startapps::files(&query, max.unwrap_or(8))).await.unwrap_or_default()
}
