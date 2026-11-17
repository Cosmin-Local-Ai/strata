//! System stats for the CPU / RAM / Top CPU / Top RAM / Network / Storage tiles. Sampled only when a
//! visible tile asks (the web UI polls every 2 s and pauses while hidden or under fullscreen).
//! CPU is the whole-machine utilisation like Task Manager; per-process CPU is divided by the logical
//! core count (Task Manager's convention) and processes are grouped by app name.

use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use sysinfo::{Disks, Networks, ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Proc {
    pub name: String,
    pub value: f64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Drive {
    pub name: String,
    pub mount: String,
    pub total: u64,
    pub free: u64,
    pub removable: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub cpu: f64,
    pub cores: usize,
    pub ram_used: u64,
    pub ram_total: u64,
    pub top_cpu: Vec<Proc>,
    pub top_ram: Vec<Proc>,
    /// Bytes per second since the previous sample.
    pub net_down: f64,
    pub net_up: f64,
    pub drives: Vec<Drive>,
}

struct State {
    sys: System,
    nets: Networks,
    last: std::time::Instant,
}

fn state() -> &'static Mutex<State> {
    static S: OnceLock<Mutex<State>> = OnceLock::new();
    S.get_or_init(|| {
        let mut sys = System::new_with_specifics(RefreshKind::nothing());
        sys.refresh_cpu_usage();
        Mutex::new(State { sys, nets: Networks::new_with_refreshed_list(), last: std::time::Instant::now() })
    })
}

/// `procs`: also sample processes (Top CPU/RAM). `drives`: also list drives.
pub fn sample(procs: bool, drives: bool) -> Stats {
    let mut g = state().lock().unwrap();
    let st = &mut *g;
    st.sys.refresh_cpu_usage();
    st.sys.refresh_memory();
    let cores = st.sys.cpus().len().max(1);
    let (mut top_cpu, mut top_ram) = (vec![], vec![]);
    if procs {
        st.sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().with_cpu().with_memory());
        let mut cpu: HashMap<String, f64> = HashMap::new();
        let mut ram: HashMap<String, f64> = HashMap::new();
        for p in st.sys.processes().values() {
            let name = p.name().to_string_lossy().trim_end_matches(".exe").to_string();
            if name.is_empty() || name == "System Idle Process" || name == "Idle" {
                continue;
            }
            *cpu.entry(name.clone()).or_default() += p.cpu_usage() as f64 / cores as f64;
            *ram.entry(name).or_default() += p.memory() as f64;
        }
        let top = |m: HashMap<String, f64>| {
            let mut v: Vec<Proc> = m.into_iter().map(|(name, value)| Proc { name, value }).collect();
            v.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap_or(std::cmp::Ordering::Equal));
            v.truncate(6);
            v
        };
        top_cpu = top(cpu);
        top_ram = top(ram);
    }
    let dt = st.last.elapsed().as_secs_f64().max(0.001);
    st.last = std::time::Instant::now();
    st.nets.refresh(true);
    let (mut down, mut up) = (0u64, 0u64);
    for (name, n) in st.nets.iter() {
        // Skip loopback and virtual adapters that would double-count traffic.
        let l = name.to_lowercase();
        if l.contains("loopback") || l.contains("vethernet") || l.contains("virtual") {
            continue;
        }
        down += n.received();
        up += n.transmitted();
    }
    let drives = if drives {
        Disks::new_with_refreshed_list()
            .iter()
            .map(|d| Drive {
                name: d.name().to_string_lossy().to_string(),
                mount: d.mount_point().to_string_lossy().to_string(),
                total: d.total_space(),
                free: d.available_space(),
                removable: d.is_removable(),
            })
            .collect()
    } else {
        vec![]
    };
    Stats {
        cpu: st.sys.global_cpu_usage() as f64,
        cores,
        ram_used: st.sys.used_memory(),
        ram_total: st.sys.total_memory(),
        top_cpu,
        top_ram,
        net_down: down as f64 / dt,
        net_up: up as f64 / dt,
        drives,
    }
}
