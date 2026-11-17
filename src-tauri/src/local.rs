//! Widgets that read data already on this PC: OpenCode history, screen time (our own tracker),
//! WhatsApp notifications, recent files and folders (Downloads, Screenshots).

use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn home() -> PathBuf { std::env::var_os("USERPROFILE").map(Into::into).unwrap_or_default() }
fn now_ms() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64 }

// ---------------------------------------------------------------- OpenCode (local history)

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpenCode {
    pub session_tokens: u64,
    pub session_title: String,
    pub week_tokens: u64,
    pub today_tokens: u64,
    pub top_model: String,
    pub last_active: u64,
}

pub fn opencode() -> Result<OpenCode, String> {
    let roots = [
        home().join(".local/share/opencode/storage"),
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_default().join("opencode/storage"),
    ];
    let root = roots.iter().find(|p| p.exists()).ok_or("OpenCode isn't installed (no local history found)")?;
    let week_ago = now_ms().saturating_sub(7 * 864e5 as u64);
    let day_start = {
        let n = now_ms();
        n - n % 86_400_000
    };
    let mut models: HashMap<String, u64> = HashMap::new();
    let mut by_session: HashMap<String, (u64, u64)> = HashMap::new();
    let mut out = OpenCode::default();
    let msg_dir = root.join("message");
    for s in std::fs::read_dir(&msg_dir).map_err(|e| e.to_string())?.flatten() {
        // Skip sessions untouched for a week (cheap: directory mtime).
        if s.metadata().and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| (d.as_millis() as u64) < week_ago).unwrap_or(false) { continue }
        for f in std::fs::read_dir(s.path()).into_iter().flatten().flatten() {
            let Ok(bytes) = std::fs::read(f.path()) else { continue };
            let Ok(v) = serde_json::from_slice::<Value>(&bytes) else { continue };
            let t = &v["tokens"];
            let tokens = ["input", "output", "reasoning"].iter().map(|k| t[*k].as_u64().unwrap_or(0)).sum::<u64>()
                + t["cache"]["read"].as_u64().unwrap_or(0) + t["cache"]["write"].as_u64().unwrap_or(0);
            if tokens == 0 { continue }
            let created = v["time"]["created"].as_u64().unwrap_or(0);
            if created < week_ago { continue }
            out.week_tokens += tokens;
            if created >= day_start { out.today_tokens += tokens }
            if let Some(m) = v["modelID"].as_str() { *models.entry(m.into()).or_default() += tokens }
            let sid = v["sessionID"].as_str().unwrap_or("").to_string();
            let e = by_session.entry(sid).or_default();
            e.0 += tokens;
            e.1 = e.1.max(created);
            out.last_active = out.last_active.max(created);
        }
    }
    if let Some((sid, (tok, _))) = by_session.iter().max_by_key(|(_, (_, last))| *last) {
        out.session_tokens = *tok;
        // Session title from storage/session/<project>/<sid>.json if present.
        if let Ok(projects) = std::fs::read_dir(root.join("session")) {
            for p in projects.flatten() {
                if let Ok(b) = std::fs::read(p.path().join(format!("{sid}.json"))) {
                    if let Ok(v) = serde_json::from_slice::<Value>(&b) { out.session_title = v["title"].as_str().unwrap_or("").into() }
                }
            }
        }
    }
    out.top_model = models.into_iter().max_by_key(|(_, v)| *v).map(|(k, _)| k).unwrap_or_default();
    Ok(out)
}

// ---------------------------------------------------------------- Screen time

