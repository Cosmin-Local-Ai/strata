// Widget registry. To add a widget: create its folder (Tile + optional Card) and add one entry here.
import { ipc } from '../lib/ipc'
import SysTile from './system/Tile.svelte'
import SysCard from './system/Card.svelte'
import ActTile from './actions/Tile.svelte'
import ActCard from './actions/Card.svelte'
import { toggleMic, toggleTheme } from './actions/actions.svelte'
import ClockVTile from './clocks2/Tile.svelte'
import ClockVCard from './clocks2/Card.svelte'
import MapCard from './clocks2/MapCard.svelte'
import ProdTile from './productivity/Tile.svelte'
import ProdCard from './productivity/Card.svelte'
import MarketTile from './markets/Tile.svelte'
import MarketCard from './markets/Card.svelte'
import SearchCard from './search/Card.svelte'
import OnlineTile from './online/Tile.svelte'
import OnlineCard from './online/Card.svelte'
import { AI } from './online/adapters/ai'
import { FINANCE } from './online/adapters/finance'
import { DEV } from './online/adapters/dev'
import { MARKETS, SOCIAL } from './online/adapters/social'
import { CLOUD } from './online/adapters/cloud'
import EvTile from './everything/Tile.svelte'
import EvCard from './everything/Card.svelte'
import type { Component } from 'svelte'
import SearchTileC from './search/Tile.svelte'
import AppTile from './app/Tile.svelte'
import ClockCard from './clock/Card.svelte'
import ClockTile from './clock/Tile.svelte'
import LayoutCard from './komorebi/LayoutCard.svelte'
import LayoutTile from './komorebi/LayoutTile.svelte'
import WorkspacesTile from './komorebi/WorkspacesTile.svelte'
import { CYCLE, layoutArg, monitorFor } from './komorebi/komorebi'
import LaunchpadCard from './launchpad/Card.svelte'
import LaunchpadTile from './launchpad/Tile.svelte'
import NotesCard from './notes/Card.svelte'
import NotesTile from './notes/Tile.svelte'
import PlannedTile from './planned/Tile.svelte'
import PowerCard from './power/Card.svelte'
import PowerTile from './power/Tile.svelte'
import StartTile from './start/Tile.svelte'
import type { DockContext, WidgetDef } from './types'
import WeatherCard from './weather/Card.svelte'
import WeatherTile from './weather/Tile.svelte'
import MediaTile from './media/Tile.svelte'
import MediaCard from './media/Card.svelte'
import MediaVariants from './media/Variants.svelte'
import SuiteTile from './suite/Tile.svelte'
import SuiteCard from './suite/Card.svelte'
import MeetTile from './meetings/Tile.svelte'
import TermTile from './terminal/Tile.svelte'
import TermCard from './terminal/Card.svelte'
import StartSearchTile from './startsearch/Tile.svelte'
import StartSearchCard from './startsearch/Card.svelte'
import CloudTile from './cloud/Tile.svelte'
import CloudCard from './cloud/Card.svelte'
import MeetCard from './meetings/Card.svelte'
import LocalAppTile from './localapp/Tile.svelte'
import LocalAppCard from './localapp/Card.svelte'
import { projectsOf } from './localapp/state.svelte'
import { idleTarget } from './media/player.svelte'
import WeatherVariants from './weather/Variants.svelte'
import ClockMore from './clocks2/More.svelte'
import SysXTile from './sysextra/Tile.svelte'
import SysXCard from './sysextra/Card.svelte'
import FilesTile from './files/Tile.svelte'
import ProgressCard from './actions/ProgressCard.svelte'
import CaptureTile from './files/CaptureTile.svelte'
import VaultTile from './vault/Tile.svelte'
import VaultCard from './vault/Card.svelte'
import DockMenuTile from './dockmenu/Tile.svelte'
import FilesCard from './files/Card.svelte'
import Prod2Tile from './productivity2/Tile.svelte'
import Prod2Card from './productivity2/Card.svelte'
import AudioTile from './audio/Tile.svelte'
import AudioCard from './audio/Card.svelte'
import WifiTile from './wifi/Tile.svelte'
import WifiCard from './wifi/Card.svelte'
import TrayTile from './tray/Tile.svelte'
import TrayCard from './tray/Card.svelte'
import WallpaperTile from './wallpaper/Tile.svelte'
import WallpaperCard from './wallpaper/Card.svelte'
import AppsTile from './apps/Tile.svelte'
import AppsCard from './apps/Card.svelte'
import { groupWindows, running, windowsForPinned } from './apps/apps.svelte'
import { settings } from '../lib/stores.svelte'
import { invoke } from '@tauri-apps/api/core'
import { notice } from '../lib/notice'
import { media } from '../lib/stores.svelte'
import { audio } from '../lib/stores.svelte'

