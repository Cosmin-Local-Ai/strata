//! Wallpaper Gallery backend (brief §6): Wallpaper Engine items + local pictures, applying either kind
//! per monitor, and seamless hand-off between them.
//!
//! * WE items: `<library>\steamapps\workshop\content\431960\<id>\project.json` and WE's own
//!   `projects\myprojects` / `projects\defaultprojects`. Presets/dependency-only items (no `type`) are
//!   skipped. Types are normalised to lowercase (WE writes both "scene" and "Scene").
//! * Apply WE: `wallpaper64.exe -control openWallpaper -file <project.json> -monitor N` (documented
//!   CLI). If WE isn't running it is started first (the CLI needs a running instance).
//! * Apply a picture: IDesktopWallpaper::SetWallpaper for that monitor *first*, then
//!   `-control closeWallpaper -monitor N` so WE stops covering it: no black frame, no stale wallpaper.
//! * Monitor index N: WE's `MonitorN` and IDesktopWallpaper's order matched on this PC (INFERRED, see
//!   glass/wallpaper.rs).

use serde::Serialize;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_MULTITHREADED};
use windows::Win32::UI::Shell::{DesktopWallpaper, IDesktopWallpaper};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeItem {
    pub id: String,
    pub title: String,
    /// scene | video | web | application
    pub kind: String,
    pub preview: String,
    /// What to pass to openWallpaper: project.json (scene/web/app) or the media file (video).
    pub file: String,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GalleryState {
    pub we_installed: bool,
    pub we_running: bool,
    pub items: Vec<WeItem>,
    /// Current WE wallpaper file per WE monitor index ("" when none).
    pub we_current: Vec<String>,
    /// Current Windows wallpaper path per monitor.
    pub win_current: Vec<String>,
    pub monitors: Vec<String>,
    pub playlists: Vec<String>,
}

fn read_item(dir: &Path) -> Option<WeItem> {
    let text = std::fs::read_to_string(dir.join("project.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let kind = v.get("type")?.as_str()?.to_lowercase();
    let title = v.get("title").and_then(|t| t.as_str()).unwrap_or("").to_string();
    let preview = v.get("preview").and_then(|p| p.as_str()).map(|p| dir.join(p)).filter(|p| p.exists())?;
    let file = if kind == "video" {
        dir.join(v.get("file").and_then(|f| f.as_str()).unwrap_or(""))
    } else {
        dir.join("project.json")
    };
    let tags = v.get("tags").and_then(|t| t.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
    Some(WeItem {
        id: dir.file_name()?.to_string_lossy().to_string(),
        title,
        kind,
        preview: preview.to_string_lossy().to_string(),
        file: file.to_string_lossy().to_string(),
        tags,
    })
}

pub fn items() -> Vec<WeItem> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    for lib in super::steam_libraries() {
        dirs.push(lib.join(r"steamapps\workshop\content\431960"));
    }
    if let Some(inst) = super::install_dir() {
        dirs.push(inst.join(r"projects\myprojects"));
        dirs.push(inst.join(r"projects\defaultprojects"));
    }
    let mut out = Vec::new();
    for d in dirs {
        if let Ok(rd) = std::fs::read_dir(&d) {
            for e in rd.flatten() {
                if let Some(it) = read_item(&e.path()) {
                    out.push(it);
                }
            }
        }
    }
    let mut seen = std::collections::HashSet::new();
    out.retain(|i| seen.insert(i.file.to_lowercase()));
    out.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    out
}

/// Saved Wallpaper Engine playlists (names), read-only from config.json.
pub fn playlists() -> Vec<String> {
    let Some(p) = super::config_path() else { return vec![] };
    let Ok(text) = std::fs::read_to_string(p) else { return vec![] };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else { return vec![] };
    let mut out = vec![];
    for (_, user) in v.as_object().into_iter().flatten() {
        if let Some(list) = user.pointer("/general/playlists").and_then(|l| l.as_array()) {
            out.extend(list.iter().filter_map(|x| x.get("name").and_then(|n| n.as_str()).map(String::from)));
        }
    }
    out
}

/// Documented WE controls: play | pause | stop | mute | unmute | next | close (next/close per monitor).
pub fn control(action: &str, monitor: Option<usize>) -> Result<(), String> {
    let m = monitor.map(|m| m.to_string());
    let mut args: Vec<&str> = match action {
        "play" => vec!["play"], "pause" => vec!["pause"], "stop" => vec!["stop"],
        "mute" => vec!["mute"], "unmute" => vec!["unmute"],
        "next" => vec!["nextWallpaper"], "close" => vec!["closeWallpaper"],
        _ => return Err(format!("unknown action {action}")),
    };
    if matches!(action, "next" | "close") {
        if let Some(m) = &m { args.extend_from_slice(&["-monitor", m]); }
    }
    we_control(&args)
}

pub fn open_playlist(name: &str, monitor: Option<usize>) -> Result<(), String> {
    let m = monitor.map(|m| m.to_string());
    let mut args = vec!["openPlaylist", "-playlist", name];
    if let Some(m) = &m { args.extend_from_slice(&["-monitor", m]); }
    we_control(&args)
}

fn we_exe() -> Option<PathBuf> {
    super::install_dir().map(|d| d.join("wallpaper64.exe")).filter(|p| p.exists())
}

fn we_running() -> bool {
    crate::util::process_running(&["wallpaper64.exe", "wallpaper32.exe"])
}

fn we_control(args: &[&str]) -> Result<(), String> {
    let exe = we_exe().ok_or("Wallpaper Engine isn't installed")?;
    let mut a = vec!["-control"];
    a.extend_from_slice(args);
    std::process::Command::new(exe).args(&a).creation_flags(CREATE_NO_WINDOW).status().map_err(|e| e.to_string())?;
    Ok(())
}

/// Current WE file per monitor from config.json (getWallpaper prints nothing when piped, D-009).
fn we_current() -> Vec<String> {
    let Some(p) = super::config_path() else { return vec![] };
    let Ok(text) = std::fs::read_to_string(p) else { return vec![] };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else { return vec![] };
    for (_, user) in v.as_object().into_iter().flatten() {
        if let Some(sel) = user.pointer("/general/wallpaperconfig/selectedwallpapers").and_then(|s| s.as_object()) {
            let mut out = vec![String::new(); sel.len()];
            for (k, m) in sel {
                if let Some(i) = k.strip_prefix("Monitor").and_then(|n| n.parse::<usize>().ok()) {
                    if i < out.len() {
                        out[i] = m.get("file").and_then(|f| f.as_str()).unwrap_or("").to_string();
                    }
                }
            }
            return out;
        }
    }
    vec![]
}

fn desktop_wallpaper() -> Option<IDesktopWallpaper> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        CoCreateInstance(&DesktopWallpaper, None, CLSCTX_ALL).ok()
    }
}

fn pw(p: windows::core::PWSTR) -> String {
    let s = unsafe { p.to_string().unwrap_or_default() };
    unsafe { CoTaskMemFree(Some(p.0 as _)) };
    s
}

/// (monitor device ids, current wallpaper per monitor)
fn windows_monitors() -> (Vec<String>, Vec<String>) {
    let mut ids = vec![];
    let mut cur = vec![];
    if let Some(dw) = desktop_wallpaper() {
        unsafe {
            let n = dw.GetMonitorDevicePathCount().unwrap_or(0);
            for i in 0..n {
                if let Ok(id) = dw.GetMonitorDevicePathAt(i) {
                    let id = pw(id);
                    let h = HSTRING::from(id.as_str());
                    cur.push(dw.GetWallpaper(PCWSTR(h.as_ptr())).map(pw).unwrap_or_default());
                    ids.push(id);
                }
            }
        }
    }
    (ids, cur)
}

pub fn state() -> GalleryState {
    let (ids, win_current) = windows_monitors();
    GalleryState {
        we_installed: we_exe().is_some(),
        we_running: we_running(),
        items: items(),
        we_current: we_current(),
        win_current,
        monitors: ids,
        playlists: playlists(),
    }
}

/// Show a Wallpaper Engine item on monitor N (None = every monitor). Starts WE if needed.
pub fn apply_we(file: &str, monitor: Option<usize>) -> Result<(), String> {
    let exe = we_exe().ok_or("Wallpaper Engine isn't installed")?;
    if !we_running() {
        std::process::Command::new(&exe).creation_flags(CREATE_NO_WINDOW).spawn().map_err(|e| e.to_string())?;
        for _ in 0..40 {
            std::thread::sleep(std::time::Duration::from_millis(250));
            if we_running() {
                std::thread::sleep(std::time::Duration::from_millis(1500));
                break;
            }
        }
    }
    let m = monitor.map(|m| m.to_string());
    let mut args = vec!["openWallpaper", "-file", file];
    if let Some(m) = &m {
        args.extend_from_slice(&["-monitor", m]);
    }
    we_control(&args)
}

/// Show a picture on monitor N (None = every monitor): Windows wallpaper first, then close WE there.
pub fn apply_picture(path: &str, monitor: Option<usize>) -> Result<(), String> {
    let dw = desktop_wallpaper().ok_or("Windows wallpaper API unavailable")?;
    let (ids, _) = windows_monitors();
    let file = HSTRING::from(path);
    unsafe {
        match monitor {
            Some(i) => {
                let id = ids.get(i).ok_or("no such monitor")?;
                let h = HSTRING::from(id.as_str());
                dw.SetWallpaper(PCWSTR(h.as_ptr()), &file).map_err(|e| e.to_string())?;
            }
            None => dw.SetWallpaper(PCWSTR::null(), &file).map_err(|e| e.to_string())?,
        }
    }
    if we_running() {
        let m = monitor.map(|m| m.to_string());
        let mut args = vec!["closeWallpaper"];
        if let Some(m) = &m {
            args.extend_from_slice(&["-monitor", m]);
        }
        we_control(&args)?;
    }
    Ok(())
}

/// Pictures in a folder (non-recursive), newest first, landscape not enforced (user's own files).
pub fn pictures(folder: &str) -> Vec<String> {
    let dir = crate::system::expand_env(folder);
    let mut v: Vec<(std::time::SystemTime, String)> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| {
            let n = e.file_name().to_string_lossy().to_lowercase();
            [".jpg", ".jpeg", ".png", ".bmp", ".webp", ".jxr"].iter().any(|x| n.ends_with(x))
        })
        .map(|e| (e.metadata().and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH), e.path().to_string_lossy().to_string()))
        .collect();
    v.sort_by(|a, b| b.0.cmp(&a.0));
    v.into_iter().map(|x| x.1).take(400).collect()
}

