//! Storage widget: what fills a drive. Sizes every top-level folder and file of a drive root (several
//! folders at once), sending progress as each one finishes. Reparse points (junctions, symlinks,
//! OneDrive placeholders' links) are not followed, so nothing is counted twice; folders Windows won't
//! let us read count as "Other" (used space minus everything found). Results are kept 5 minutes.

use serde::Serialize;
use std::collections::HashMap;
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScanItem { pub name: String, pub path: String, pub dir: bool, pub bytes: u64 }

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScanState {
    pub root: String,
    pub done: usize,
    pub total: usize,
    /// Largest first.
    pub items: Vec<ScanItem>,
    pub finished: bool,
    pub elapsed_ms: u64,
    /// Unix ms when the scan finished (0 while running).
    pub at: u64,
}

const REPARSE: u32 = 0x400;
static SCANS: Mutex<Option<HashMap<String, (Instant, ScanState)>>> = Mutex::new(None);

fn key(root: &str) -> String { root.trim_end_matches(['\\', '/']).to_uppercase() }

/// A finished scan of `root` younger than 5 minutes, or the one still running.
pub fn cached(root: &str) -> Option<ScanState> {
    let g = SCANS.lock().unwrap();
    let (t, s) = g.as_ref()?.get(&key(root))?;
    (!s.finished || t.elapsed() < Duration::from_secs(300)).then(|| s.clone())
}

fn size_of(dir: &Path) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let Ok(m) = e.metadata() else { continue };
            if m.file_attributes() & REPARSE != 0 { continue }
            if m.is_dir() { stack.push(e.path()) } else { total += m.len() }
        }
    }
    total
}

/// Start (or join) a scan of `root`; progress arrives as `storage:scan` events.
pub fn start(app: AppHandle, root: String, force: bool) {
    let k = key(&root);
    {
        let mut g = SCANS.lock().unwrap();
        let map = g.get_or_insert_with(HashMap::new);
        if let Some((t, s)) = map.get(&k) {
            if !s.finished || (!force && t.elapsed() < Duration::from_secs(300)) {
                let _ = app.emit("storage:scan", s.clone());
                return;
            }
        }
        map.insert(k.clone(), (Instant::now(), ScanState { root: root.clone(), ..Default::default() }));
    }
    std::thread::spawn(move || {
        let t0 = Instant::now();
        let base = PathBuf::from(if root.ends_with('\\') { root.clone() } else { format!("{root}\\") });
        let mut files = vec![];
        let mut dirs = vec![];
        for e in std::fs::read_dir(&base).into_iter().flatten().flatten() {
            let Ok(m) = e.metadata() else { continue };
            if m.file_attributes() & REPARSE != 0 { continue }
            let name = e.file_name().to_string_lossy().into_owned();
            if m.is_dir() { dirs.push((name, e.path())) } else { files.push(ScanItem { name, path: e.path().to_string_lossy().into(), dir: false, bytes: m.len() }) }
        }
        let total = dirs.len() + files.len();
        let state = Arc::new(Mutex::new(ScanState { root: root.clone(), done: files.len(), total, items: files, ..Default::default() }));
        let publish = |s: &ScanState| {
            let mut s = s.clone();
            s.items.sort_by(|a, b| b.bytes.cmp(&a.bytes));
            s.elapsed_ms = t0.elapsed().as_millis() as u64;
            if let Some(m) = SCANS.lock().unwrap().as_mut() { m.insert(k.clone(), (Instant::now(), s.clone())); }
            let _ = app.emit("storage:scan", s);
        };
        publish(&state.lock().unwrap());
        // Several folders at once: the disk queue stays full, small folders don't wait for Windows.
        let queue = Arc::new(Mutex::new(dirs));
        let workers: Vec<_> = (0..6).map(|_| {
            let (queue, state) = (queue.clone(), state.clone());
            std::thread::spawn(move || loop {
                let Some((name, path)) = queue.lock().unwrap().pop() else { return };
                let bytes = size_of(&path);
                let mut s = state.lock().unwrap();
                s.items.push(ScanItem { name, path: path.to_string_lossy().into(), dir: true, bytes });
                s.done += 1;
            })
        }).collect();
        // Progress twice a second until the workers are done.
        while workers.iter().any(|w| !w.is_finished()) {
            std::thread::sleep(Duration::from_millis(500));
            publish(&state.lock().unwrap());
        }
        let mut s = state.lock().unwrap().clone();
        s.finished = true;
        s.at = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
        log::info!("storage scan of {root}: {} items in {} ms", s.items.len(), t0.elapsed().as_millis());
        publish(&s);
    });
}
