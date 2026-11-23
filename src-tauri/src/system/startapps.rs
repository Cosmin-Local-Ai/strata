//! Start-style search sources: every app the Start menu lists (shell:AppsFolder, desktop and Store
//! apps alike) and, when Everything isn't running, a quick file list from your main folders.

use serde::Serialize;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StartApp {
    pub name: String,
    /// Launch / icon target: `shell:AppsFolder\<AUMID or path>`.
    pub target: String,
    /// The program it starts, when the shell knows it (desktop apps): lets the Project Launcher
    /// tell whether the app runs and close it.
    pub exe: String,
}

static APPS: Mutex<Option<(Instant, Vec<StartApp>)>> = Mutex::new(None);

fn enumerate_apps() -> windows::core::Result<Vec<StartApp>> {
    use windows::core::Interface;
    use windows::Win32::System::Com::*;
    use windows::Win32::UI::Shell::*;
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let folder: IShellItem = SHGetKnownFolderItem(&FOLDERID_AppsFolder, KF_FLAG_DEFAULT, None)?;
        let en: IEnumShellItems = folder.BindToHandler(None, &BHID_EnumItems)?;
        let mut out = vec![];
        loop {
            let mut items = [None];
            let mut got = 0u32;
            if en.Next(&mut items, Some(&mut got)).is_err() || got == 0 { break }
            let Some(it) = items[0].take() else { break };
            let name = it.GetDisplayName(SIGDN_NORMALDISPLAY).ok().and_then(|p| { let s = p.to_string().ok(); CoTaskMemFree(Some(p.0 as _)); s });
            let id = it.GetDisplayName(SIGDN_PARENTRELATIVEPARSING).ok().and_then(|p| { let s = p.to_string().ok(); CoTaskMemFree(Some(p.0 as _)); s });
            let exe = it.cast::<IShellItem2>().ok().and_then(|i2| i2.GetString(&windows::Win32::Storage::EnhancedStorage::PKEY_Link_TargetParsingPath).ok())
                .and_then(|p| { let s = p.to_string().ok(); CoTaskMemFree(Some(p.0 as _)); s }).unwrap_or_default();
            if let (Some(name), Some(id)) = (name, id) {
                // Skip uninstallers, readmes and help links the Start menu folder also carries.
                let l = name.to_lowercase();
                if l.contains("uninstall") || l.ends_with("readme") || l.contains("documentation") || l.contains("release notes") { continue }
                let exe = if exe.is_empty() && id.contains(':') && id.to_lowercase().ends_with(".exe") { id.clone() } else { exe };
                out.push(StartApp { name, target: format!(r"shell:AppsFolder\{id}"), exe });
            }
        }
        out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        out.dedup_by(|a, b| a.name.eq_ignore_ascii_case(&b.name));
        Ok(out)
    }
}

/// Installed apps, cached for 2 minutes (run off the UI thread).
pub fn apps() -> Vec<StartApp> {
    if let Some((t, v)) = APPS.lock().unwrap().as_ref() {
        if t.elapsed() < Duration::from_secs(120) { return v.clone() }
    }
    let v = std::thread::spawn(|| enumerate_apps().unwrap_or_default()).join().unwrap_or_default();
    *APPS.lock().unwrap() = Some((Instant::now(), v.clone()));
    v
}

/// A program inside a Store / MSIX package (`...\WindowsApps\Claude_1.2.3.0_x64__pubid\app\claude.exe`)
/// -> that package's Start menu entry (`shell:AppsFolder\Claude_pubid!Claude`). The versioned folder
/// disappears when the app updates and can't be started directly anyway; the entry always works.
pub fn packaged_target(path: &str) -> Option<String> {
    let l = path.to_lowercase().replace('/', "\\");
    let i = l.find("\\windowsapps\\")?;
    let folder = l[i + "\\windowsapps\\".len()..].split('\\').next()?;
    let (name, publisher) = (folder.split('_').next()?, folder.rsplit('_').next()?);
    if name.is_empty() || publisher.is_empty() || name == folder { return None }
    let prefix = format!("shell:appsfolder\\{name}_{publisher}!");
    let file = l.rsplit('\\').next().unwrap_or("").trim_end_matches(".exe").to_string();
    let all = apps();
    let mine: Vec<&StartApp> = all.iter().filter(|a| a.target.to_lowercase().starts_with(&prefix)).collect();
    mine.iter().find(|a| a.name.to_lowercase().contains(&file)).or(mine.first()).map(|a| a.target.clone())
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileHit { pub path: String, pub name: String, pub folder: bool, pub modified: u64 }

static FILES: Mutex<Option<(Instant, Vec<FileHit>)>> = Mutex::new(None);

fn walk(dir: &std::path::Path, depth: usize, out: &mut Vec<FileHit>) {
    if out.len() > 20_000 { return }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name.eq_ignore_ascii_case("desktop.ini") || name == "node_modules" { continue }
        let Ok(m) = e.metadata() else { continue };
        let modified = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis() as u64).unwrap_or(0);
        out.push(FileHit { path: e.path().to_string_lossy().into(), name: name.clone(), folder: m.is_dir(), modified });
        if m.is_dir() && depth > 0 && !m.file_type().is_symlink() { walk(&e.path(), depth - 1, out) }
    }
}

/// Files and folders whose name contains `query` (all words), from Desktop, Documents, Downloads,
/// Pictures (3 levels) and recent items; newest first. The list is cached for a minute.
pub fn files(query: &str, max: usize) -> Vec<FileHit> {
    let fresh = FILES.lock().unwrap().as_ref().is_some_and(|(t, _)| t.elapsed() < Duration::from_secs(60));
    if !fresh {
        let mut v = vec![];
        let home = std::env::var("USERPROFILE").unwrap_or_default();
        for sub in ["Desktop", "Documents", "Downloads", "Pictures", "OneDrive\\Desktop", "OneDrive\\Documents"] {
            walk(&std::path::Path::new(&home).join(sub), 2, &mut v);
        }
        if let Ok(recent) = crate::local::list_folder("recent", 400) {
            v.extend(recent.into_iter().map(|e| FileHit { path: e.path, name: e.name, folder: e.dir, modified: e.modified }));
        }
        *FILES.lock().unwrap() = Some((Instant::now(), v));
    }
    let words: Vec<String> = query.to_lowercase().split_whitespace().map(String::from).collect();
    if words.is_empty() { return vec![] }
    let g = FILES.lock().unwrap();
    let mut hits: Vec<FileHit> = g.as_ref().map(|(_, v)| v.iter().filter(|f| { let n = f.name.to_lowercase(); words.iter().all(|w| n.contains(w.as_str())) }).cloned().collect()).unwrap_or_default();
    hits.sort_by(|a, b| b.modified.cmp(&a.modified));
    hits.dedup_by(|a, b| a.name.eq_ignore_ascii_case(&b.name) && a.folder == b.folder);
    hits.truncate(max);
    hits
}
