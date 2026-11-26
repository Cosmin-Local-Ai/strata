// Project Launcher: your projects and their running state (polled while shown). The list lives in
// %APPDATA%\Strata\projects.json, not in the widget: removing the widget (or adding a second one)
// keeps it.
import { invoke } from '@tauri-apps/api/core'
import type { DockItem } from '../../lib/ipc'
import { dataFile } from '../../lib/stores.svelte'

export type Project = { id: string; name: string; folder: string; start: string; stop: string; url: string; port: number | null; process: string; icon: string; repo: string }

export const projects = dataFile<{ list: Project[] }>('projects', { list: [] })

/** Your projects (from projects.json). */
export function projectsOf(_item?: DockItem): Project[] {
  return projects.value?.list ?? []
}

/** Projects an older Strata kept in the widget's own options (moved to projects.json once). */
export function legacyProjects(item: DockItem): Project[] {
  const o = item.options
  if (Array.isArray(o.projects)) return o.projects as Project[]
  if (o.start) return [{ id: 'main', name: o.name ?? 'App', folder: o.folder ?? '', start: o.start ?? '', stop: o.stop ?? '', url: o.url ?? '', port: o.port ? +o.port : null, process: o.process ?? '', icon: o.icon ?? '', repo: '' }]
  return []
}

// A plain Map of reactive objects: creating an entry is not a state change, so state() may be called
// from templates and $derived (creating reactive state there throws in Svelte 5).
class S {
  running = $state(false)
  busy = $state<'' | 'starting' | 'stopping'>('')
  error = $state('')
}
const states = new Map<string, S>()
export function projState(uid: string, pid: string): S {
  const k = `${uid}:${pid}`
  let s = states.get(k)
  if (!s) { s = new S(); states.set(k, s) }
  return s
}

export async function poll(uid: string, pid: string) {
  const s = projState(uid, pid)
  try {
    const r = await invoke<boolean>('project_action', { uid, project: pid, action: 'status' })
    if (s.busy === 'starting' && r) s.busy = ''
    if (s.busy === 'stopping' && !r) s.busy = ''
    s.running = r
  } catch { /* not saved yet */ }
}

export async function run(uid: string, pid: string, action: 'start' | 'stop') {
  const s = projState(uid, pid)
  s.error = ''
  s.busy = action === 'start' ? 'starting' : 'stopping'
  try { await invoke('project_action', { uid, project: pid, action }) } catch (e) { s.error = String(e); s.busy = ''; return }
  // A server can take minutes to come up the first time (Docker builds, asset builds); stop waiting after 6.
  const until = Date.now() + 6 * 60_000
  const t = setInterval(async () => { await poll(uid, pid); if (!s.busy || Date.now() > until) { s.busy = ''; clearInterval(t) } }, 2000)
}

/** Pictures load as files (the shell would show the file type's icon); exe / ico / lnk via the icon extractor. */
export function iconSrc(p: string, iconUrl: (p: string, s?: number) => string) {
  if (!p) return ''
  return /\.(png|jpe?g|webp|gif|svg)$/i.test(p) ? `http://thumb.localhost/${encodeURIComponent(p)}?raw=1` : iconUrl(p, 64)
}
