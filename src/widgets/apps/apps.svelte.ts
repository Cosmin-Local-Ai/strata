import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'

export type AppWindow = { hwnd: number; title: string; exe: string; aumid: string; minimized: boolean; foreground: boolean; cloaked: boolean }
export type AppGroup = { key: string; name: string; icon: string; exe: string; windows: AppWindow[]; active: boolean; allCloaked: boolean }

export const running = $state<{ value: AppWindow[] }>({ value: [] })
let started = false
export function startWindows() {
  if (started) return
  started = true
  invoke<AppWindow[]>('windows_list').then((l) => (running.value = l))
  listen<AppWindow[]>('windows:list', (e) => (running.value = e.payload))
}

const base = (p: string) => p.split('\\').pop() ?? p
/** Packaged (Store) apps are identified by AUMID; Win32 apps by exe path. */
export function groupKey(w: AppWindow) {
  return w.aumid.includes('!') ? 'aumid:' + w.aumid.toLowerCase() : 'exe:' + w.exe.toLowerCase()
}

/** Name used to match a pinned path (exe or shortcut, env vars unexpanded) with running windows:
 *  "C:\\Program Files\\App\\app.exe" -> "app", "...\\Some App.lnk" -> "some app". */
export function matchName(path: string) {
  return (path.split(/[\\/]/).pop() ?? '').replace(/\.(exe|lnk)$/i, '').toLowerCase()
}
/** What to launch so a running app opens a *new window* instead of just coming to the front:
 *  browsers, editors and Windows Terminal need an argument for it; others open one when started again. */
export function newWindow(exe: string, fallback = exe): [string, string | undefined] {
  const n = matchName(exe)
  if (n === 'windowsterminal' || n === 'wt') return ['wt.exe', '-w new']
  if (['chrome', 'msedge', 'brave', 'vivaldi', 'opera', 'chromium', 'code', 'cursor', 'windsurf'].includes(n)) return [exe, '--new-window']
  if (['firefox', 'librewolf', 'zen', 'waterfox', 'floorp'].includes(n)) return [exe, '-new-window']
  return [fallback, undefined]
}

/** Running windows belonging to a pinned path. */
export function windowsForPinned(path: string, list: AppWindow[]) {
  // Store apps pinned by their Start entry: match the window's app ID.
  const id = /^shell:appsfolder[\\/](.+)$/i.exec(path)?.[1]?.toLowerCase()
  if (id) return list.filter((w) => w.aumid.toLowerCase() === id)
  const n = matchName(path)
  if (!n) return []
  return list.filter((w) => {
    const e = matchName(w.exe)
    return e === n || e.startsWith(n + '.') // App.lnk -> App.DesktopApp.exe
  })
}

/** First-seen order of app groups, so icons never jump when you switch between apps. */
const seen: string[] = []

export function groupWindows(list: AppWindow[]): AppGroup[] {
  const map = new Map<string, AppGroup>()
  for (const w of list) {
    const key = groupKey(w)
    let g = map.get(key)
    if (!g) {
      const packaged = key.startsWith('aumid:')
      g = {
        key, exe: w.exe, windows: [], active: false, allCloaked: true,
        icon: packaged ? `shell:AppsFolder\\${w.aumid}` : w.exe,
        name: packaged ? w.title : base(w.exe).replace(/\.exe$/i, ''),
      }
      map.set(key, g)
    }
    g.windows.push(w)
    g.active ||= w.foreground
    g.allCloaked &&= w.cloaked
  }
  for (const k of map.keys()) if (!seen.includes(k)) seen.push(k)
  return [...map.values()].sort((a, b) => seen.indexOf(a.key) - seen.indexOf(b.key))
}
