//! Project Launcher: start, open and stop apps you run yourself (GitHub projects, self-hosted apps).
//!
//! A dock item holds a list of projects in its options (`projects: [...]`; an older single-project
//! item keeps its fields at the top level). Commands are read here from the saved settings by item
//! uid and project id: the page never sends a command line (D-033). Output of each start/stop goes to
//! %APPDATA%\Strata\projects\<id>.log. Without a stop command, Strata ends whatever listens on the
//! project's port (and its child processes).

use serde::{Deserialize, Serialize};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Project {
    pub id: String,
    pub name: String,
    /// Working folder for the commands (the cloned repository).
    pub folder: String,
    pub start: String,
    pub stop: String,
    pub url: String,
    pub port: Option<u16>,
    pub process: String,
    pub icon: String,
    /// GitHub address it was cloned from (informational).
    pub repo: String,
}

/// The project `pid` of dock item `uid` (legacy single-project items: their own fields, id "main").
pub fn find(s: &crate::config::Settings, uid: &str, pid: &str) -> Result<Project, String> {
    // Your list: projects.json (independent of the widget, see state.svelte.ts).
    if let Some(p) = std::fs::read_to_string(crate::config::data_dir().join("projects.json")).ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|v| v.get("list").and_then(|l| l.as_array()).cloned())
        .and_then(|l| l.into_iter().filter_map(|v| serde_json::from_value::<Project>(v).ok()).find(|p| p.id == pid))
    {
        return Ok(p);
    }
    // An older Strata kept them in the widget's options.
    let it = s.items.iter().find(|i| i.uid == uid && i.widget == "local-app").ok_or("This project is no longer in the list")?;
    let o = &it.options;
    if let Some(list) = o.get("projects").and_then(|v| v.as_array()) {
        return list.iter().filter_map(|v| serde_json::from_value::<Project>(v.clone()).ok()).find(|p| p.id == pid)
            .ok_or_else(|| "This project is no longer in the list".to_string());
    }
    let mut p: Project = serde_json::from_value(o.clone()).unwrap_or_default();
    if p.port.is_none() {
        p.port = o.get("port").and_then(|v| v.as_str()).and_then(|s| s.trim().parse().ok());
    }
    p.id = "main".into();
    Ok(p)
}

pub fn running(p: &Project) -> bool {
    if let Some(port) = p.port.filter(|x| *x > 0) {
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
        if std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(250)).is_ok() {
            return true;
        }
    }
    !p.process.trim().is_empty() && crate::util::process_running(&[p.process.trim()])
}

fn log_file(p: &Project) -> Option<std::fs::File> {
    let dir = crate::config::data_dir().join("projects");
    std::fs::create_dir_all(&dir).ok()?;
    let id: String = p.id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
    std::fs::OpenOptions::new().create(true).append(true).open(dir.join(format!("{}.log", if id.is_empty() { "project".into() } else { id }))).ok()
}

/// Split "program" args (the program may be quoted or an unquoted path with spaces).
fn split(line: &str) -> Result<(String, String), String> {
    if let Some(rest) = line.strip_prefix('"') {
        let end = rest.find('"').ok_or("Unclosed quote in the command")?;
        return Ok((rest[..end].to_string(), rest[end + 1..].trim().to_string()));
    }
    let lower = line.to_lowercase();
    if let Some(i) = [".ps1", ".exe", ".cmd", ".bat"].iter().filter_map(|e| lower.find(&format!("{e} ")).map(|i| i + e.len()).or_else(|| lower.ends_with(e).then(|| line.len()))).min() {
        if line[..i].contains(['\\', '/']) {
            return Ok((line[..i].to_string(), line[i..].trim().to_string()));
        }
    }
    Ok(match line.split_once(' ') { Some((a, b)) => (a.to_string(), b.to_string()), None => (line.to_string(), String::new()) })
}