/// Samples the foreground app every 10 s (skipping idle time > 2 min and the lock screen) and keeps
/// per-day files `%APPDATA%\Strata\screentime\YYYY-MM-DD.json`: { hours: { "13": { "Code": 540 } } }.
pub fn start_screen_time() {
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
    std::thread::Builder::new().name("strata-screentime".into()).spawn(|| {
        let mut day = String::new();
        let mut data: Value = Value::Null;
        let mut dirty = 0u32;
        loop {
            std::thread::sleep(std::time::Duration::from_secs(10));
            let idle_ms = unsafe {
                let mut li = LASTINPUTINFO { cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32, ..Default::default() };
                if GetLastInputInfo(&mut li).as_bool() { windows::Win32::System::SystemInformation::GetTickCount().wrapping_sub(li.dwTime) } else { 0 }
            };
            let fg = unsafe { GetForegroundWindow() };
            if fg.0.is_null() { continue }
            let mut pid = 0u32;
            unsafe { GetWindowThreadProcessId(fg, Some(&mut pid)) };
            let exe = crate::shell::gamemode::process_name(pid);
            if exe.is_empty() || exe.eq_ignore_ascii_case("LockApp.exe") { continue }
            // Media playing in the foreground counts even without input (watching a video).
            if idle_ms > 120_000 && !fullscreen_foreground(fg) { continue }
            let (d, hour) = local_day_hour();
            if d != day {
                if dirty > 0 && !day.is_empty() { save_day(&day, &data) }
                day = d;
                data = load_day(&day);
                dirty = 0;
            }
            let app = exe.trim_end_matches(".exe").trim_end_matches(".EXE").to_string();
            let slot = &mut data["hours"][hour.to_string()][&app];
            *slot = Value::from(slot.as_u64().unwrap_or(0) + 10);
            dirty += 1;
            if dirty >= 6 { save_day(&day, &data); dirty = 0 }
        }
    }).ok();
}

fn fullscreen_foreground(h: windows::Win32::Foundation::HWND) -> bool {
    use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONULL};
    unsafe {
        let mon = MonitorFromWindow(h, MONITOR_DEFAULTTONULL);
        let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        let mut r = windows::Win32::Foundation::RECT::default();
        !mon.0.is_null() && GetMonitorInfoW(mon, &mut mi).as_bool() && windows::Win32::UI::WindowsAndMessaging::GetWindowRect(h, &mut r).is_ok()
            && r.left <= mi.rcMonitor.left && r.top <= mi.rcMonitor.top && r.right >= mi.rcMonitor.right && r.bottom >= mi.rcMonitor.bottom
    }
}

fn local_day_hour() -> (String, u32) {
    let st = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
    (format!("{:04}-{:02}-{:02}", st.wYear, st.wMonth, st.wDay), st.wHour as u32)
}
fn st_dir() -> PathBuf { crate::config::data_dir().join("screentime") }
fn load_day(day: &str) -> Value {
    std::fs::read(st_dir().join(format!("{day}.json"))).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_else(|| serde_json::json!({ "hours": {} }))
}
fn save_day(day: &str, v: &Value) {
    let _ = std::fs::create_dir_all(st_dir());
    let tmp = st_dir().join(format!("{day}.tmp"));
    if std::fs::write(&tmp, v.to_string()).is_ok() { let _ = std::fs::rename(&tmp, st_dir().join(format!("{day}.json"))); }
}
/// Days `YYYY-MM-DD` → their data (today includes the unsaved minute or so).
pub fn screen_time(days: Vec<String>) -> HashMap<String, Value> {
    days.into_iter().map(|d| { let v = load_day(&d); (d, v) }).collect()
}

// ---------------------------------------------------------------- WhatsApp (from Windows notifications)

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WaNote { pub sender: String, pub text: String, pub time: u64 }

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WhatsApp { pub unread: usize, pub notes: Vec<WaNote> }

/// Reads WhatsApp toasts still in Notification Center from the user's own notification database
/// (read-only). "Unread" = WhatsApp notifications not yet dismissed.
pub fn whatsapp() -> Result<WhatsApp, String> {
    let db = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_default().join("Microsoft/Windows/Notifications/wpndatabase.db");
    let c = rusqlite::Connection::open_with_flags(&db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX)
        .map_err(|e| format!("notification database: {e}"))?;
    let mut st = c.prepare(
        "SELECT n.Payload, n.ArrivalTime FROM Notification n JOIN NotificationHandler h ON n.HandlerId = h.RecordId \
         WHERE h.PrimaryId LIKE '%WhatsApp%' AND n.Type = 'toast' ORDER BY n.ArrivalTime DESC LIMIT 30",
    ).map_err(|e| e.to_string())?;
    let rows = st.query_map([], |r| Ok((r.get::<_, Vec<u8>>(0).or_else(|_| r.get::<_, String>(0).map(String::into_bytes))?, r.get::<_, i64>(1)?)))
        .map_err(|e| e.to_string())?;
    let mut notes = vec![];
    for (payload, t) in rows.flatten() {
        let xml = String::from_utf8_lossy(&payload);
        let texts: Vec<String> = xml.split("<text").skip(1).filter_map(|p| p.split_once('>').map(|(_, r)| r.split("</text>").next().unwrap_or("").to_string()))
            .map(|s| unescape(&s)).filter(|s| !s.is_empty()).collect();
        // FILETIME (100 ns since 1601) → unix ms.
        let ms = ((t as u64) / 10_000).saturating_sub(11_644_473_600_000);
        notes.push(WaNote { sender: texts.first().cloned().unwrap_or_default(), text: texts.get(1..).map(|x| x.join(" · ")).unwrap_or_default(), time: ms });
    }
    Ok(WhatsApp { unread: notes.len(), notes })
}

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&#39;", "'").replace("&amp;", "&")
}

