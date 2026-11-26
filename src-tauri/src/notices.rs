//! Strata notifications (Settings → Notifications). When on, Windows' own pop-ups are switched off
//! per app (the "Show notification banners" switch Settings uses, HKCU\…\Notifications\Settings\<app>
//! ShowBanner = 0) and Strata shows each new notification itself: it reads them from the notification
//! database (wpndatabase.db), where they still arrive, so the notification list keeps them too.
//! Optionally Windows' notification sounds are silenced too (the Notification.* sound events).
//! Every value changed is saved first (notices-backup.json) and put back when the switch goes off.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use windows::core::HSTRING;
use windows::Win32::System::Registry::*;

const SETTINGS: &str = r"Software\Microsoft\Windows\CurrentVersion\Notifications\Settings";
const SOUNDS: &str = r"AppEvents\Schemes\Apps\.Default";

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", default)]
#[derive(Default)]
pub struct Toast {
    pub id: String,
    /// App display name.
    pub app: String,
    /// shell:AppsFolder target (icon and click).
    pub target: String,
    pub title: String,
    pub body: String,
    pub source: String,
    /// Unix ms when it arrived.
    pub at: u64,
}

/// Original values: app id → ShowBanner (None = wasn't set); sound event → its .Current file.
#[derive(Serialize, Deserialize, Default)]
struct Backup { banners: BTreeMap<String, Option<u32>>, sounds: BTreeMap<String, String> }

