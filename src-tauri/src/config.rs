//! Settings: a readable JSON file under %APPDATA%\Strata\settings.json (power users and backups),
//! always written by the app, never required to be hand-edited. Secrets never live here (Credential
//! Manager, see secrets.rs).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Edge {
    Top,
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskbarMode {
    /// Windows taskbar set to auto-hide while Strata runs (default).
    AutoHide,
    /// Windows taskbar fully hidden while Strata runs.
    Hidden,
    /// Strata does not touch the taskbar (also used when another taskbar tool is running).
    Leave,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DockStyle {
    Black,
    White,
    LiquidGlass,
    IosGlass,
    Midnight,
    Clear,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DisplayPref {
    pub enabled: bool,
    /// Friendly name at the time it was seen, for the Settings list.
    pub name: String,
}

impl Default for DisplayPref {
    fn default() -> Self {
        Self { enabled: false, name: String::new() }
    }
}

/// Automatic wallpaper change (brief §6: daily/weekly, from one source or all).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WallpaperSchedule {
    /// "off" | "daily" | "weekly"
    pub mode: String,
    /// "we" | "pictures" | "both"
    pub source: String,
    /// Screen index, or None for all screens.
    pub monitor: Option<usize>,
    pub folder: String,
    /// Unix seconds of the last automatic change.
    pub last: u64,
}

impl Default for WallpaperSchedule {
    fn default() -> Self {
        Self { mode: "off".into(), source: "both".into(), monitor: Some(0), folder: r"%USERPROFILE%\Pictures".into(), last: 0 }
    }
}

/// One widget instance on the dock.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockItem {
    pub uid: String,
    pub widget: String,
    #[serde(default)]
    pub options: serde_json::Value,
}

fn item(uid: &str, widget: &str, options: serde_json::Value) -> DockItem {
    DockItem { uid: uid.into(), widget: widget.into(), options }
}

/// Default dock for a new install: a useful set of everyday widgets (more in the Widgets picker).
pub fn default_items() -> Vec<DockItem> {
    use serde_json::json;
    vec![
        item("start", "start", json!({})),
        item("launchpad", "launchpad", json!({})),
        item("explorer", "app", json!({ "path": r"%WINDIR%\explorer.exe", "name": "File Explorer" })),
        item("apps", "apps", json!({})),
        item("tray", "tray", json!({})),
        item("media", "now-playing", json!({})),
        item("weather", "weather", json!({})),
        item("volume", "audio-output", json!({})),
        item("wifi", "wifi", json!({})),
        item("cpu", "cpu", json!({})),
        item("clock", "clock", json!({})),
        item("power", "power", json!({})),
    ]
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Widgets on the dock, left to right.
    pub items: Vec<DockItem>,
    pub edge: Edge,
    /// Dock body height in logical px (S/M/L presets map to 48/56/64; draggable 32..=120).
    pub dock_height: u32,
    /// Gap between dock and screen edge, logical px (0..=24).
    pub edge_distance: u32,
    pub padding: u32,
    pub spacing: u32,
    /// Floating pill (content width) vs full width.
    pub full_width: bool,
    /// Max width as a fraction of the monitor (pill mode); ultrawides need this.
    pub max_width_fraction: f32,
    pub style: DockStyle,
    pub accent: String,
    pub auto_hide: bool,
    pub show_on_desktop: bool,
    pub taskbar_mode: TaskbarMode,
    /// Docks per display, keyed by the display's device-interface path (stable across re-plugging).
    pub displays: BTreeMap<String, DisplayPref>,
    /// Opt-in live glass (D-003): captures the dock's monitor and hides the dock from recordings.
    pub live_glass: bool,
    pub live_glass_fps: u32,
    /// Apps that put Strata into GPU-yield mode while they use the GPU (D-011).
    pub yield_apps: Vec<String>,
    pub yield_for_steam_games: bool,
    pub lock_dock: bool,
    pub magnification: bool,
    pub magnification_scale: f32,
    pub hover_names: bool,
    /// Live window previews when hovering app icons (user request: on by default).
    pub window_previews: bool,
    pub wallpaper_schedule: WallpaperSchedule,
    /// Power panel background: "blur" (blurred desktop) or "dim".
    pub power_backdrop: String,
    /// The mouse wheel slides an overflowing dock (Ctrl+wheel goes to the widget).
    pub wheel_scrolls_dock: bool,
    /// Background update checks: "notify" (message on the dock), "auto" (install and restart) or "off".
    pub update_mode: String,
    /// komorebi arrange rules (komorebi/arrange.rs): apps kept in fixed places of a workspace.
    pub komorebi_arrange: Vec<crate::komorebi::arrange::ArrangeRule>,
    /// Strata shows notifications instead of Windows' pop-ups (notices.rs).
    pub notify_strata: bool,
    /// With Strata notifications: keep Windows' notification sounds.
    pub notify_sound: bool,
    /// Smart notifications from widgets (to-do times, AI limits, low disk space).
    pub notify_smart: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            items: default_items(),
            edge: Edge::Bottom,
            dock_height: 56,
            edge_distance: 5,
            padding: 8,
            spacing: 8,
            full_width: false,
            max_width_fraction: 1.0,
            style: DockStyle::LiquidGlass,
            accent: "#0a84ff".into(),
            auto_hide: false,
            show_on_desktop: false,
            taskbar_mode: TaskbarMode::AutoHide,
            displays: BTreeMap::new(),
            live_glass: false,
            live_glass_fps: 30,
            yield_apps: default_yield_apps(),
            yield_for_steam_games: true,
            lock_dock: false,
            magnification: false,
            magnification_scale: 1.35,
            hover_names: true,
            window_previews: true,
            wallpaper_schedule: WallpaperSchedule::default(),
            power_backdrop: "blur".into(),
            wheel_scrolls_dock: false,
            update_mode: "notify".into(),
            komorebi_arrange: vec![],
            notify_strata: false,
            notify_sound: true,
            notify_smart: true,
        }
    }
}

