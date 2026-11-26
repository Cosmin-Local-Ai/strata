//! System actions used by tiles: launching, power, Start / Win+X, uptime.

pub mod audio;
pub mod audio_ext;
pub mod everything;
pub mod icons;
pub mod media;
pub mod misc;
pub mod startapps;
pub mod diskscan;
pub mod stats;
pub mod net;
pub mod netinfo;
pub mod previews;
pub mod thumbs;
pub mod tray;
pub mod winlist;

use std::os::windows::process::CommandExt;
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_LWIN, VK_X};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Open a file, shortcut, exe, folder, URL or `shell:AppsFolder\<AUMID>` the way Explorer would.
pub fn launch(target: &str, args: Option<&str>) -> Result<(), String> {
    // Logged so a stray window (e.g. a console flashing open) can be traced to what started it.
    log::info!("launch {target}{}", args.map(|a| format!(" {a}")).unwrap_or_default());
    let t = HSTRING::from(resolve_app_path(target));
    let a = args.map(HSTRING::from);
    let r = unsafe {
        ShellExecuteW(None, w!("open"), &t, a.as_ref().map(|a| PCWSTR(a.as_ptr())).unwrap_or(PCWSTR::null()), PCWSTR::null(), SW_SHOWNORMAL)
    };
    if r.0 as isize > 32 {
        Ok(())
    } else {
        Err(format!("Windows could not open {target} (code {})", r.0 as isize))
    }
}

/// Expand %VARS% and repair paths into versioned app folders. Squirrel-installed apps (Discord,
/// Slack, GitHub Desktop, ...) live in `<root>\app-1.2.3\Name.exe` and delete that folder on every
/// update, so a pinned path goes stale within days. Prefer the stable launcher `<root>\Name.exe`
/// when the app ships one, otherwise the newest `app-*` folder that has the exe.
pub fn resolve_app_path(path: &str) -> String {
    let p = expand_env(path);
    if p.to_lowercase().contains("\\windowsapps\\") {
        if let Some(t) = startapps::packaged_target(&p) { return t }
    }
    let pb = std::path::Path::new(&p);
    let (Some(file), Some(dir)) = (pb.file_name(), pb.parent()) else { return p };
    let Some(dir_name) = dir.file_name().map(|d| d.to_string_lossy().to_lowercase()) else { return p };
    if !dir_name.starts_with("app-") || !dir_name[4..].starts_with(|c: char| c.is_ascii_digit()) {
        return p;
    }
    let Some(root) = dir.parent() else { return p };
    let stub = root.join(file);
    if stub.is_file() {
        return stub.to_string_lossy().into_owned();
    }
    if pb.is_file() {
        return p;
    }
    let ver = |n: &str| n[4..].split('.').map(|x| x.parse::<u64>().unwrap_or(0)).collect::<Vec<_>>();
    let newest = std::fs::read_dir(root).ok().into_iter().flatten().flatten()
        .filter_map(|e| { let n = e.file_name().to_string_lossy().to_lowercase(); (n.starts_with("app-") && e.path().join(file).is_file()).then(|| (ver(&n), e.path().join(file))) })
        .max_by(|a, b| a.0.cmp(&b.0));
    newest.map(|(_, f)| f.to_string_lossy().into_owned()).unwrap_or(p)
}

pub fn expand_env(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find('%') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        if let Some(j) = after.find('%') {
            let name = &after[..j];
            match std::env::var(name) {
                Ok(v) => out.push_str(&v),
                Err(_) => {
                    out.push('%');
                    out.push_str(name);
                    out.push('%');
                }
            }
            rest = &after[j + 1..];
        } else {
            out.push('%');
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

fn key(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: vk, dwFlags: if up { KEYEVENTF_KEYUP } else { Default::default() }, ..Default::default() } },
    }
}

