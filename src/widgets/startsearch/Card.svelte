<script lang="ts">
  // Search, like the one in Start: apps (everything the Start menu lists), Settings pages, files and
  // folders (Everything when it runs, else your main folders and recent items), and the web. Type,
  // arrow keys to move, Enter to open; Ctrl+Enter opens a file's folder or runs an app as admin.
  import { AppWindow, File, Folder, Globe, Search, Settings2 } from '@lucide/svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { iconUrl, ipc } from '../../lib/ipc'
  import type { CardProps } from '../types'
  import { SETTINGS } from './settings'
  let { close }: CardProps = $props()

  type App = { name: string; target: string }
  type FileHit = { path: string; name: string; folder: boolean }
  type Row = { kind: 'app' | 'setting' | 'file' | 'web'; name: string; sub: string; target: string; icon?: string; folder?: boolean }

  let q = $state('')
  let input = $state<HTMLInputElement>()
  let apps = $state<App[]>([])
  let files = $state<FileHit[]>([])
  let sel = $state(0)
  let everything = false
  $effect(() => { input?.focus() })
  invoke<App[]>('start_apps').then((v) => (apps = v))
  invoke<boolean>('everything_running').then((v) => (everything = v)).catch(() => {})

  // Recently launched from here float to the top of empty and matching results.
  // Folders that hold Windows', programs' or tools' files rather than yours.
  const SKIP_ROOT = new Set(['windows', 'program files', 'program files (x86)', 'programdata', 'system volume information', '$recycle.bin', 'recovery', 'msocache'])
  const SKIP_ANY = new Set(['appdata', 'node_modules', '.git', '.cache', 'target', '.venv', '__pycache__'])
  // Like Windows Search: your own folders. Inside your user folder anything but AppData and hidden
  // folders; on other drives anything outside program, game and tool folders. No real path: skipped.
  let home = ''
  invoke<string>('folder_path', { which: 'downloads' }).then((d) => (home = d.replace(/[\\/][^\\/]+$/, '').toLowerCase())).catch(() => {})
  const GAMES = new Set(['steam', 'steamlibrary', 'steamapps', 'epic games', 'riot games', 'windowsapps', 'xboxgames', 'games', 'ruby34-x64', 'python', 'msys64', 'cygwin64'])
  const SYSTEM = { test: (p: string) => {
    const l = p.toLowerCase()
    if (!/^[a-z]:[\\/]/.test(l)) return true
    const parts = l.split(/[\\/]/)
    if (parts.slice(1, -1).some((x) => SKIP_ANY.has(x) || x.startsWith('.'))) return true
    if (home && l.startsWith(home + '\\')) return false
    if (SKIP_ROOT.has(parts[1] ?? '') || GAMES.has(parts[1] ?? '')) return true
    // Elsewhere on the system drive (e.g. C:\Tools): only if the first folder isn't a program's.
    return l.startsWith(home.slice(0, 2)) && parts.length > 2 && !/^(dev|projects|work|documents|data)$/.test(parts[1])
  } }
  const KEY = 'startsearch:recent'
  const recent: string[] = (() => { try { return JSON.parse(localStorage.getItem(KEY) ?? '[]') } catch { return [] } })()

  /** 0 = no match; higher = better (whole start > word start > anywhere, then initials). */
  function score(name: string, extra = ''): number {
    const n = name.toLowerCase(), t = q.trim().toLowerCase()
    if (!t) return 0
    if (n === t) return 100
    if (n.startsWith(t)) return 90 - Math.min(20, n.length - t.length) / 4
    if (n.split(/[\s\-_.]+/).some((w) => w.startsWith(t))) return 70
    const initials = n.split(/[\s\-_.]+/).map((w) => w[0]).join('')
    if (t.length >= 2 && initials.startsWith(t)) return 60
    if (n.includes(t)) return 50
    // Keywords match at word starts only ("word" must not find "password").
    if (extra && t.split(/\s+/).every((w) => extra.split(' ').some((k) => k.startsWith(w)))) return 30
    return 0
  }
  const bump = (s: number, target: string) => s + (recent.includes(target) ? 15 - recent.indexOf(target) : 0)

  const appRows = $derived(apps.map((a) => ({ a, s: bump(score(a.name), a.target) })).filter((x) => x.s > 0).sort((x, y) => y.s - x.s).slice(0, 6)
    .map(({ a }): Row => ({ kind: 'app', name: a.name, sub: 'App', target: a.target, icon: a.target })))
  const setRows = $derived(SETTINGS.map((p) => ({ p, s: score(p.name, p.words) })).filter((x) => x.s > 0).sort((x, y) => y.s - x.s).slice(0, 4)
    .map(({ p }): Row => ({ kind: 'setting', name: p.name, sub: p.uri.startsWith('ms-settings') ? 'Settings' : 'System tool', target: p.uri })))
  const fileRows = $derived(files.slice(0, 6).map((f): Row => ({ kind: 'file', name: f.name, sub: f.path.replace(/\\[^\\]*$/, ''), target: f.path, icon: f.folder ? undefined : f.path, folder: f.folder })))
  const webRow = $derived<Row[]>(q.trim() ? [{ kind: 'web', name: q.trim(), sub: 'Search the web', target: `https://www.google.com/search?q=${encodeURIComponent(q.trim())}` }] : [])

  // Best match: the top app, else the top setting.
  const sections = $derived.by(() => {
    if (!q.trim()) {
      const r = recent.map((t) => apps.find((a) => a.target === t)).filter(Boolean).slice(0, 8) as App[]
      return [{ title: 'Recent', rows: r.map((a): Row => ({ kind: 'app', name: a.name, sub: 'App', target: a.target, icon: a.target })) }]
    }
    const best = appRows[0] ?? setRows[0]
    const out: { title: string; rows: Row[] }[] = []
    if (best) out.push({ title: 'Best match', rows: [best] })
    const a = appRows.filter((r) => r !== best), st = setRows.filter((r) => r !== best)
    if (a.length) out.push({ title: 'Apps', rows: a })
    if (st.length) out.push({ title: 'Settings', rows: st })
    if (fileRows.length) out.push({ title: 'Documents and folders', rows: fileRows })
    out.push({ title: 'Web', rows: webRow })
    return out
  })
  const flat = $derived(sections.flatMap((s) => s.rows))
  $effect(() => { void q; sel = 0 })

  // Files: debounced, Everything first.
  let ft: ReturnType<typeof setTimeout> | undefined
  $effect(() => {
    const t = q.trim()
    clearTimeout(ft)
    if (t.length < 2) { files = []; return }
    ft = setTimeout(async () => {
      try {
        const r = everything ? await invoke<FileHit[]>('everything_search', { query: t, max: 60 }) : await invoke<FileHit[]>('quick_files', { query: t, max: 8 })
        // Your files, not Windows' own: skip system, program and app-data folders.
        // Names match at word starts, like Windows Search ("word" finds "Word report", not "password").
        const words = t.toLowerCase().split(/\s+/)
        const tokens = (n: string) => n.replace(/([a-z])([A-Z])/g, '$1 $2').toLowerCase().split(/[^a-z0-9]+/).filter(Boolean)
        const fits = (n: string) => { const tk = tokens(n); return words.every((w) => tk.some((x) => x.startsWith(w))) }
        if (q.trim() === t) files = r.filter((f) => !SYSTEM.test(f.path) && fits(f.name)).slice(0, 8)
      } catch { files = [] }
    }, 140)
  })

  async function open(r: Row, alt = false) {
    try {
      if (r.kind === 'app') {
        localStorage.setItem(KEY, JSON.stringify([r.target, ...recent.filter((x) => x !== r.target)].slice(0, 12)))
        if (alt) await invoke('launch_admin', { target: r.target }); else await ipc.launch(r.target)
      } else if (r.kind === 'file' && alt) await invoke('reveal', { target: r.target })
      else if (r.kind === 'setting' && r.target.includes(' ')) { const [exe, ...args] = r.target.split(' '); await ipc.launch(exe, args.join(' ')) }
      else await ipc.launch(r.target)
      close()
    } catch { /* the launch reports its own error */ }
  }
  function key(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') { e.preventDefault(); sel = Math.min(flat.length - 1, sel + 1) }
    else if (e.key === 'ArrowUp') { e.preventDefault(); sel = Math.max(0, sel - 1) }
    else if (e.key === 'Enter' && flat[sel]) { e.preventDefault(); open(flat[sel], e.ctrlKey) }
    else if (e.key === 'Escape') close()
  }
  const indexOf = (r: Row) => flat.indexOf(r)