/// Run a command line without a window. `.ps1` → PowerShell (policy bypass for that script); `.exe`
/// directly; anything else (`npm run dev`, `docker compose up -d`, `.bat`) through cmd, so commands
/// on PATH resolve like in a terminal.
fn run_line(line: &str, cwd: Option<&Path>, log: Option<std::fs::File>) -> Result<(), String> {
    log::info!("project command: {line}");
    let line = crate::system::expand_env(line.trim());
    let (prog, args) = split(&line)?;
    let lower = prog.to_lowercase();
    let mut c = if lower.ends_with(".ps1") {
        let mut c = std::process::Command::new("powershell.exe");
        c.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-WindowStyle", "Hidden", "-File", &prog]);
        if !args.is_empty() { c.raw_arg(&args); }
        c
    } else if lower.ends_with(".exe") {
        let mut c = std::process::Command::new(&prog);
        if !args.is_empty() { c.raw_arg(&args); }
        c
    } else {
        let mut c = std::process::Command::new("cmd.exe");
        c.raw_arg(format!("/d /s /c \"{line}\""));
        c
    };
    // The project folder; else the script's folder (its parent when the script sits in `bin`).
    let dir = cwd.filter(|d| d.is_dir()).map(Path::to_path_buf).or_else(|| {
        Path::new(&prog).parent().filter(|d| d.is_dir()).map(|d| if d.file_name().is_some_and(|n| n.eq_ignore_ascii_case("bin")) { d.parent().unwrap_or(d).to_path_buf() } else { d.to_path_buf() })
    });
    if let Some(d) = dir { c.current_dir(d); }
    if let Some(f) = log {
        if let Ok(f2) = f.try_clone() { c.stdout(f2); }
        c.stderr(f);
    } else {
        c.stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    }
    c.stdin(std::process::Stdio::null());
    c.creation_flags(CREATE_NO_WINDOW).spawn().map(|_| ()).map_err(|e| format!("Could not run {prog}: {e}"))
}