/// Open the Start menu (same as pressing the Windows key).
pub fn open_start() {
    let inputs = [key(VK_LWIN, false), key(VK_LWIN, true)];
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

/// Open Windows' quick-link menu (Win+X), the Start tile's right-click.
pub fn open_winx() {
    let inputs = [key(VK_LWIN, false), key(VK_X, false), key(VK_X, true), key(VK_LWIN, true)];
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

/// Power actions: shut down, restart, sign out, hibernate, sleep, lock.
pub fn power(action: &str) -> Result<(), String> {
    let run = |args: &[&str]| {
        std::process::Command::new("shutdown").args(args).creation_flags(CREATE_NO_WINDOW).spawn().map(|_| ()).map_err(|e| e.to_string())
    };
    match action {
        "lock" => unsafe { windows::Win32::System::Shutdown::LockWorkStation().map_err(|e| e.to_string()) },
        "signout" => run(&["/l"]),
        "sleep" => unsafe {
            // Sleep, not hibernate; wake events allowed.
            if windows::Win32::System::Power::SetSuspendState(false, false, false) { Ok(()) } else { Err("sleep was refused by Windows".into()) }
        },
        "hibernate" => run(&["/h"]),
        "restart" => run(&["/r", "/t", "0"]),
        "shutdown" => run(&["/s", "/hybrid", "/t", "0"]),
        _ => Err(format!("unknown power action {action}")),
    }
}

/// Run as administrator (UAC prompt).
pub fn launch_admin(target: &str) -> Result<(), String> {
    let t = HSTRING::from(expand_env(target));
    let r = unsafe { ShellExecuteW(None, w!("runas"), &t, PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL) };
    if r.0 as isize > 32 { Ok(()) } else { Err("cancelled or failed".into()) }
}

/// Show a file in Explorer, selected.
pub fn reveal(target: &str) -> Result<(), String> {
    std::process::Command::new("explorer.exe").arg(format!("/select,{}", expand_env(target))).spawn().map(|_| ()).map_err(|e| e.to_string())
}

/// Seconds since boot.
pub fn uptime() -> u64 {
    unsafe { windows::Win32::System::SystemInformation::GetTickCount64() / 1000 }
}

/// Terminal widget: open a terminal running `command` (empty = just a prompt). Windows Terminal when
/// installed, else a console window. `shell`: "pwsh" | "powershell" | "cmd". PowerShell gets the
/// command base64-encoded (-EncodedCommand), so quotes, `;` and `&` reach it untouched.
pub fn terminal(command: &str, shell: &str, admin: bool, cwd: Option<&str>) -> Result<(), String> {
    log::info!("terminal ({shell}{}) {command}", if admin { ", admin" } else { "" });
    use base64::Engine;
    let ps_exe = if shell == "pwsh" { "pwsh.exe" } else { "powershell.exe" };
    let dir = cwd.map(expand_env).filter(|d| std::path::Path::new(d).is_dir())
        .unwrap_or_else(|| std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".into()));
    let encode = |c: &str| {
        let utf16: Vec<u8> = c.encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
        base64::engine::general_purpose::STANDARD.encode(utf16)
    };
    let run = |verb: PCWSTR, file: &str, params: &str, dir: &str| -> Result<(), String> {
        let r = unsafe { ShellExecuteW(None, verb, &HSTRING::from(file), &HSTRING::from(params), &HSTRING::from(dir), SW_SHOWNORMAL) };
        if r.0 as isize > 32 { Ok(()) } else { Err(format!("Could not open a terminal (code {})", r.0 as isize)) }
    };

    // As administrator: elevate the shell itself. Elevating wt.exe drops the command it was given (an
    // empty admin window), and that window then also swallowed later non-admin runs aimed at it.
    // Windows still shows the shell in Windows Terminal when that is the default terminal app. An
    // elevated process ignores the start folder, so the command changes to it first.
    if admin {
        return if shell == "cmd" {
            let cd = format!("cd /d \"{dir}\"");
            run(w!("runas"), "cmd.exe", &if command.trim().is_empty() { format!("/k {cd}") } else { format!("/k {cd} && {command}") }, &dir)
        } else {
            let cd = format!("Set-Location -LiteralPath '{}'", dir.replace('\'', "''"));
            let full = if command.trim().is_empty() { cd } else { format!("{cd}\n{command}") };
            run(w!("runas"), ps_exe, &format!("-NoLogo -NoExit -EncodedCommand {}", encode(&full)), &dir)
        };
    }

    let inner: String = match shell {
        "cmd" => if command.trim().is_empty() { "cmd.exe".into() } else { format!("cmd.exe /k {}", command) },
        _ => if command.trim().is_empty() { format!("{ps_exe} -NoLogo") } else { format!("{ps_exe} -NoLogo -NoExit -EncodedCommand {}", encode(command)) },
    };
    let wt = std::env::var("LOCALAPPDATA").map(|l| std::path::Path::new(&l).join(r"Microsoft\WindowsApps\wt.exe")).ok().filter(|p| p.exists());
    match wt {
        // Tabs in Strata's own Terminal window ("-w strata"): "-w 0" meant whichever window was used
        // last, and if that one runs as administrator a normal run can't reach it (the command was lost).
        // Windows Terminal treats `;` as its own separator: escape it (PowerShell's are base64, none).
        Some(w) => run(w!("open"), &w.to_string_lossy(), &format!("-w strata new-tab -d \"{}\" {}", if dir.ends_with('\\') { format!("{dir}.") } else { dir.clone() }, inner.replace(';', r"\;")), &dir),
        None => {
            let (exe, rest) = inner.split_once(' ').map(|(a, b)| (a.to_string(), b.to_string())).unwrap_or((inner.clone(), String::new()));
            run(w!("open"), &exe, &rest, &dir)
        }
    }
}
