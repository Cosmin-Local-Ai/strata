mod capture;
mod cards;
mod google;
mod local;
mod mail;
mod services;
mod toggles;
mod vault;
mod dialog;
mod projects;
mod commands;
mod config;
mod dock;
mod glass;
mod komorebi;
mod migrate;
mod secrets;
mod shell;
mod system;
mod util;
mod wallpaper_engine;

use dock::{DockLayout, Docks};
use glass::GlassRect;
use shell::appbar::ShellEvent;
use shell::gamemode;
use shell::taskbar::{self, TaskbarStatus};
use std::sync::Mutex;
use tauri::{Emitter, Manager, State, WebviewWindow};
use windows::Win32::Foundation::RECT;

/// Game / local-AI yield mode is active (D-011).
pub static YIELDING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(crate) struct AppState {
    pub store: config::Store,
    pub docks: Docks,
    pub taskbar: Mutex<TaskbarStatus>,
    pub cards: cards::Cards,
    pub kstate: Mutex<komorebi::KState>,
    pub media: Mutex<Option<system::media::Media>>,
    pub audio: Mutex<Option<system::audio::Audio>>,
}

#[tauri::command]
fn dock_layout(window: WebviewWindow, state: State<AppState>) -> Option<DockLayout> {
    state.docks.layout_for(window.label())
}

#[tauri::command]
fn glass_rect(window: WebviewWindow, state: State<AppState>, rect: GlassRect) {
    state.docks.set_glass_rect(window.label(), rect);
}

/// Interactive areas of the dock window in physical px: [x, y, w, h, radius].
#[tauri::command]
fn dock_region(window: WebviewWindow, state: State<AppState>, rects: Vec<[f64; 5]>) {
    let r = rects
        .into_iter()
        .map(|[x, y, w, h, rad]| {
            (RECT { left: x.floor() as i32, top: y.floor() as i32, right: (x + w).ceil() as i32, bottom: (y + h).ceil() as i32 }, rad.round() as i32)
        })
        .collect();
    state.docks.update_region(window.label(), r);
}

#[tauri::command]
fn glass_debug(window: WebviewWindow, state: State<AppState>) -> String {
    state.docks.glass_for(window.label()).map(|g| g.debug()).unwrap_or_else(|| "no glass".into())
}

#[tauri::command]
fn taskbar_status(state: State<AppState>) -> TaskbarStatus {
    state.taskbar.lock().unwrap().clone()
}

fn handle_shell_event(app: &tauri::AppHandle, ev: ShellEvent) {
    const SPI_SETDESKWALLPAPER: u32 = 0x0014;
    let state = app.state::<AppState>();
    match ev {
        ShellEvent::Fullscreen { .. } => gamemode::notify(gamemode::Trigger::FullscreenHint),
        ShellEvent::PosChanged { .. } | ShellEvent::DisplayChange | ShellEvent::DpiChange { .. } => schedule_sync(app),
        ShellEvent::SettingChange { spi } if spi == SPI_SETDESKWALLPAPER => state.docks.for_each_glass(|g| g.refresh_background()),
        ShellEvent::SettingChange { .. } | ShellEvent::TaskbarState => {}
        ShellEvent::ExplorerRestarted if system::tray::SELF_BROADCAST_UNTIL.lock().unwrap().map(|t| std::time::Instant::now() < t).unwrap_or(false) => {}
        ShellEvent::ExplorerRestarted => {
            log::warn!("Explorer restarted: re-applying taskbar state and AppBars");
            taskbar::reapply(&state.taskbar.lock().unwrap());
            state.docks.forget_appbars();
            state.docks.sync();
        }
        ShellEvent::Resumed | ShellEvent::Unlocked => schedule_sync(app),
    }
}

/// Coalesce bursts of shell notifications (each AppBar SETPOS makes the shell notify every AppBar,
/// including us) into one sync 300 ms after the first.
fn schedule_sync(app: &tauri::AppHandle) {
    use std::sync::atomic::{AtomicBool, Ordering};
    static PENDING: AtomicBool = AtomicBool::new(false);
    if PENDING.swap(true, Ordering::SeqCst) {
        return;
    }
    let h = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(300));
        let h2 = h.clone();
        let _ = h.run_on_main_thread(move || {
            PENDING.store(false, Ordering::SeqCst);
            h2.state::<AppState>().docks.sync();
        });
    });
}