/// PIDs listening on `port` (IPv4 and IPv6).
fn listeners(port: u16) -> Vec<u32> {
    use windows::Win32::NetworkManagement::IpHelper::*;
    let mut out = vec![];
    for af in [2u32 /* AF_INET */, 23 /* AF_INET6 */] {
        let mut size = 0u32;
        unsafe {
            let _ = GetExtendedTcpTable(None, &mut size, false, af, TCP_TABLE_OWNER_PID_LISTENER, 0);
            if size == 0 { continue }
            let mut buf = vec![0u8; size as usize + 256];
            if GetExtendedTcpTable(Some(buf.as_mut_ptr() as _), &mut size, false, af, TCP_TABLE_OWNER_PID_LISTENER, 0) != 0 { continue }
            let n = u32::from_ne_bytes(buf[0..4].try_into().unwrap()) as usize;
            // Row layouts: v4 = state, local addr, local port, remote addr, remote port, pid (6 × u32);
            // v6 = local addr (16), scope (4), local port (4), remote addr (16), scope (4), remote port (4), state (4), pid (4).
            let (row, port_off, pid_off) = if af == 2 { (24usize, 8usize, 20usize) } else { (56, 20, 52) };
            for i in 0..n {
                let base = 4 + i * row;
                if base + row > buf.len() { break }
                let raw = u32::from_ne_bytes(buf[base + port_off..base + port_off + 4].try_into().unwrap());
                let lport = u16::from_be(raw as u16);
                let pid = u32::from_ne_bytes(buf[base + pid_off..base + pid_off + 4].try_into().unwrap());
                if lport == port && pid > 4 { out.push(pid); }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

pub fn act(p: &Project, action: &str) -> Result<(), String> {
    let cwd = (!p.folder.trim().is_empty()).then(|| PathBuf::from(crate::system::expand_env(p.folder.trim())));
    match action {
        "start" => {
            let s = p.start.trim();
            if s.is_empty() { return Err("No start command set".into()) }
            // An installed app (shell:AppsFolder\…, a shortcut or a plain program): open it like Start does.
            let lower = s.to_lowercase();
            if lower.starts_with("shell:") || lower.trim_matches('"').ends_with(".lnk") || (lower.trim_matches('"').ends_with(".exe") && !s.contains(' ')) {
                return crate::system::launch(s.trim_matches('"'), None);
            }
            run_line(&p.start, cwd.as_deref(), log_file(p))
        }
        "stop" => {
            if !p.stop.trim().is_empty() {
                return run_line(&p.stop, cwd.as_deref(), log_file(p));
            }
            // An app known by its process: ask it to close (like the window's X), then force it.
            if p.port.filter(|x| *x > 0).is_none() && !p.process.trim().is_empty() {
                let name = p.process.trim().to_string();
                let _ = std::process::Command::new("taskkill").args(["/IM", &name, "/T"]).creation_flags(CREATE_NO_WINDOW).output();
                std::thread::sleep(std::time::Duration::from_secs(3));
                if crate::util::process_running(&[name.as_str()]) {
                    let _ = std::process::Command::new("taskkill").args(["/IM", &name, "/T", "/F"]).creation_flags(CREATE_NO_WINDOW).output();
                }
                return Ok(());
            }
            let port = p.port.filter(|x| *x > 0).ok_or("No stop command and no port or process to find the app by")?;
            let pids = listeners(port);
            if pids.is_empty() { return Err(format!("Nothing is listening on port {port}")) }
            for pid in pids {
                let _ = std::process::Command::new("taskkill").args(["/PID", &pid.to_string(), "/T", "/F"]).creation_flags(CREATE_NO_WINDOW).output();
            }
            Ok(())
        }
        _ => Err("unknown action".into()),
    }
}

// ---- set-up helpers ---------------------------------------------------------------------------

fn read(p: &Path) -> String { std::fs::read_to_string(p).unwrap_or_default() }

/// First `localhost:NNNN` / `127.0.0.1:NNNN` mentioned in the README (where projects say where they run).
fn readme_port(dir: &Path) -> Option<u16> {
    for n in ["README.md", "readme.md", "README.MD", "README.txt", "README"] {
        let t = read(&dir.join(n));
        for key in ["localhost:", "127.0.0.1:"] {
            let mut rest = t.as_str();
            while let Some(i) = rest.find(key) {
                let digits: String = rest[i + key.len()..].chars().take_while(|c| c.is_ascii_digit()).collect();
                if let Ok(p) = digits.parse::<u16>() { if p >= 80 { return Some(p) } }
                rest = &rest[i + key.len()..];
            }
        }
    }
    None
}

fn command_exists(cmd: &str) -> bool {
    std::process::Command::new("where").arg(cmd).creation_flags(CREATE_NO_WINDOW).output().map(|o| o.status.success()).unwrap_or(false)
}

/// Best guess at how to run the project in `folder`. Everything stays editable in the card.
pub fn detect(folder: &str) -> Result<Project, String> {
    let dir = PathBuf::from(crate::system::expand_env(folder.trim()));
    if !dir.is_dir() { return Err("That folder doesn't exist".into()) }
    let mut p = Project { folder: dir.to_string_lossy().into(), name: dir.file_name().map(|n| n.to_string_lossy().replace(['-', '_'], " ")).unwrap_or_default(), ..Default::default() };
    p.port = readme_port(&dir);
    let has = |n: &str| dir.join(n).exists();

    // 1. A start/stop script of its own (e.g. bin/app.ps1 start|stop).
    let script = ["bin", "scripts", "."].iter().flat_map(|sub| std::fs::read_dir(dir.join(sub)).ok().into_iter().flatten().flatten())
        .map(|e| e.path()).find(|f| f.extension().is_some_and(|x| x.eq_ignore_ascii_case("ps1")) && {
            let t = read(f).to_lowercase();
            t.contains("validateset(\"start\", \"stop\"") || t.contains("validateset('start', 'stop'")
        });
    let compose = ["docker-compose.yml", "docker-compose.yaml", "compose.yml", "compose.yaml"].iter().find(|n| has(n));
    if let Some(s) = script {
        let q = format!("\"{}\"", s.display());
        p.start = format!("{q} start");
        p.stop = format!("{q} stop");
        if p.port.is_none() {
            let t = read(&s);
            p.port = t.find("$port = ").and_then(|i| t[i + 8..].chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().ok());
        }
    } else if compose.is_some() && command_exists("docker") {
        // 2. Docker Compose (needs Docker Desktop running).
        p.start = "docker compose up -d".into();
        p.stop = "docker compose down".into();
    } else if let Some(b) = ["launch.bat", "start.bat", "run.bat", "launch.cmd", "start.cmd"].iter().find(|n| has(n)) {
        // 3. The project's own Windows launcher.
        p.start = format!("\"{}\"", dir.join(b).display());
    } else if has("package.json") {
        // 4. npm scripts.
        let pkg: serde_json::Value = serde_json::from_str(&read(&dir.join("package.json"))).unwrap_or_default();
        let runner = if has("pnpm-lock.yaml") { "pnpm" } else if has("yarn.lock") { "yarn" } else { "npm" };
        let scripts = &pkg["scripts"];
        let script = ["dev", "start", "serve", "preview"].iter().find(|s| scripts.get(**s).is_some());
        if let Some(s) = script {
            p.start = if *s == "start" && runner == "npm" { "npm start".into() } else { format!("{runner} run {s}") };
            let body = scripts[*s].as_str().unwrap_or("");
            if p.port.is_none() {
                p.port = body.split_whitespace().skip_while(|w| *w != "--port" && *w != "-p").nth(1).and_then(|x| x.parse().ok())
                    .or(if body.contains("vite") { Some(5173) } else if body.contains("next") || body.contains("react-scripts") { Some(3000) } else { None });
            }
        }
    } else {
        // 5. Python entry points.
        let req = read(&dir.join("requirements.txt")).to_lowercase();
        if has("manage.py") { p.start = "python manage.py runserver".into(); p.port = p.port.or(Some(8000)); }
        else if let Some(f) = ["app.py", "main.py", "streamlit_app.py"].iter().find(|n| has(n)) {
            if req.contains("streamlit") { p.start = format!("streamlit run {f}"); p.port = p.port.or(Some(8501)); }
            else if req.contains("uvicorn") || req.contains("fastapi") { p.start = format!("python -m uvicorn {}:app", f.trim_end_matches(".py")); p.port = p.port.or(Some(8000)); }
            else { p.start = format!("python {f}"); }
        }
    }
    if let Some(port) = p.port { p.url = format!("http://localhost:{port}"); }
    // An icon from the project, if it has one.
    for c in ["public/android-chrome-192x192.png", "public/apple-touch-icon.png", "public/logo192.png", "frontend/public/logo192.png", "public/favicon.ico", "frontend/public/favicon.ico", "favicon.ico", "icon.png", "logo.png", "docs/logo.png"] {
        let f = dir.join(c);
        if f.is_file() { p.icon = f.to_string_lossy().into(); break }
    }
    Ok(p)
}

/// `git clone` a GitHub repository into `parent` (only https://github.com/<owner>/<repo>).
/// Returns the new folder. Nothing in the repository is run.
pub fn clone(url: &str, parent: &str) -> Result<String, String> {
    let u = url.trim().trim_end_matches('/').trim_end_matches(".git");
    let rest = u.strip_prefix("https://github.com/").ok_or("Paste a https://github.com/owner/repository link")?;
    let parts: Vec<&str> = rest.split('/').collect();
    let ok = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c)) && s != "." && s != "..";
    if parts.len() != 2 || !ok(parts[0]) || !ok(parts[1]) { return Err("Paste a https://github.com/owner/repository link".into()) }
    let parent = PathBuf::from(crate::system::expand_env(parent.trim()));
    if !parent.is_dir() { return Err("The folder to clone into doesn't exist".into()) }
    let dest = parent.join(parts[1]);
    if dest.exists() { return Ok(dest.to_string_lossy().into()) }
    if !command_exists("git") { return Err("Git isn't installed (git-scm.com)".into()) }
    let o = std::process::Command::new("git").args(["clone", "--depth", "1", &format!("https://github.com/{}/{}.git", parts[0], parts[1])]).arg(&dest)
        .creation_flags(CREATE_NO_WINDOW).output().map_err(|e| e.to_string())?;
    if !o.status.success() {
        return Err(format!("git clone failed: {}", String::from_utf8_lossy(&o.stderr).lines().last().unwrap_or("").trim()));
    }
    Ok(dest.to_string_lossy().into())
}
