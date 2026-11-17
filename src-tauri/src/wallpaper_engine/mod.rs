//! Wallpaper Engine detection (read-only). Control goes through WE's documented CLI only (D-009).

pub mod gallery;

use std::path::{Path, PathBuf};
use windows::Win32::System::Registry::HKEY_CURRENT_USER;

/// Steam library folders: the Steam install plus every `path` in steamapps/libraryfolders.vdf.
pub fn steam_libraries() -> Vec<PathBuf> {
    let mut libs = Vec::new();
    let Some(steam) = crate::util::reg_string(HKEY_CURRENT_USER, r"Software\Valve\Steam", "SteamPath") else { return libs };
    let steam = PathBuf::from(steam.replace('/', "\\"));
    libs.push(steam.clone());
    if let Ok(vdf) = std::fs::read_to_string(steam.join(r"steamapps\libraryfolders.vdf")) {
        for line in vdf.lines() {
            let t = line.trim();
            if let Some(rest) = t.strip_prefix("\"path\"") {
                let p = rest.trim().trim_matches('"').replace("\\\\", "\\");
                let p = PathBuf::from(p);
                let same = |a: &PathBuf| a.to_string_lossy().to_lowercase().trim_end_matches('\\') == p.to_string_lossy().to_lowercase().trim_end_matches('\\');
                if !libs.iter().any(same) {
                    libs.push(p);
                }
            }
        }
    }
    libs
}

pub fn install_dir() -> Option<PathBuf> {
    steam_libraries().into_iter().map(|l| l.join(r"steamapps\common\wallpaper_engine")).find(|p| p.join("wallpaper64.exe").exists() || p.join("wallpaper32.exe").exists())
}

pub fn config_path() -> Option<PathBuf> {
    install_dir().map(|d| d.join("config.json")).filter(|p| p.exists())
}

/// Preview image for a wallpaper file path as written in config.json (…\<item>\scene.pkg, …\x.mp4,
/// or …\project.json): read the item's project.json `preview`.
pub fn preview_for(file: &Path) -> Option<PathBuf> {
    let dir = if file.is_dir() { file.to_path_buf() } else { file.parent()?.to_path_buf() };
    let text = std::fs::read_to_string(dir.join("project.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let preview = v.get("preview")?.as_str()?;
    let p = dir.join(preview);
    p.exists().then_some(p)
}
