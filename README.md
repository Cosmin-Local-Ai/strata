# Strata

A glassy, widget-packed dock for Windows 11. It replaces the taskbar with a floating (or full-width)
bar of live tiles: running apps with window previews, now playing with lyrics, weather, system stats,
calendar, AI usage limits, analytics, mail, Office and Google Workspace launchers, cloud storage, a
password vault (with import from Google, Apple and Chromium browsers) and more than 130 other widgets. Click a
tile and a card opens right there to do things, instead of sending you to Windows Settings.

- **Light on resources.** A native Rust core with the system WebView2 for the interface. About 100 MB
  of memory and near-zero CPU at idle; it steps out of the way while you game (fullscreen apps,
  Steam games and apps you choose).
- **Glass that stays out of the GPU.** The glass effect renders on the CPU (WARP), so game overlays
  and GPU tools are never triggered.
- **Private by design.** No account, no telemetry, no server. Keys and passwords live in Windows
  Credential Manager; each online widget talks only to its own service, and only once it is on your
  dock.

> Status: early. Windows 11 only for now; a Linux (Hyprland / Wayland) version is planned.

## Install

1. Download `Strata_x.y.z_x64-setup.exe` from [Releases](../../releases) and run it.
   Windows SmartScreen may warn because the installer isn't code-signed yet: choose
   *More info → Run anyway*.
2. Strata starts right away and from then on with Windows (listed in Task Manager → Startup apps;
   switch it off in Settings → About). It's also in the Start menu.
3. Right-click an empty part of the dock (or add the **Dock Menu** widget) → **Widgets…** to add,
   remove and reorder tiles. Drag tiles on the dock to reorder them any time.

Updates: Settings → About → **Check for updates** (updates are signed and verified before they install).

### Requirements

- Windows 11 (22H2 or later). Windows 10 may work but isn't tested.
- Microsoft Edge WebView2 Runtime: already part of Windows 11.

### The Windows taskbar

By default Strata sets the Windows taskbar to **auto-hide** while it runs and puts it back when it
quits, even after a crash (a tiny watchdog, `strata-guard.exe`, restores it). Settings → Windows
taskbar lets you choose *Auto-hide*, *Hidden* or *Leave alone*. If you already use a tool that hides
the taskbar (for example *Taskbar Hide*), Strata detects it and leaves the taskbar to that tool.

### Optional companions

Strata works on its own; these make some widgets light up:

| Tool | For | Notes |
| --- | --- | --- |
| [komorebi](https://github.com/LGUG2Z/komorebi) | Workspaces and Layout widgets | Strata adds its own ignore rule so komorebi never tiles the dock. |
| [Everything](https://www.voidtools.com/) | Everything widget (instant file search) | Must be running. |
| [Flow Launcher](https://www.flowlauncher.com/) | Flow Launcher widget | |
| [LibreHardwareMonitor](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor) | Temperature widget | Turn on *Options → Remote Web Server* (port 8085). |
| [Wallpaper Engine](https://www.wallpaperengine.io/) | Wallpaper Gallery (your workshop items, playlists, controls) | Pictures and Unsplash work without it. |
| Windows Hello | Vault widget unlock | Without Hello the vault still opens (protected by your Windows sign-in). |
| Claude Code / Codex CLI | Claude and ChatGPT usage widgets | Uses the sign-in already on your PC, read-only. |

### Connecting accounts

Online widgets (Stripe, GitHub, Google Analytics, Gmail, Yahoo Mail, Meta Ads, YouTube, …) show a
connect form in their card with exact steps and a link to the page where you create the key.
Everything is stored in Windows Credential Manager under `Strata/…` (you can see and delete entries
in Control Panel → Credential Manager) and is sent only to that service. Google widgets use your own
Google Cloud OAuth client (read-only scopes; Google Drive lists file names and dates, never contents);
Yahoo Mail and Digi Storage use an app password, and Yahoo Mail never marks mail as read.

### Vault

Logins, API keys, secure notes, cards, Wi-Fi networks, ID documents, recovery codes and small files.
Secrets live in Credential Manager (larger items and files in DPAPI-encrypted files under
`%APPDATA%\Strata\vault`). Import a CSV exported from Google Password Manager, Apple Passwords /
iCloud Keychain, Vivaldi, Chrome, Edge, Firefox, Bitwarden or 1Password; export to CSV or JSON. The
health report flags weak, reused and old passwords and can check them against Have I Been Pwned
without sending them (k-anonymity: only 5 characters of each hash leave the PC). Unlock with Windows
Hello, an optional master password, or both.

## Build from source

You need [Node.js 22](https://nodejs.org/), [Rust (stable, MSVC)](https://rustup.rs/) and the
[Visual Studio 2022 Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with
*Desktop development with C++*.

```powershell
git clone https://github.com/Cosmin-Local-Ai/strata
cd strata
npm ci
tray\build.cmd                                          # tray hook DLL
cargo build --release --manifest-path guard/Cargo.toml  # taskbar watchdog
npx tauri build --no-bundle                             # src-tauri\target\release\strata.exe
```

`npx tauri build` (without `--no-bundle`) also makes the installer; it needs an updater signing key
(`npx tauri signer generate`) in `TAURI_SIGNING_PRIVATE_KEY`. Releases are built by
`.github/workflows/release.yml` when a `v*` tag is pushed.

Command line (also handy for hotkeys): `strata.exe settings | widgets | power | reload | quit`.

## Project layout

- `src/`: the interface (Svelte 5). Each widget is a folder in `src/widgets` plus one line in
  `src/widgets/index.ts`; online services are small adapters in `src/widgets/online/adapters`.
- `src-tauri/`: the Rust core: dock windows and AppBar, glass renderer, taskbar handling, system
  data (audio, media, network, tray, windows), the service gateway that adds credentials, vault,
  capture, Google and IMAP sign-in.
- `guard/`: the taskbar watchdog. `tray/`: the tray hook DLL (C++).

## Credits

- Tray hook adapted from [YASB](https://github.com/amnweb/yasb) (MIT, see `tray/LICENSE-YASB`).
- Weather and geocoding by [Open-Meteo](https://open-meteo.com/); exchange rates by
  [Frankfurter](https://frankfurter.dev/) (ECB data); world map from [Natural Earth](https://www.naturalearthdata.com/) (public domain).
- Icons: [Lucide](https://lucide.dev/) (ISC).

## License

MIT. See [LICENSE](LICENSE).