function stepLayout(ctx: DockContext, dir: 1 | -1) {
  const m = monitorFor(ctx.layout.hmonitor)
  const cur = m ? layoutArg(m.workspaces[m.focusedWorkspace]?.layout ?? '') : undefined
  const i = cur ? CYCLE.indexOf(cur) : -1
  const next = CYCLE[(i + dir + CYCLE.length) % CYCLE.length]
  ipc.komorebi('change-layout', next)
}

const planned = (id: string, name: string, category: WidgetDef['category'], description: string): WidgetDef => ({
  id, name, category, description, tile: PlannedTile, shape: 'square', planned: true,
  title: () => `${name} (not built yet)`,
})

export const widgets: WidgetDef[] = [
  {
    id: 'start', name: 'Start', category: 'apps', description: 'Opens Start. Right-click opens the Windows quick-link (Win+X) menu.',
    tile: StartTile, shape: 'square',
    click: () => { ipc.startMenu() },
    rightClick: () => { ipc.winx() },
  },
  {
    id: 'app', name: 'App', category: 'apps', description: 'A pinned app, file, folder or shortcut.',
    tile: AppTile, shape: 'square',
    title: (item) => item.options.name ?? 'App',
    // Like a taskbar pin: switch to the running window (minimise if in front), else launch.
    click: (item) => {
      const wins = windowsForPinned(item.options.path ?? '', running.value)
      const target = wins.find((w) => w.foreground) ?? wins[0]
      if (target) invoke('window_activate', { hwnd: target.hwnd, toggle: true })
      else ipc.launch(item.options.path, item.options.args).catch(notice)
    },
    middle: (item) => { ipc.launch(item.options.path, item.options.middleArgs ?? item.options.args) },
  },
  {
    id: 'launchpad', name: 'Launchpad', category: 'apps', description: 'Search and open your apps.',
    tile: LaunchpadTile, card: LaunchpadCard, shape: 'square',
  },
  {
    id: 'notes', name: 'Notes', category: 'productivity', description: 'Your latest notes; click to edit.',
    tile: NotesTile, card: NotesCard, shape: 'square',
  },
  {
    id: 'komorebi-workspaces', name: 'Workspaces', category: 'komorebi', description: "Komorebi workspaces on this dock's monitor.",
    tile: WorkspacesTile, shape: 'wide',
    click: () => {},
    wheel: (_i, delta) => { ipc.komorebi('cycle-workspace', delta > 0 ? 'next' : 'previous') },
    menu: (item) => {
      const st = item.options.style ?? 'dots', pal = item.options.palette ?? 'theme'
      return [
        { label: 'Dots', action: 'style-dots', checked: st === 'dots' },
        { label: 'Names', action: 'style-names', checked: st === 'names' },
        { label: 'Numbers', action: 'style-numbers', checked: st === 'numbers' },
        { separator: true },
        { label: 'Accent colours', action: 'palette-theme', checked: pal === 'theme' },
        { label: 'Pastel colours', action: 'palette-pastel', checked: pal === 'pastel' },
        { separator: true },
        { label: 'Start komorebi', action: 'k-start' }, { label: 'Restart komorebi', action: 'k-restart' },
        { label: 'Stop komorebi', action: 'k-stop', danger: true },
      ]
    },
    onMenu: (item, a, ctx) => {
      if (a.startsWith('style-')) ctx.setOptions(item, { style: a.slice(6) })
      else if (a.startsWith('palette-')) ctx.setOptions(item, { palette: a.slice(8) })
      else ipc.komorebiControl(a.slice(2) as 'start' | 'stop' | 'restart')
    },
  },
  {
    id: 'komorebi-layout', name: 'Layout', category: 'komorebi',
    description: 'Current komorebi layout. Click for all layouts and toggles; middle-click next, right-click previous.',
    tile: LayoutTile, card: LayoutCard, shape: 'wide',
    click: (item, ctx, el) => { ctx.openCard(item, el, { hmonitor: ctx.layout.hmonitor }) },
    middle: (_i, ctx) => stepLayout(ctx, 1),
    rightClick: (_i, ctx) => { stepLayout(ctx, -1) },
  },
  {
    id: 'clock', name: 'Clock', category: 'clocks', description: 'Time and date; click for the calendar.',
    tile: ClockTile, card: ClockCard, shape: 'wide',
  },
  {
    id: 'weather', name: 'Weather', category: 'weather', description: 'Current conditions; click for the next hours and the week.',
    tile: WeatherTile, card: WeatherCard, shape: 'wide',
  },
  {
    id: 'power', name: 'Power', category: 'system', description: 'Lock, sleep, hibernate, sign out, restart, shut down.',
    tile: PowerTile, card: PowerCard, shape: 'square',
    // Default: full-screen panel. Option style: 'card' = compact card over the tile.
    click: (item) => { if (item.options.style === 'card') return true; ipc.overlayOpen('power') },
  },
  {
    id: 'apps', name: 'Apps', category: 'apps', description: 'Running apps, like taskbar buttons. Click to switch or minimise; middle-click for a new window.',
    tile: AppsTile, card: AppsCard, shape: 'wide',
    click: () => {},
    onMenu: (item, action) => {
      const [verb, key] = action.split('|')
      const g = groupWindows(running.value).find((x) => x.key === key)
      if (!g) return
      const target = g.key.startsWith('aumid:') ? g.icon : g.exe
      if (verb === 'new') ipc.launch(target)
      else if (verb === 'close') g.windows.forEach((w) => invoke('window_close', { hwnd: w.hwnd }))
      else if (verb === 'pin') {
        const items = settings.value?.items ?? []
        const at = items.findIndex((i) => i.uid === item.uid)
        const pin = { uid: 'pin' + Date.now(), widget: 'app', options: { path: target, name: g.name } }
        ipc.setItems([...items.slice(0, at), pin, ...items.slice(at)])
      }
    },
  },
  {
    id: 'tray', name: 'Tray', category: 'system', description: 'Your notification-area icons in a grid; click, right-click and double-click like the taskbar.',
    tile: TrayTile, card: TrayCard, shape: 'square',
  },
  {
    id: 'now-playing', name: 'Now Playing', category: 'media', description: 'Cover art and controls for any app that reports to Windows.',
    tile: MediaTile, card: MediaCard, shape: 'wide',
    // Nothing playing: the click opens the last player (Spotify by default) instead of an empty card.
    click: () => {
      if (media.value?.active) return true
      const t = idleTarget()
      if (t) invoke('launch', { target: t })
    },
    menu: (item) => [{ label: 'Lyrics mode (lrclib.net)', action: 'lyrics', checked: !!item.options.lyrics }],
    onMenu: (item, a, ctx) => { if (a === 'lyrics') ctx.setOptions(item, { lyrics: !item.options.lyrics }) },
  },
  {
    id: 'wallpaper-gallery', name: 'Wallpaper Gallery', category: 'wallpaper', description: 'Wallpaper Engine and your pictures in one gallery; one click to apply, per screen.',
    tile: WallpaperTile, card: WallpaperCard, shape: 'square',
  },
  {
    id: 'audio-output', name: 'Audio Output', category: 'media', description: 'Volume and output devices; scroll to change volume, middle-click to mute.',
    tile: AudioTile, card: AudioCard, shape: 'wide',
    wheel: (_i, delta) => { const v = audio.value?.volume ?? 0; invoke('audio_set_volume', { volume: Math.max(0, Math.min(100, v - delta * 2)) }) },
    middle: () => { invoke('audio_set_mute', { muted: !audio.value?.muted }) },
    menu: (item) => [{ label: audio.value?.muted ? 'Unmute' : 'Mute', action: 'mute' }, { label: 'Sound settings…', action: 'sound' }],
    onMenu: (_i, a) => { if (a === 'mute') invoke('audio_set_mute', { muted: !audio.value?.muted }); else if (a === 'sound') ipc.launch('ms-settings:sound') },
  },
  {
    id: 'wifi', name: 'Network', category: 'system', description: 'Ethernet or Wi-Fi, internet status, nearby networks to join.',
    tile: WifiTile, card: WifiCard, shape: 'wide',
  },
  { id: 'cpu', name: 'CPU', category: 'system', description: 'Processor use (like Task Manager); click for top processes.', tile: SysTile, card: SysCard, shape: 'wide' },
  { id: 'ram', name: 'RAM', category: 'system', description: 'Memory in use; click for the biggest apps.', tile: SysTile, card: SysCard, shape: 'wide' },
  { id: 'top-cpu', name: 'Top CPU', category: 'system', description: 'The app using the most CPU right now.', tile: SysTile, card: SysCard, shape: 'wide' },
  { id: 'top-ram', name: 'Top RAM', category: 'system', description: 'The app using the most memory right now.', tile: SysTile, card: SysCard, shape: 'wide' },
  { id: 'network', name: 'Network', category: 'system', description: 'Download and upload speed with a live chart.', tile: SysTile, card: SysCard, shape: 'wide' },
  { id: 'storage', name: 'Storage', category: 'system', description: 'Free space on your drives, including USB.', tile: SysTile, card: SysCard, shape: 'wide' },
  { id: 'show-desktop', name: 'Show Desktop', category: 'system', description: 'Minimise everything (Win+D).', tile: ActTile, shape: 'square', click: () => { invoke('sys_action', { action: 'show-desktop' }) } },
  { id: 'notifications', name: 'Notifications', category: 'system', description: 'Open Notification Center (Win+N).', tile: ActTile, shape: 'square', click: () => { invoke('sys_action', { action: 'notifications' }) } },
  { id: 'keyboard', name: 'Keyboard', category: 'system', description: 'On-Screen Keyboard.', tile: ActTile, shape: 'square', click: () => { invoke('sys_action', { action: 'keyboard' }) } },
  { id: 'theme-toggle', name: 'Theme Toggle', category: 'system', description: 'Switch Windows between light and dark.', tile: ActTile, shape: 'square', click: () => { toggleTheme() } },
  { id: 'microphone', name: 'Microphone', category: 'system', description: 'Mute or unmute your microphone for all apps.', tile: ActTile, shape: 'square', click: () => { toggleMic() } },
  { id: 'recycle-bin', name: 'Recycle Bin', description: 'Item count; click to empty or open.', tile: ActTile, card: ActCard, shape: 'square', category: 'apps' as const },
  { id: 'language', name: 'Language', category: 'system', description: 'Current keyboard layout; click to switch.', tile: ActTile, card: ActCard, shape: 'square' },
  { id: 'battery', name: 'Battery', category: 'system', description: 'Charge level (shows AC on desktops).', tile: ActTile, shape: 'wide' },
  { id: 'day-progress', name: 'Day Progress', category: 'system', description: 'How much of today has passed; click for hours left and a countdown.', tile: ActTile, card: ProgressCard, shape: 'wide' },
  { id: 'month-progress', name: 'Month Progress', category: 'system', description: 'How much of this month has passed; click for days left and a countdown.', tile: ActTile, card: ProgressCard, shape: 'wide' },
  { id: 'year-progress', name: 'Year Progress', category: 'system', description: 'The year as a dot grid; click for quarters, months and a countdown.', tile: ActTile, card: ProgressCard, shape: 'wide' },
  { id: 'flip-clock', name: 'Flip Clock', category: 'clocks', description: 'Hours and minutes on flipping cards.', tile: ClockVTile, card: ClockVCard, shape: 'wide' },
  { id: 'analog', name: 'Analog Minimal', category: 'clocks', description: 'A small analog face.', tile: ClockVTile, card: ClockVCard, shape: 'square' },
  { id: 'world-clocks', name: 'World Clocks', category: 'clocks', description: 'Three analog faces for other time zones.', tile: ClockVTile, card: ClockVCard, shape: 'wide' },
  { id: 'world-map', name: 'World Map', category: 'clocks', description: 'Day and night across the world, with city times.', tile: ClockVTile, card: MapCard, shape: 'wide' },
  { id: 'date', name: 'Date', category: 'clocks', description: 'Weekday and date.', tile: ClockVTile, card: ClockVCard, shape: 'wide' },
  { id: 'day-number', name: 'Day Number', category: 'clocks', description: 'Today as a big number.', tile: ClockVTile, card: ClockVCard, shape: 'square' },
  { id: 'week', name: '7 Days', category: 'clocks', description: 'This week, today highlighted.', tile: ClockVTile, card: ClockVCard, shape: 'wide' },
  { id: 'todos', name: 'Todos', category: 'productivity', description: 'Your next task with progress; tick it off from the dock.', tile: ProdTile, card: ProdCard, shape: 'wide' },
  { id: 'pomodoro', name: 'Pomodoro', category: 'productivity', description: 'Focus timer: start and skip from the bar.', tile: ProdTile, card: ProdCard, shape: 'wide' },
  { id: 'timer', name: 'Timer', category: 'productivity', description: 'One-click countdown with saved durations.', tile: ProdTile, card: ProdCard, shape: 'wide' },
  { id: 'stopwatch', name: 'Stopwatch', category: 'productivity', description: 'Start, pause and lap from the bar.', tile: ProdTile, card: ProdCard, shape: 'wide' },
  { id: 'currency', name: 'Currency', category: 'markets', description: 'Exchange rate and converter (ECB rates).', tile: MarketTile, card: MarketCard, shape: 'wide' },
  { id: 'crypto', name: 'Crypto', category: 'markets', description: 'Binance prices: BTC/USDT plus up to three pairs.', tile: MarketTile, card: MarketCard, shape: 'wide' },
  { id: 'search', name: 'Web Search', category: 'apps', description: 'Search the web or open a folder by typing its path.', tile: SearchTileC as Component<any>, card: SearchCard, shape: 'square' },
  { id: 'everything', name: 'Everything', category: 'apps', description: 'Instant file search powered by your running Everything (voidtools).', tile: EvTile, card: EvCard, shape: 'square' },
  { id: 'flow-launcher', name: 'Flow Launcher', category: 'apps', description: 'Opens the Flow Launcher search box.', tile: EvTile, shape: 'square', click: () => { invoke('flow_toggle') } },
  { id: 'now-playing-art', name: 'Now Playing Art', category: 'media', description: 'The cover art blurred behind the title and controls.', tile: MediaVariants, card: MediaCard, shape: 'wide' },
  { id: 'now-playing-compact', name: 'Now Playing Compact', category: 'media', description: 'Cover art and title.', tile: MediaVariants, card: MediaCard, shape: 'wide' },
  { id: 'now-playing-mini', name: 'Now Playing Mini', category: 'media', description: 'Round play button with a progress ring.', tile: MediaVariants, card: MediaCard, shape: 'square', click: () => true },
  { id: 'now-playing-bar', name: 'Now Playing Bar', category: 'media', description: 'Title over a progress bar, with skip.', tile: MediaVariants, card: MediaCard, shape: 'wide' },
  { id: 'temperature', name: 'Temperature', category: 'system', description: 'CPU/GPU temperature gauge with a selectable sensor and ten-minute history (via LibreHardwareMonitor).', tile: SysXTile, card: SysXCard, shape: 'wide' },
  { id: 'battery-ring', name: 'Battery Ring', category: 'system', description: 'Ring with an animated spark while plugged in.', tile: SysXTile, card: SysXCard, shape: 'square' },
  { id: 'battery-core', name: 'Battery Core', category: 'system', description: 'Twelve cells.', tile: SysXTile, card: SysXCard, shape: 'wide' },
  { id: 'battery-dots', name: 'Battery Dots', category: 'system', description: 'Five dots.', tile: SysXTile, card: SysXCard, shape: 'wide' },
  { id: 'battery-gauge', name: 'Battery Gauge', category: 'system', description: 'E to F gauge.', tile: SysXTile, card: SysXCard, shape: 'wide' },
  { id: 'brightness', name: 'Brightness', category: 'system', description: 'Main monitor brightness (DDC/CI); scroll to change.', tile: SysXTile, card: SysXCard, shape: 'wide', wheel: (i, d, ctx) => { invoke<number>('brightness_get').then((v) => invoke('brightness_set', { value: Math.max(0, Math.min(100, v - d * 5)) })).catch(() => { const b = Math.max(40, Math.min(100, (i.options.sb ?? 100) - d * 5)); ctx.setOptions(i, { ...i.options, sb: b, soft: true }) }) } },
  { id: 'do-not-disturb', name: 'Do Not Disturb', category: 'system', description: 'Silence notification banners.', tile: SysXTile, shape: 'square', click: () => { invoke<{ id: string; on: boolean }[]>('toggles_get').then((t) => invoke('toggle_set', { id: 'dnd', on: !t.find((x) => x.id === 'dnd')?.on }).then(() => window.dispatchEvent(new Event('toggles:changed')))) } },
  { id: 'quick-toggles', name: 'Quick Toggles', category: 'system', description: 'Wi-Fi, Bluetooth, Do Not Disturb, microphone, camera, keep awake, desktop icons and more.', tile: SysXTile, card: SysXCard, shape: 'wide' },
  { id: 'recently-opened', name: 'Recently Opened', category: 'apps', description: 'Files and folders you opened lately.', tile: FilesTile, card: FilesCard, shape: 'square' },
  { id: 'downloads', name: 'Downloads', category: 'apps', description: 'Newest downloads; the badge counts today\'s.', tile: FilesTile, card: FilesCard, shape: 'square' },
  { id: 'screenshots', name: 'Screenshots', category: 'apps', description: 'Your latest screenshots as thumbnails.', tile: FilesTile, card: FilesCard, shape: 'square' },
  // Replaced by Start Search (apps, settings, files, web); kept so docks that have it keep working.
  { id: 'commands', name: 'Commands', category: 'apps', description: 'Replaced by Start Search.', tile: StartSearchTile, card: StartSearchCard, shape: 'square', hidden: true },
  { id: 'start-search', name: 'Start Search', category: 'apps', description: 'Search apps, settings, documents and the web, like the search in Start.', tile: StartSearchTile, card: StartSearchCard, shape: 'square' },
  { id: 'terminal', name: 'Terminal', category: 'apps', description: 'Type a command and it opens in a terminal, already running (Ctrl+Enter: as administrator).', tile: TermTile, card: TermCard, shape: 'square',
    middle: () => { invoke('terminal_run', { command: '', shell: 'powershell', admin: false, cwd: null }) } },
  { id: 'cloud-storage', name: 'Cloud Storage', category: 'office', description: 'Your cloud accounts (Google Drive, Digi Storage) in one tile; click for space and files, then open either one.', tile: CloudTile, card: CloudCard, shape: 'square' },
  { id: 'cloud-storage-wide', name: 'Cloud Storage (wide)', category: 'office', description: 'Each account with its usage percentage.', tile: CloudTile, card: CloudCard, shape: 'wide' },
  { id: 'cloud-storage-large', name: 'Cloud Storage (large)', category: 'office', description: 'Each account with a usage bar and space used of total.', tile: CloudTile, card: CloudCard, shape: 'wide' },
  { id: 'screenshot-capture', name: 'Screenshot Capture', category: 'apps', description: 'Window, region and screen buttons on the dock; right-click for options (text recognition, saving).', tile: CaptureTile, card: FilesCard, shape: 'wide', click: () => {}, rightClick: (item, ctx, el) => { ctx.openCard(item, el); return false } },
  { id: 'weather-card', name: 'Weather Card', category: 'weather', description: 'Conditions and city on a sky-coloured card with a large temperature and a colour icon.', tile: WeatherVariants, card: WeatherCard, shape: 'wide' },
  { id: 'weather-details', name: 'Weather Details', category: 'weather', description: 'Temperature, feels-like, humidity and wind.', tile: WeatherVariants, card: WeatherCard, shape: 'wide' },
  { id: 'today-forecast', name: 'Today & Forecast', category: 'weather', description: 'Today and the next three days.', tile: WeatherVariants, card: WeatherCard, shape: 'wide' },
  { id: 'weather-status', name: 'Weather Status', category: 'weather', description: 'Conditions in words with high and low.', tile: WeatherVariants, card: WeatherCard, shape: 'wide' },
  { id: 'hourly', name: 'Hourly', category: 'weather', description: 'The next six hours.', tile: WeatherVariants, card: WeatherCard, shape: 'wide' },
  { id: 'sunrise-sunset', name: 'Sunrise & Sunset', category: 'weather', description: 'Today\'s sun arc with both times.', tile: WeatherVariants, card: WeatherCard, shape: 'wide' },
  { id: 'world-time', name: 'World Time', category: 'clocks', description: 'Digital times for your cities.', tile: ClockMore, card: ClockVCard, shape: 'wide' },
  { id: 'today', name: 'Today', category: 'clocks', description: 'Weekday, date and how much of today has passed.', tile: ClockMore, card: ClockVCard, shape: 'wide' },
  { id: 'segment-clock', name: 'Segment Clock', category: 'clocks', description: 'Seven-segment digits in your accent colour.', tile: ClockMore, card: ClockVCard, shape: 'wide' },
  { id: 'time-date-weather', name: 'Time, Date & Weather', category: 'clocks', description: 'Time on a red block, the weekday and date, and the temperature.', tile: ClockMore, card: ClockVCard, shape: 'wide' },
  { id: 'clock-weather', name: 'Clock & Weather', category: 'clocks', description: 'Time with the current temperature.', tile: ClockMore, card: ClockVCard, shape: 'wide' },
  { id: 'calendar', name: 'Calendar', category: 'productivity', description: 'Google, Outlook and iCloud via secret iCal links, plus AI limit resets.', tile: Prod2Tile, card: Prod2Card, shape: 'wide' },
  { id: 'screen-time', name: 'Screen Time', category: 'productivity', description: 'Hour by hour, per app, today and this week.', tile: Prod2Tile, card: Prod2Card, shape: 'wide' },
  { id: 'day-timeline', name: 'Day Timeline', category: 'productivity', description: 'Today by app type, meetings and sunrise/sunset on one line.', tile: Prod2Tile, card: Prod2Card, shape: 'wide' },
  { id: 'whatsapp', name: 'WhatsApp', category: 'productivity', description: 'Unread count and last sender, from Windows notifications.', tile: Prod2Tile, card: Prod2Card, shape: 'wide' },
  { id: 'drives', name: 'Drives', category: 'system', description: 'Every drive at once, including USB.', tile: SysTile, card: SysCard, shape: 'wide' },
  { id: 'quick-utilities', name: 'Quick Utilities', category: 'apps', description: 'Terminal, PowerShell, Snipping Tool, Task Manager and other Windows tools.', tile: FilesTile, card: FilesCard, shape: 'square' },
  { id: 'dock-menu', name: 'Dock Menu', category: 'apps', description: 'Opens the dock menu (Widgets, Settings, Quit) without hunting for empty space.', tile: DockMenuTile, shape: 'square', click: (_i, _c, el) => { window.dispatchEvent(new CustomEvent('strata:dockmenu', { detail: el })) } },
  { id: 'meeting-notes', name: 'Meeting Notes', category: 'productivity', description: 'Agenda, notes and action items per meeting, with a timer; export as Markdown.', tile: MeetTile, card: MeetCard, shape: 'square', title: () => 'Meeting Notes' },
  { id: 'office', name: 'Microsoft Office', category: 'office', description: 'Word, Excel, PowerPoint, Outlook… one click each; pinned and recent documents.', tile: SuiteTile, card: SuiteCard, shape: 'wide', menu: (item) => [{ label: 'Show app icons', action: 'style', checked: item.options.style === 'apps' }], onMenu: (item, a, ctx) => { if (a === 'style') ctx.setOptions(item, { style: item.options.style === 'apps' ? 'icon' : 'apps' }) } },
  { id: 'google-workspace', name: 'Google Workspace', category: 'office', description: 'Docs, Sheets, Slides, Drive, Gmail, Calendar; create new, pinned and recent files.', tile: SuiteTile, card: SuiteCard, shape: 'wide', menu: (item) => [{ label: 'Show app icons', action: 'style', checked: item.options.style === 'apps' }], onMenu: (item, a, ctx) => { if (a === 'style') ctx.setOptions(item, { style: item.options.style === 'apps' ? 'icon' : 'apps' }) } },
  {
    id: 'local-app', name: 'Project Launcher', category: 'apps',
    description: 'Your own apps and GitHub projects in one place: add the ones you choose, then start, open and stop them.',
    tile: LocalAppTile, card: LocalAppCard, shape: 'square',
    title: (item) => { const l = projectsOf(item); return l.length === 1 ? l[0].name : 'Projects' },
  },

  { id: 'vault', name: 'Vault', category: 'productivity', description: 'Passwords and API keys, one click to copy. Unlocks with Windows Hello; stored in Credential Manager.', tile: VaultTile, card: VaultCard, shape: 'square' },
]