// ---------------------------------------------------------------- Files

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry { pub path: String, pub name: String, pub dir: bool, pub size: u64, pub modified: u64 }

pub fn known_folder(which: &str) -> Result<PathBuf, String> {
    use windows::Win32::UI::Shell::*;
    let id = match which {
        "downloads" => FOLDERID_Downloads,
        "screenshots" => FOLDERID_Screenshots,
        "recent" => FOLDERID_Recent,
        "pictures" => FOLDERID_Pictures,
        _ => return Err("unknown folder".into()),
    };
    unsafe {
        let p = SHGetKnownFolderPath(&id, KNOWN_FOLDER_FLAG(0), None).map_err(|e| e.to_string())?;
        let s = p.to_string().unwrap_or_default();
        windows::Win32::System::Com::CoTaskMemFree(Some(p.0 as _));
        Ok(PathBuf::from(s))
    }
}

/// Newest entries of a known folder (or "recent": Explorer's recent items, shortcuts resolved by name).
pub fn list_folder(which: &str, limit: usize) -> Result<Vec<Entry>, String> {
    let dir = known_folder(which)?;
    let mut v: Vec<Entry> = std::fs::read_dir(&dir).map_err(|e| e.to_string())?.flatten().filter_map(|e| {
        let m = e.metadata().ok()?;
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name.eq_ignore_ascii_case("desktop.ini") { return None }
        if which == "recent" && !name.to_ascii_lowercase().ends_with(".lnk") { return None }
        Some(Entry {
            path: e.path().to_string_lossy().into(),
            name: if which == "recent" { name.trim_end_matches(".lnk").to_string() } else { name },
            dir: m.is_dir(),
            size: m.len(),
            modified: m.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_millis() as u64,
        })
    }).collect();
    v.sort_by(|a, b| b.modified.cmp(&a.modified));
    v.truncate(limit);
    Ok(v)
}

pub fn folder_path(which: &str) -> Result<String, String> { known_folder(which).map(|p| p.to_string_lossy().into()) }

#[allow(dead_code)]
fn exists(p: &Path) -> bool { p.exists() }

#[cfg(test)]
mod live_tests {
    #[test]
    #[ignore] // read-only checks against this PC
    fn read_only_backends() {
        match crate::services::request("claude", "GET", "https://api.anthropic.com/api/oauth/usage", None, Default::default()) {
            Ok(r) => println!("CLAUDE status={} five_hour={:?} seven_day={:?}", r.status, r.body["five_hour"]["utilization"], r.body["seven_day"]["utilization"]),
            Err(e) => println!("CLAUDE error: {e}"),
        }
        match super::whatsapp() { Ok(w) => println!("WHATSAPP unread={}", w.unread), Err(e) => println!("WHATSAPP error: {e}") }
        match super::opencode() { Ok(_) => println!("OPENCODE ok"), Err(e) => println!("OPENCODE: {e}") }
        for t in crate::toggles::states() { println!("TOGGLE {} on={} available={}", t.id, t.on, t.available) }
        println!("BRIGHTNESS {:?}", crate::toggles::brightness_get());
        for w in ["downloads", "screenshots", "recent"] { println!("FOLDER {w}: {:?}", super::list_folder(w, 3).map(|v| v.len())) }
        match crate::services::request("public", "GET", "https://query1.finance.yahoo.com/v8/finance/chart/AAPL?range=1d&interval=5m", None, Default::default()) {
            Ok(r) => println!("YAHOO status={} price={:?}", r.status, r.body["chart"]["result"][0]["meta"]["regularMarketPrice"]),
            Err(e) => println!("YAHOO error: {e}"),
        }
        println!("REFUSED foreign host: {:?}", crate::services::request("github", "GET", "https://evil.example.com/", None, Default::default()).err());
    }
}