fn backup_path() -> std::path::PathBuf { crate::config::data_dir().join("notices-backup.json") }
fn load() -> Backup { std::fs::read(backup_path()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default() }
fn store(b: &Backup) { let _ = std::fs::write(backup_path(), serde_json::to_vec_pretty(b).unwrap_or_default()); }

fn get_dword(path: &str, name: &str) -> Option<u32> {
    let mut v = 0u32;
    let mut len = 4u32;
    unsafe { RegGetValueW(HKEY_CURRENT_USER, &HSTRING::from(path), &HSTRING::from(name), RRF_RT_REG_DWORD, None, Some(&mut v as *mut _ as _), Some(&mut len)) }.is_ok().then_some(v)
}
fn set_dword(path: &str, name: &str, v: u32) {
    unsafe { let _ = RegSetKeyValueW(HKEY_CURRENT_USER, &HSTRING::from(path), &HSTRING::from(name), REG_DWORD.0, Some(&v as *const _ as _), 4); }
}
fn delete_value(path: &str, name: &str) {
    unsafe { let _ = RegDeleteKeyValueW(HKEY_CURRENT_USER, &HSTRING::from(path), &HSTRING::from(name)); }
}
fn set_default_sz(path: &str, s: &str) {
    let w: Vec<u16> = s.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe { let _ = RegSetKeyValueW(HKEY_CURRENT_USER, &HSTRING::from(path), None, REG_SZ.0, Some(w.as_ptr() as _), (w.len() * 2) as u32); }
}

/// The apps Windows has notification settings for, plus any seen in the database.
fn known_apps(extra: &[String]) -> Vec<String> {
    let mut out: Vec<String> = extra.to_vec();
    unsafe {
        let mut key = HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, &HSTRING::from(SETTINGS), None, KEY_READ, &mut key).is_ok() {
            let mut i = 0;
            loop {
                let mut name = [0u16; 512];
                let mut len = name.len() as u32;
                if RegEnumKeyExW(key, i, Some(windows::core::PWSTR(name.as_mut_ptr())), &mut len, None, None, None, None).is_err() { break }
                out.push(String::from_utf16_lossy(&name[..len as usize]));
                i += 1;
            }
            let _ = RegCloseKey(key);
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Banners off for these apps (saving what they were).
fn suppress(apps: &[String], b: &mut Backup) -> bool {
    let mut changed = false;
    for a in apps {
        if b.banners.contains_key(a) { continue }
        let path = format!(r"{SETTINGS}\{a}");
        b.banners.insert(a.clone(), get_dword(&path, "ShowBanner"));
        set_dword(&path, "ShowBanner", 0);
        changed = true;
    }
    changed
}

fn sound_events() -> Vec<String> {
    let mut out = vec![];
    unsafe {
        let mut key = HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, &HSTRING::from(SOUNDS), None, KEY_READ, &mut key).is_ok() {
            let mut i = 0;
            loop {
                let mut name = [0u16; 256];
                let mut len = name.len() as u32;
                if RegEnumKeyExW(key, i, Some(windows::core::PWSTR(name.as_mut_ptr())), &mut len, None, None, None, None).is_err() { break }
                let n = String::from_utf16_lossy(&name[..len as usize]);
                if n.starts_with("Notification.") { out.push(n) }
                i += 1;
            }
            let _ = RegCloseKey(key);
        }
    }
    out
}

fn mute_sounds(b: &mut Backup) -> bool {
    let mut changed = false;
    for ev in sound_events() {
        if b.sounds.contains_key(&ev) { continue }
        let path = format!(r"{SOUNDS}\{ev}\.Current");
        b.sounds.insert(ev, crate::util::reg_string(HKEY_CURRENT_USER, &path, "").unwrap_or_default());
        set_default_sz(&path, "");
        changed = true;
    }
    changed
}

fn restore_sounds(b: &mut Backup) {
    for (ev, file) in std::mem::take(&mut b.sounds) { set_default_sz(&format!(r"{SOUNDS}\{ev}\.Current"), &file) }
}

/// Put every app's banner switch and the sounds back the way they were.
pub fn restore() {
    let mut b = load();
    if b.banners.is_empty() && b.sounds.is_empty() { return }
    for (a, v) in std::mem::take(&mut b.banners) {
        let path = format!(r"{SETTINGS}\{a}");
        match v { Some(x) => set_dword(&path, "ShowBanner", x), None => delete_value(&path, "ShowBanner") }
    }
    restore_sounds(&mut b);
    store(&b);
    log::info!("notifications: Windows banners and sounds restored");
}

fn text_of(payload: &str) -> Vec<String> {
    let mut out = vec![];
    let mut rest = payload;
    while let Some(i) = rest.find("<text") {
        let after = &rest[i..];
        let Some(open_end) = after.find('>') else { break };
        if after[..open_end].ends_with('/') { rest = &after[open_end + 1..]; continue }
        let body = &after[open_end + 1..];
        let Some(close) = body.find("</text>") else { break };
        let t = body[..close].replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'");
        if !t.trim().is_empty() { out.push(t.trim().to_string()) }
        rest = &body[close..];
    }
    out
}

fn app_name(id: &str) -> String {
    let target = format!("shell:appsfolder\\{}", id.to_lowercase());
    crate::system::startapps::apps().into_iter().find(|a| a.target.to_lowercase() == target).map(|a| a.name)
        .unwrap_or_else(|| id.rsplit(['\\', '.', '!']).find(|s| !s.is_empty() && !s.eq_ignore_ascii_case("exe") && !s.eq_ignore_ascii_case("app")).unwrap_or(id).to_string())
}

pub fn start(app: AppHandle) {
    let _ = std::thread::Builder::new().name("notices".into()).spawn(move || {
        let db = std::env::var("LOCALAPPDATA").map(|l| std::path::PathBuf::from(l).join(r"Microsoft\Windows\Notifications\wpndatabase.db")).unwrap_or_default();
        let mut last: Option<i64> = None;
        let mut was_on = false;
        let mut was_sound = true;
        let mut tick = 0u32;
        // Off at start (or Strata ended before it could put things back): Windows' own settings.
        if !app.state::<crate::AppState>().store.get().notify_strata { restore() }
        loop {
            std::thread::sleep(Duration::from_millis(700));
            tick = tick.wrapping_add(1);
            let s = app.state::<crate::AppState>().store.get();
            // Smart: a drive almost full (checked every ~30 minutes, at most once a day per drive).
            if s.notify_smart && tick % 2570 == 5 { low_disk(&app) }
            if !s.notify_strata {
                if was_on { restore(); was_on = false; last = None; }
                continue;
            }
            let mut b = load();
            if !was_on || tick % 40 == 0 {
                // New apps get notification settings when they first notify: catch them regularly.
                let changed = suppress(&known_apps(&[]), &mut b);
                if changed { store(&b) }
                was_on = true;
            }
            if s.notify_sound != was_sound || b.sounds.is_empty() != s.notify_sound {
                if s.notify_sound { restore_sounds(&mut b) } else { mute_sounds(&mut b); }
                store(&b);
                was_sound = s.notify_sound;
            }
            let Ok(c) = rusqlite::Connection::open_with_flags(&db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX) else { continue };
            let max: i64 = c.query_row("select coalesce(max(Id), 0) from Notification", [], |r| r.get(0)).unwrap_or(0);
            let from = match last { Some(l) => l, None => { last = Some(max); continue } };
            if max <= from { continue }
            let rows: Vec<(i64, String, Vec<u8>)> = c.prepare("select n.Id, h.PrimaryId, n.Payload from Notification n join NotificationHandler h on h.RecordId = n.HandlerId where n.Id > ?1 and n.Type = 'toast' order by n.Id")
                .and_then(|mut st| st.query_map([from], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?.collect())
                .unwrap_or_default();
            last = Some(max);
            let mut fresh = vec![];
            for (id, handler, payload) in rows {
                fresh.push(handler.clone());
                let texts = text_of(&String::from_utf8_lossy(&payload));
                if texts.is_empty() { continue }
                let toast = Toast {
                    id: format!("win-{id}"), app: app_name(&handler), target: format!(r"shell:AppsFolder\{handler}"),
                    title: texts[0].clone(), body: texts[1..].join("\n"), source: "windows".into(), at: now_ms(),
                };
                crate::toasts::push(&app, toast);
            }
            // An app notifying for the first time: its banner was shown this once; not the next.
            if suppress(&fresh, &mut b) { store(&b) }
        }
    });
}

static DISK_TOLD: std::sync::Mutex<Option<(String, std::collections::HashSet<char>)>> = std::sync::Mutex::new(None);

fn low_disk(app: &AppHandle) {
    use windows::Win32::Storage::FileSystem::{GetDiskFreeSpaceExW, GetDriveTypeW};
    let today = chrono_day();
    let mut told = DISK_TOLD.lock().unwrap();
    if told.as_ref().is_none_or(|(d, _)| *d != today) { *told = Some((today, Default::default())) }
    let seen = &mut told.as_mut().unwrap().1;
    for letter in 'C'..='Z' {
        let root = HSTRING::from(format!("{letter}:\\"));
        if unsafe { GetDriveTypeW(&root) } != 3 { continue } // fixed disks only
        let (mut free, mut total) = (0u64, 0u64);
        if unsafe { GetDiskFreeSpaceExW(&root, None, Some(&mut total), Some(&mut free)) }.is_err() || total == 0 { continue }
        if (free as f64) / (total as f64) < 0.10 && seen.insert(letter) {
            crate::toasts::push(app, Toast {
                id: format!("disk-{letter}-{}", now_ms() / 86_400_000), app: "Storage".into(), target: String::new(),
                title: format!("Drive {letter}: is almost full"),
                body: format!("{:.1} GB free of {:.0} GB. The Storage widget shows what fills it.", free as f64 / 1e9, total as f64 / 1e9),
                source: "strata".into(), at: now_ms(),
            });
        }
    }
}
fn chrono_day() -> String { format!("{}", now_ms() / 86_400_000) }

fn now_ms() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0) }

/// Strata's own notifications (smart ones), newest last; Windows' stay in Windows' database.
static OWN: std::sync::Mutex<Vec<Toast>> = std::sync::Mutex::new(Vec::new());
pub fn remember(t: &Toast) {
    if t.source == "windows" { return }
    let mut o = OWN.lock().unwrap();
    o.retain(|x| x.id != t.id);
    o.push(Toast { at: if t.at == 0 { now_ms() } else { t.at }, ..t.clone() });
    let n = o.len();
    if n > 50 { o.drain(..n - 50); }
}
pub fn clear_own() { OWN.lock().unwrap().clear() }

/// Notifications cleared in Strata that Windows wouldn't remove (no tag to name them by): hidden from
/// Strata's list. Kept in dismissed.json (the newest 1000).
fn dismissed_path() -> std::path::PathBuf { crate::config::data_dir().join("notices-dismissed.json") }
fn dismissed() -> std::collections::HashSet<String> {
    std::fs::read(dismissed_path()).ok().and_then(|b| serde_json::from_slice::<Vec<String>>(&b).ok()).unwrap_or_default().into_iter().collect()
}
fn add_dismissed(ids: &[String]) {
    let mut v: Vec<String> = std::fs::read(dismissed_path()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    v.extend(ids.iter().cloned());
    let n = v.len();
    if n > 1000 { v.drain(..n - 1000); }
    let _ = std::fs::write(dismissed_path(), serde_json::to_vec(&v).unwrap_or_default());
}

fn db_conn() -> Option<rusqlite::Connection> {
    let db = std::env::var("LOCALAPPDATA").map(|l| std::path::PathBuf::from(l).join(r"Microsoft\Windows\Notifications\wpndatabase.db")).ok()?;
    rusqlite::Connection::open_with_flags(&db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX).ok()
}

/// Clear notifications from the Notifications widget: Windows removes them from its own list too when
/// it can (Strata asks the way apps do: by tag, or all of an app's); the rest are hidden in Strata.
pub fn dismiss(ids: &[String], all: bool) {
    use windows::UI::Notifications::ToastNotificationManager;
    let hist = ToastNotificationManager::History().ok();
    {
        let mut own = OWN.lock().unwrap();
        if all { own.clear() } else { own.retain(|t| !ids.contains(&t.id)) }
    }
    let Some(c) = db_conn() else { return };
    let rows: Vec<(i64, String, String, String)> = c.prepare("select n.Id, h.PrimaryId, coalesce(n.Tag, ''), coalesce(n.\"Group\", '') from Notification n join NotificationHandler h on h.RecordId = n.HandlerId where n.Type = 'toast'")
        .and_then(|mut st| st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?.collect())
        .unwrap_or_default();
    let mut hide = vec![];
    let mut cleared_apps = std::collections::HashSet::new();
    for (id, app, tag, group) in rows {
        let key = format!("win-{id}");
        if !all && !ids.contains(&key) { continue }
        if let Some(h) = &hist {
            if all {
                if cleared_apps.insert(app.clone()) { let _ = h.ClearWithId(&HSTRING::from(&app)); }
            } else if !tag.is_empty() {
                let _ = h.RemoveGroupedTagWithId(&HSTRING::from(&tag), &HSTRING::from(&group), &HSTRING::from(&app));
            }
        }
        hide.push(key);
    }
    // Whatever Windows kept stays out of Strata's list.
    add_dismissed(&hide);
}

/// Everything in Windows' notification list plus Strata's own, newest first (Notifications widget).
pub fn history() -> Vec<Toast> {
    let mut out: Vec<Toast> = OWN.lock().unwrap().clone();
    let db = std::env::var("LOCALAPPDATA").map(|l| std::path::PathBuf::from(l).join(r"Microsoft\Windows\Notifications\wpndatabase.db")).unwrap_or_default();
    if let Ok(c) = rusqlite::Connection::open_with_flags(&db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX) {
        let rows: Vec<(i64, String, Vec<u8>, i64)> = c.prepare("select n.Id, h.PrimaryId, n.Payload, n.ArrivalTime from Notification n join NotificationHandler h on h.RecordId = n.HandlerId where n.Type = 'toast' order by n.ArrivalTime desc limit 60")
            .and_then(|mut st| st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?.collect())
            .unwrap_or_default();
        for (id, handler, payload, arrival) in rows {
            let texts = text_of(&String::from_utf8_lossy(&payload));
            if texts.is_empty() { continue }
            // FILETIME (100 ns since 1601) -> Unix ms.
            let at = (arrival / 10_000 - 11_644_473_600_000).max(0) as u64;
            out.push(Toast { id: format!("win-{id}"), app: app_name(&handler), target: format!(r"shell:AppsFolder\{handler}"), title: texts[0].clone(), body: texts[1..].join("\n"), source: "windows".into(), at });
        }
    }
    let hidden = dismissed();
    out.retain(|t| !hidden.contains(&t.id));
    out.sort_by(|a, b| b.at.cmp(&a.at));
    out
}

/// Emit a toast from Rust code (updates, disk space, …).
pub fn emit(app: &AppHandle, t: Toast) { let _ = app.emit("toast:push", t); }
