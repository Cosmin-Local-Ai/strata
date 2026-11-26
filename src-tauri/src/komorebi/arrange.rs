//! Arrange rules: put chosen apps in fixed places of a komorebi workspace, e.g. on workspace "I"
//! (UltrawideVerticalStack): the browser in the middle, Discord on the left, Spotify on the right.
//!
//! komorebi has no "this app goes in that column" rule, but its layouts fill containers in order
//! (UltrawideVerticalStack: 1st = middle, 2nd = left, the rest stack on the right) and `promote`
//! moves the focused window to the first slot. Promoting the apps in reverse order leaves them in
//! the listed order. It runs only when a new window appears on the workspace, never on a reorder
//! you make yourself, and gives focus back to the window you were using.

use super::{komorebic, KState};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ArrangeRule {
    /// Workspace name, e.g. "I".
    pub workspace: String,
    /// Places in layout order; each lists the exes that may take it (e.g. any browser).
    pub slots: Vec<Vec<String>>,
}

static SEEN: Mutex<Option<HashMap<(usize, usize), Vec<i64>>>> = Mutex::new(None);
static BUSY: AtomicBool = AtomicBool::new(false);

pub fn check(s: &KState, rules: &[ArrangeRule]) {
    if rules.is_empty() || !s.online || s.paused || BUSY.load(Ordering::SeqCst) {
        return;
    }
    let mut seen = SEEN.lock().unwrap();
    let seen = seen.get_or_insert_with(HashMap::new);
    for m in &s.monitors {
        for w in &m.workspaces {
            let Some(rule) = rules.iter().find(|r| r.workspace.eq_ignore_ascii_case(&w.name)) else { continue };
            let mut hwnds: Vec<i64> = w.windows.iter().map(|x| x.hwnd).collect();
            hwnds.sort();
            let before = seen.insert((m.index, w.index), hwnds.clone());
            // Only when one of the rule's apps was added (first sight counts: komorebi or Strata just
            // started). Other new windows may shift things, but re-shuffling for each one is annoying.
            let ruled = |exe: &str| rule.slots.iter().flatten().any(|a| a.eq_ignore_ascii_case(exe));
            let added = match &before {
                None => true,
                Some(b) => w.windows.iter().any(|x| !b.contains(&x.hwnd) && ruled(&x.exe)),
            };
            if !added || w.monocle || w.maximized || m.focused_workspace != w.index {
                continue;
            }
            // The exe (as komorebi spells it) that fills each slot, for the apps that are open.
            let wanted: Vec<String> = rule.slots.iter().filter_map(|alts| {
                w.containers.iter().find(|c| alts.iter().any(|a| a.eq_ignore_ascii_case(c))).cloned()
            }).collect();
            if wanted.len() < 2 {
                continue;
            }
            let in_place = w.containers.len() >= wanted.len()
                && w.containers.iter().zip(&wanted).all(|(c, want)| c.eq_ignore_ascii_case(want));
            if in_place {
                continue;
            }
            log::info!("komorebi arrange: workspace {} -> {:?}", w.name, wanted);
            BUSY.store(true, Ordering::SeqCst);
            std::thread::spawn(move || {
                // Let the new window finish appearing before moving things around.
                std::thread::sleep(Duration::from_millis(800));
                let back = unsafe { windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow() }.0 as i64;
                super::without_animation(|| for exe in wanted.iter().rev() {
                    let _ = komorebic(&["eager-focus", exe]);
                    // Promote acts on the focused window: wait until it really is this app's.
                    if !wait_foreground(exe) {
                        log::warn!("komorebi arrange: {exe} did not take focus; stopping");
                        break;
                    }
                    let _ = komorebic(&["promote"]);
                    std::thread::sleep(Duration::from_millis(150));
                });
                if back != 0 {
                    crate::system::winlist::activate(back, false, None);
                }
                std::thread::sleep(Duration::from_millis(400));
                BUSY.store(false, Ordering::SeqCst);
            });
            return;
        }
    }
}

/// Up to 1.5 s for the foreground window to belong to `exe`.
fn wait_foreground(exe: &str) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
    for _ in 0..30 {
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut pid)); }
        let path = crate::shell::gamemode::process_path(pid);
        if path.rsplit(['\\', '/']).next().is_some_and(|n| n.eq_ignore_ascii_case(exe)) {
            std::thread::sleep(Duration::from_millis(80));
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}