/// `strata <command>`: actions you can bind in whkd (brief §5, optional).
///   settings · quit · power · komorebi-start · komorebi-stop · komorebi-restart · reload
fn cli_command(app: &tauri::AppHandle, argv: &[String]) {
    let Some(cmd) = argv.get(1).map(|s| s.trim_start_matches('-').to_lowercase()) else { return };
    log::info!("cli command: {cmd}");
    let h = app.clone();
    match cmd.as_str() {
        // Same as the Show Desktop widget (handy for hotkeys and for testing).
        "show-desktop" => { std::thread::spawn(crate::system::misc::show_desktop); }
        "settings" => {
            tauri::async_runtime::spawn(async move {
                let _ = commands::open_settings(h).await;
            });
        }
        "quit" | "exit" => app.exit(0),
        "close" => {
            let st = app.state::<AppState>();
            st.cards.overlay_close(app);
            st.cards.hide(app);
        }
        "widgets" | "power" => {
            let kind = cmd.clone();
            if let Some(dock) = app.state::<AppState>().docks.labels().first().and_then(|l| app.get_webview_window(l)) {
                tauri::async_runtime::spawn(async move {
                    let _ = commands::overlay_open(h, dock, kind).await;
                });
            }
        }
        "komorebi-start" => { let _ = komorebi::control("start"); }
        "komorebi-stop" => { let _ = komorebi::control("stop"); }
        "komorebi-restart" => { let _ = komorebi::control("restart"); }
        "reload" => {
            let _ = app.run_on_main_thread(move || h.state::<AppState>().docks.sync());
        }
        other => log::warn!("unknown command: {other}"),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        // One Strata at a time. A second `strata.exe <command>` forwards the command (for whkd bindings).
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| cli_command(app, &argv)))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_log::Builder::default().level(log::LevelFilter::Info).level_for("strata_lib::system::audio", log::LevelFilter::Debug).build())
        .register_asynchronous_uri_scheme_protocol("icon", |_ctx, request, responder| commands::icon_protocol(request, responder))
        .register_asynchronous_uri_scheme_protocol("thumb", |_ctx, request, responder| commands::thumb_protocol(request, responder))
        .invoke_handler(tauri::generate_handler![
            dock_layout, glass_rect, dock_region, glass_debug, taskbar_status,
            commands::settings_get, commands::settings_set_items, commands::card_open, commands::card_ready,
            commands::card_close, commands::card_action, commands::launch, commands::power, commands::start_menu,
            commands::winx_menu, commands::uptime, commands::komorebi_cmd, commands::komorebi_control,
            commands::komorebi_state, commands::data_read, commands::data_write, commands::overlay_open, commands::overlay_close, commands::overlay_pending, commands::settings_update, commands::displays_list, commands::open_settings, commands::quit, commands::taskbar_state, commands::dock_ready, commands::overlay_ready,
            commands::media_state, commands::media_control, commands::media_seek, commands::audio_state,
            commands::audio_set_volume, commands::audio_set_mute, commands::wifi_state, commands::net_info, commands::wifi_networks, commands::wifi_connect, commands::eq_rect, commands::windows_list, commands::window_activate, commands::window_close, commands::gallery_state, commands::gallery_pictures, commands::gallery_apply, commands::previews_set, commands::card_close_if, commands::preview_hover, commands::settings_reset, commands::secret_has, commands::secret_set, commands::secret_delete, commands::download_wallpaper, commands::http_get, commands::wallpaper_change_now, commands::tray_icons, commands::tray_click, commands::card_sticky, commands::we_control, commands::we_playlist, commands::audio_set_default, commands::audio_sessions, commands::audio_session_set,
            commands::sys_stats, commands::recycle_bin, commands::recycle_bin_empty, commands::theme_is_light, commands::theme_set_light,
            commands::mic_muted, commands::mic_set_muted, commands::sys_action, commands::kb_layouts, commands::kb_set_layout, commands::battery, commands::launch_admin, commands::reveal, commands::audio_devices, commands::audio_device_set, commands::everything_search, commands::everything_running, commands::api, commands::google_connect, commands::google_status, commands::google_disconnect, commands::yahoo_login, commands::yahoo_mail, commands::yahoo_disconnect, commands::opencode_usage, commands::screen_time, commands::whatsapp, commands::folder_list, commands::folder_path, commands::toggles_get, commands::toggle_set, commands::brightness_get, commands::brightness_set, commands::capture_screen, commands::capture_window, commands::capture_region_begin, commands::capture_region_finish, commands::capture_region_cancel, commands::clipboard_text, commands::capture_temp_path, commands::calendars_list, commands::calendars_add, commands::calendars_remove, commands::calendars_fetch, commands::flow_toggle, commands::gamma_set, commands::gamma_reset, commands::autostart_get, commands::autostart_set, commands::capture_save_set, commands::vault_list, commands::vault_state, commands::vault_unlock, commands::vault_lock, commands::vault_save, commands::vault_delete, commands::vault_reveal, commands::vault_copy, commands::vault_generate, commands::default_player, commands::vault_lock_info, commands::vault_set_master, commands::vault_import, commands::vault_export, commands::vault_health, commands::vault_attach, commands::vault_file_save, commands::vault_file_remove, commands::vault_passphrase, commands::pick_file, commands::office_apps, commands::project_action, commands::project_detect, commands::project_clone, commands::pick_folder, commands::terminal_run, commands::terminal_info, commands::start_apps, commands::google_client_saved, commands::quick_files, commands::save_text_as, commands::ui_error
        ])
        .on_window_event(|window, event| {
            // Close popups on focus loss, but only once they really had focus: when Windows refuses
            // to give them the foreground, a "lost focus" right after showing must not close them.
            use std::sync::atomic::Ordering;
            let is_overlay = window.label() == commands::OVERLAY_LABEL || window.label() == commands::POWER_LABEL;
            let had = if window.label() == commands::CARD_LABEL { Some(&commands::CARD_FOCUSED) } else if is_overlay { Some(&commands::OVERLAY_FOCUSED) } else { None };
            if let (Some(flag), tauri::WindowEvent::Focused(true)) = (had, event) {
                flag.store(true, Ordering::SeqCst);
            }
            if let tauri::WindowEvent::Focused(false) = event {
                if !had.map(|f| f.swap(false, Ordering::SeqCst)).unwrap_or(false) {
                    return;
                }
                if window.label() == commands::CARD_LABEL {
                    commands::on_card_blur(window.app_handle());
                } else if is_overlay {
                    // The Widgets picker stays open while you rearrange tiles on the dock.
                    let st = window.app_handle().state::<AppState>();
                    if st.cards.overlay_kind().as_deref() != Some("widgets") {
                        st.cards.overlay_close(window.app_handle());
                    }
                }
            }
        })
        .setup(|app| {
            let store = config::Store::load();
            let settings = store.get();
            // Before any Strata window exists, so komorebi never manages one even for a moment (D-004).
            // Re-applied on every komorebi (re)connect as well.
            komorebi::ensure_ignore_rules();
            wallpaper_engine::gallery::start_schedule(store.clone());
            let tb = taskbar::start(settings.taskbar_mode);
            log::info!("taskbar: {tb:?}");

            let (tx, rx) = std::sync::mpsc::channel::<ShellEvent>();
            let docks = Docks::new(app.handle().clone(), store.clone(), tx);
            app.manage(AppState { store, docks: docks.clone(), taskbar: Mutex::new(tb), cards: cards::Cards::default(), kstate: Mutex::new(Default::default()), media: Mutex::new(None), audio: Mutex::new(None) });

            // Media sessions and audio: event-driven, started off the main thread.
            let h = app.handle().clone();
            std::thread::spawn(move || {
                let h2 = h.clone();
                let m = system::media::Media::start(move |st| { let _ = h2.emit("media:state", st); });
                *h.state::<AppState>().media.lock().unwrap() = m;
            });
            let h = app.handle().clone();
            let audio = system::audio::Audio::start(move |st| { let _ = h.emit("audio:state", st); });
            *app.state::<AppState>().audio.lock().unwrap() = Some(audio);
            docks.sync();
            app.state::<AppState>().cards.warm(app.handle());

            // Komorebi live state (also re-applies the ignore rule on every connect).
            let h = app.handle().clone();
            let h2 = app.handle().clone();
            komorebi::start_with_recovery(move |st| {
                *h.state::<AppState>().kstate.lock().unwrap() = st.clone();
                let _ = h.emit("komorebi:state", st);
            }, move || {
                let h3 = h2.clone();
                let _ = h2.run_on_main_thread(move || h3.state::<AppState>().docks.recreate());
            });

            // Shell events arrive inside the window procedure; forward them so it never blocks.
            let h = app.handle().clone();
            std::thread::Builder::new().name("strata-shell-events".into()).spawn(move || {
                for ev in rx {
                    let h2 = h.clone();
                    let _ = h.run_on_main_thread(move || handle_shell_event(&h2, ev));
                }
            })?;

            // Equaliser driver: real output level -> native bars, 20 Hz, only while music plays and
            // Strata isn't yielding to a game. DWM animates between updates (glass/eq.rs).
            let h = app.handle().clone();
            std::thread::Builder::new().name("strata-eq".into()).spawn(move || {
                use std::sync::atomic::Ordering;
                let mut lv = [0.3f32; 3];
                let mut t = 0f32;
                // Automatic gain: normalise to the recent loudest peak so the bars use their full range
                // whatever the volume (decays slowly, never below a floor so silence stays low).
                let mut gain_ref = 0.05f32;
                loop {
                    let st = h.state::<AppState>();
                    let (playing, hint) = st.media.lock().unwrap().as_ref().map(|m| (m.playing(), m.app_hint())).unwrap_or((false, String::new()));
                    if !playing || YIELDING.load(Ordering::Relaxed) {
                        std::thread::sleep(std::time::Duration::from_millis(300));
                        continue;
                    }
                    let audio = st.audio.lock().unwrap().clone();
                    let (l, r) = audio.as_ref().and_then(|a| a.peak_for(&hint)).unwrap_or((0.0, 0.0));
                    t += 0.05;
                    // Follows the music within about a second (0.985 per 50 ms tick).
                    gain_ref = (gain_ref * 0.985).max(l.max(r)).max(0.02);
                    let base = [l / gain_ref, (l + r) * 0.5 / gain_ref, r / gain_ref];
                    for i in 0..3 {
                        // Perceptual curve plus a little per-bar motion so three bars don't move as one.
                        let wob = 0.78 + 0.22 * (t * (6.0 + i as f32 * 2.7) + i as f32 * 1.9).sin();
                        let target = (base[i].powf(0.8) * wob).min(1.0);
                        // Fast attack, slower release, like a VU meter.
                        lv[i] = if target > lv[i] { target } else { lv[i] * 0.72 + target * 0.28 };
                    }
                    st.docks.for_each_glass(|g| g.set_eq_levels(lv, 60.0));
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            })?;

            // Tray icons via the injected hook (user-approved, see system/tray.rs).
            let h = app.handle().clone();
            system::tray::start(move |list| { let _ = h.emit("tray:icons", list); });

            // Running windows for the Apps tile (hooks on the main thread).
            let h = app.handle().clone();
            system::winlist::start(move |list| { let _ = h.emit("windows:list", list); });

            local::start_screen_time();
            // First run: start with Windows by default (listed in Task Manager → Startup apps; switch in Settings).
            {
                let marker = config::data_dir().join(".autostart-initialised");
                if !marker.exists() {
                    let _ = toggles::set_autostart(true);
                    let _ = std::fs::write(&marker, b"1");
                } else if toggles::autostart() {
                    let _ = toggles::set_autostart(true); // keep the path current if Strata moved
                }
            }
            gamemode::install_foreground_hook();
            let h = app.handle().clone();
            gamemode::start(settings.yield_apps.clone(), settings.yield_for_steam_games, move |st| {
                let st = st.clone();
                let h2 = h.clone();
                let _ = h.run_on_main_thread(move || {
                    let state = h2.state::<AppState>();
                    YIELDING.store(st.active, std::sync::atomic::Ordering::Relaxed);
                    state.docks.for_each_glass(|g| g.set_yield(st.active));
                    state.docks.set_fullscreen_monitors(&st.fullscreen_monitors);
                    let _ = h2.emit("dock:yield", &st);
                    if st.active {
                        util::trim_working_set();
                    }
                });
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Strata");

    app.run(|handle, event| {
        if let tauri::RunEvent::Exit = event {
            if let Some(state) = handle.try_state::<AppState>() {
                state.docks.shutdown();
            }
            taskbar::stop();
            toggles::gamma_reset();
            log::info!("Strata exited cleanly");
        }
    });
}
