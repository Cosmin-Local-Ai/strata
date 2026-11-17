//! Widget data files (launchpad, notes, todos, …) in %APPDATA%\Strata, allow-listed by name.

use std::path::PathBuf;

/// Read/write small JSON data files owned by widgets (allow-listed names only).
pub fn data_path(name: &str) -> Option<PathBuf> {
    const ALLOWED: &[&str] = &["launchpad", "notes", "todos", "timers", "calendars", "widget-state", "commands", "ai-history", "finance-cache", "automations", "calendar-events"];
    ALLOWED.contains(&name).then(|| crate::config::data_dir().join(format!("{name}.json")))
}