/// Background scheduler: every 10 minutes, change the wallpaper if the daily/weekly interval passed.
pub fn start_schedule(store: crate::config::Store) {
    std::thread::Builder::new().name("strata-wallpaper-schedule".into()).spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
        let s = store.get().wallpaper_schedule;
        let every = match s.mode.as_str() { "daily" => 86_400, "weekly" => 7 * 86_400, _ => 0 };
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        if every > 0 && now.saturating_sub(s.last) >= every {
            if let Err(e) = change_now(&s) {
                log::warn!("scheduled wallpaper change failed: {e}");
            } else {
                log::info!("scheduled wallpaper change ({}, {})", s.mode, s.source);
            }
            store.update(|x| x.wallpaper_schedule.last = now);
        }
        std::thread::sleep(std::time::Duration::from_secs(540));
    }).expect("schedule thread");
}

/// Pick a random wallpaper from the schedule's source and apply it.
pub fn change_now(s: &crate::config::WallpaperSchedule) -> Result<(), String> {
    let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(7);
    let mut rng = seed ^ 0x9E37_79B9_7F4A_7C15;
    let mut next = || { rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17; rng };
    let we: Vec<WeItem> = if s.source != "pictures" { items() } else { vec![] };
    let pics: Vec<String> = if s.source != "we" { pictures(&s.folder) } else { vec![] };
    let total = we.len() + pics.len();
    if total == 0 {
        return Err("nothing to choose from".into());
    }
    let i = (next() % total as u64) as usize;
    if i < we.len() { apply_we(&we[i].file, s.monitor) } else { apply_picture(&pics[i - we.len()], s.monitor) }
}