</script>

<div class="ss">
  <label class="bar"><Search size={16} /><input bind:this={input} bind:value={q} onkeydown={key} placeholder="Search apps, settings and documents" spellcheck="false" autocomplete="off" /></label>
  {#each sections as s (s.title)}
    {#if s.rows.length}
      <h4>{s.title}</h4>
      <ul class:best={s.title === 'Best match'}>
        {#each s.rows as r (r.kind + r.target)}
          <li>
            <button class:sel={indexOf(r) === sel} onmouseenter={() => (sel = indexOf(r))} onclick={(e) => open(r, e.ctrlKey)}>
              <span class="ico">
                {#if r.kind === 'app' || (r.kind === 'file' && r.icon)}<img src={iconUrl(r.icon!, s.title === 'Best match' ? 64 : 32)} alt="" onerror={(e) => ((e.currentTarget as HTMLImageElement).style.visibility = 'hidden')} />
                {:else if r.kind === 'setting'}<Settings2 size={16} />
                {:else if r.kind === 'web'}<Globe size={16} />
                {:else if r.folder}<Folder size={16} />
                {:else}<File size={16} />{/if}
              </span>
              <span class="tx"><strong>{r.name}</strong><small>{r.sub}</small></span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  {/each}
  {#if !q.trim() && !flat.length}<p class="hint"><AppWindow size={13} /> Start typing: apps, Settings pages, files, or a web search.</p>{/if}
</div>

<style>
  .ss { width: 460px; max-height: 620px; overflow-y: auto; padding: 14px; display: flex; flex-direction: column; gap: 6px; }
  .bar { display: flex; align-items: center; gap: 9px; padding: 9px 12px; border-radius: 12px; background: var(--surface); color: var(--fg-3); border: 1px solid var(--border); }
  .bar:focus-within { border-color: var(--accent); }
  .bar input { flex: 1; background: none; border: 0; outline: none; color: var(--fg); font-size: 14px; }
  h4 { margin: 6px 4px 0; font-size: 11px; font-weight: 600; color: var(--fg-3); text-transform: uppercase; letter-spacing: .04em; }
  ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 1px; }
  li button { width: 100%; display: flex; align-items: center; gap: 10px; padding: 6px 8px; border-radius: 9px; text-align: left; }
  li button.sel { background: var(--surface-hover); }
  .ico { width: 28px; height: 28px; display: grid; place-items: center; flex: none; color: var(--fg-2); }
  .ico img { width: 24px; height: 24px; object-fit: contain; }
  ul.best li button { padding: 10px 8px; }
  ul.best .ico { width: 40px; height: 40px; }
  ul.best .ico img { width: 36px; height: 36px; }
  ul.best strong { font-size: 14.5px; }
  .tx { min-width: 0; display: flex; flex-direction: column; }
  .tx strong { font-size: 13px; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .tx small { font-size: 11px; color: var(--fg-3); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .hint { margin: 4px; font-size: 12px; color: var(--fg-3); display: flex; gap: 6px; align-items: center; }
</style>