// Online widgets: one Tile/Card pair driven by adapters (online/adapters/*).
const DESC: Record<string, string> = {"ai-limits":"Two chosen AI plans on one tile, with reset times.","claude-usage":"Claude session limit and reset, from your Claude Code sign-in.","claude-plan":"Claude session and weekly limits, most-used model.","chatgpt-usage":"Codex 5-hour and weekly limits, from your Codex sign-in.","gemini-usage":"Antigravity session and weekly limits.","grok-usage":"SuperGrok credits.","deepseek-balance":"DeepSeek balance and spend (pasted key).","glm-usage":"Z.ai GLM coding plan usage (pasted key).","opencode-usage":"Current OpenCode session from local history.","opencode-plan":"OpenCode tokens this week and top model.","cursor-usage":"Cursor usage (preview with sample data).","total-revenue":"30 days across your stores, monthly goal and pace.","github":"Unread GitHub notifications.","github-activity":"Contribution grid and streak.","github-overview":"Notifications, open PRs and stars.","linear":"Your open Linear issues.","supabase":"Supabase project health.","resend":"Emails sent and bounces.","dub":"Link clicks over 30 days.","plausible":"Visitors, pageviews and bounce rate.","datafast-chart":"DataFast visitors chart.","leetcode":"Solved problems by difficulty and streak.","search-console":"28-day clicks, impressions, top queries.","google-analytics":"GA4 users, sessions, pages, sources, realtime.","meta-ads":"Ad spend, clicks, CTR and campaigns.","x-profile":"Followers and posts.","youtube":"Subscribers, views and latest videos.","instagram":"Followers and recent posts.","tiktok":"Followers, likes and videos.","gmail":"Unread count; inbox, unread or starred with search.","yahoo-mail":"Unread count; inbox, unread or flagged with search (read-only).","stock-chart":"Intraday price chart for one ticker.","stock-watchlist":"Three tickers, refreshed every minute.","google-drive":"Storage used and your recent, starred or shared Drive files.","digi-storage":"Digi Storage (RCS & RDS) space used and your latest files."}
const ANALYTICS = new Set(['github', 'github-activity', 'github-overview', 'linear', 'supabase', 'resend', 'dub', 'plausible', 'datafast-chart', 'leetcode', 'search-console', 'google-analytics', 'meta-ads'])
const onlineDefs: WidgetDef[] = [
  ...Object.entries(AI).map(([id, a]) => [id, a, 'ai'] as const),
  ...Object.entries(FINANCE).map(([id, a]) => [id, a, 'finance'] as const),
  ...Object.entries(DEV).map(([id, a]) => [id, a, ANALYTICS.has(id) ? 'analytics' : 'social'] as const),
  ...Object.entries(SOCIAL).map(([id, a]) => [id, a, 'social'] as const),
  ...Object.entries(MARKETS).map(([id, a]) => [id, a, 'markets'] as const),
  ...Object.entries(CLOUD).map(([id, a]) => [id, a, 'office'] as const),
].map(([id, a, category]) => ({
  id, name: a.title, category: category as WidgetDef['category'],
  description: DESC[id] ?? (id.endsWith('-chart') ? 'Weekly revenue, change and a 14-day line.' : 'Today, week and 30-day revenue (read-only key).'),
  tile: OnlineTile, card: OnlineCard, shape: 'wide' as const,
  // Merged into Claude Usage (which now shows the session and the weekly limit).
  hidden: id === 'claude-plan',
}))

// Later definitions replace placeholders with the same id.
for (const d of onlineDefs) { const i = widgets.findIndex((w) => w.id === d.id); if (i >= 0) widgets.splice(i, 1, d); else widgets.push(d) }

export const registry: Record<string, WidgetDef> = Object.fromEntries(widgets.map((w) => [w.id, w]))