pub fn default_yield_apps() -> Vec<String> {
    [
        "ollama.exe:runner", "ollama_llama_server.exe", "llama-server.exe", "llama-cli.exe", "LM Studio.exe", "lms.exe",
        "koboldcpp.exe", "Jan.exe", "vllm.exe", "python.exe:comfyui", "python.exe:text-generation-webui",
        "python.exe:stable-diffusion", "obs64.exe:recording",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    let d = base.join("Strata");
    let _ = std::fs::create_dir_all(&d);
    d
}

pub fn local_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    // Not "Strata": that is the install folder; app data stays separate from program files.
    let d = base.join("local.strata.dock");
    let _ = std::fs::create_dir_all(&d);
    d
}

fn settings_path() -> PathBuf {
    data_dir().join("settings.json")
}

#[derive(Clone)]
pub struct Store(Arc<RwLock<Settings>>);

impl Store {
    pub fn load() -> Self {
        let s = std::fs::read_to_string(settings_path())
            .ok()
            .and_then(|t| serde_json::from_str::<Settings>(&t).map_err(|e| log::warn!("settings.json unreadable, using defaults: {e}")).ok())
            .unwrap_or_default();
        let mut s = s;
        // One-time migration: the old default width cap (60%) cut the dock short; grow to full width.
        if (s.max_width_fraction - 0.6).abs() < 1e-6 {
            s.max_width_fraction = 1.0;
        }
        Store(Arc::new(RwLock::new(s)))
    }

    pub fn get(&self) -> Settings {
        self.0.read().unwrap().clone()
    }

    pub fn update(&self, f: impl FnOnce(&mut Settings)) -> Settings {
        let mut g = self.0.write().unwrap();
        f(&mut g);
        let s = g.clone();
        drop(g);
        self.save(&s);
        s
    }

    fn save(&self, s: &Settings) {
        let path = settings_path();
        let tmp = path.with_extension("json.tmp");
        if let Ok(text) = serde_json::to_string_pretty(s) {
            // Write-then-rename so a crash never leaves a half-written settings file.
            if std::fs::write(&tmp, text).is_ok() {
                let _ = std::fs::rename(&tmp, &path);
            }
        }
    }
}
