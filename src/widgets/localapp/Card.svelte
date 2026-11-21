<script lang="ts">
  // Project Launcher card: every project with Open / Start / Stop, and the editor. Add a project from
  // a GitHub link (cloned with git, nothing is run) or from a folder already on this PC; Strata
  // suggests how to start it (its own start/stop script, Docker Compose, launch.bat, npm, Python) and
  // where it listens. Everything stays editable.
  import { AppWindow, ArrowLeft, ExternalLink, FolderOpen, GitBranch, LoaderCircle, Pencil, Play, Plus, Rocket, Square, Trash2, Wand2 } from '@lucide/svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { iconUrl, ipc } from '../../lib/ipc'
  import type { CardProps } from '../types'
  import { iconSrc, poll, projectsOf, run, projState, type Project } from './state.svelte'
  let { item, act, close }: CardProps = $props()
  const list = $derived(projectsOf(item))
  $effect(() => { list.forEach((p) => poll(item.uid, p.id)) })

  let edit = $state<Project | null>(null)
  let err = $state('')
  let working = $state('')
  $effect(() => { invoke('card_sticky', { on: !!edit }); return () => { invoke('card_sticky', { on: false }) } })
  const blank = (): Project => ({ id: 'p' + Date.now().toString(36), name: '', folder: '', start: '', stop: '', url: '', port: null, process: '', icon: '', repo: '' })

  function save(next: Project[]) {
    // Writing the list replaces an older single-project item's own fields.
    act('set-options', { projects: next, name: null, start: null, stop: null, url: null, port: null, process: null, icon: null })
  }
  function saveEdit() {
    if (!edit) return
    const p = { ...$state.snapshot(edit), name: edit.name.trim(), port: edit.port ? +edit.port : null }
    if (!p.name || !p.start.trim()) { err = 'A name and a start command are needed'; return }
    save([...list.filter((x) => x.id !== p.id), p])
    edit = null; err = ''
    setTimeout(() => poll(item.uid, p.id), 500)
  }
  function remove() { if (edit) { save(list.filter((x) => x.id !== edit!.id)); edit = null } }

  async function detect(folder: string) {
    working = 'Looking at the project…'; err = ''
    try {
      const d = await invoke<Project>('project_detect', { folder })
      if (!edit) return
      edit = { ...edit, folder: d.folder, name: edit.name || d.name, start: d.start || edit.start, stop: d.stop || edit.stop, url: d.url || edit.url, port: d.port ?? edit.port, icon: d.icon || edit.icon }
      if (!d.start) err = 'No start command found: type the one from the project\'s README.'
    } catch (e) { err = String(e) } finally { working = '' }
  }
  async function pickFolder() {
    const f = await invoke<string | null>('pick_folder', { title: 'Choose the project folder' })
    if (f && edit) { edit.folder = f; await detect(f) }
  }
  async function cloneRepo() {
    if (!edit?.repo.trim()) { err = 'Paste the GitHub link first'; return }
    const parent = await invoke<string | null>('pick_folder', { title: 'Where should the project go? (e.g. C:\\Dev)' })
    if (!parent) return
    working = 'Downloading from GitHub…'; err = ''
    try { const folder = await invoke<string>('project_clone', { url: edit.repo.trim(), parent }); working = ''; edit.folder = folder; await detect(folder) }
    catch (e) { err = String(e); working = '' }
  }
  async function pickIcon() {
    const p = await invoke<string | null>('pick_file', { save: false, title: 'Choose an icon', filters: [['Icons and pictures', '*.ico;*.png;*.jpg;*.svg;*.exe;*.lnk'], ['All files', '*.*']], defaultName: null })
    if (p && edit) edit.icon = p
  }
  // ---- an installed app (from the same list as Start): open it, see whether it runs, close it.
  type StartApp = { name: string; target: string; exe: string }
  let appPick = $state(false), appQ = $state(''), apps = $state<StartApp[]>([])
  async function showApps() { appPick = !appPick; if (appPick && !apps.length) apps = await invoke<StartApp[]>('start_apps') }
  const appList = $derived(apps.filter((a) => !appQ.trim() || a.name.toLowerCase().includes(appQ.trim().toLowerCase())).slice(0, 40))
  function chooseApp(a: StartApp) {
    if (!edit) return
    const exe = a.exe ? a.exe.split(/[\\/]/).pop() ?? '' : ''
    edit = { ...edit, name: a.name, start: a.target, stop: '', folder: '', url: '', port: null, process: exe, icon: a.target, repo: '' }
    appPick = false; err = exe ? '' : "This app doesn't tell Windows which program it runs, so Strata can open it but can't see whether it runs."
  }
  const statusText = (s: ReturnType<typeof projState>) => s.busy === 'starting' ? 'Starting…' : s.busy === 'stopping' ? 'Stopping…' : s.running ? 'Running' : 'Stopped'
</script>

<div class="pc">
  {#if edit}
    <div class="hd"><button class="ic" aria-label="Back" onclick={() => { edit = null; err = '' }}><ArrowLeft size={15} /></button><strong>{list.some((x) => x.id === edit!.id) ? 'Edit project' : 'Add a project'}</strong></div>
    <div class="pick">
      <span>An app installed on this PC</span>
      <button class="btn" onclick={showApps}><AppWindow size={13} /> {appPick ? 'Hide' : 'Choose…'}</button>
    </div>
    {#if appPick}
      <div class="apps">
        <input bind:value={appQ} placeholder="Search your apps" spellcheck="false" />
        <ul>
          {#each appList as a (a.target)}<li><button class="ap" onclick={() => chooseApp(a)}><img src={iconUrl(a.target, 32)} alt="" />{a.name}</button></li>{/each}
        </ul>
      </div>
    {/if}
    <p class="or">or a project</p>
    <label>From GitHub (optional)
      <span class="row"><input bind:value={edit.repo} placeholder="https://github.com/owner/repository" spellcheck="false" /><button class="btn" onclick={cloneRepo} disabled={!!working}><GitBranch size={13} /> Download</button></span>
    </label>
    <label>Project folder
      <span class="row"><input bind:value={edit.folder} placeholder="C:\Dev\My project" spellcheck="false" /><button class="btn" onclick={pickFolder} disabled={!!working}><FolderOpen size={13} /> Choose</button><button class="btn" title="Suggest the commands again" onclick={() => edit && detect(edit.folder)} disabled={!!working || !edit.folder}><Wand2 size={13} /></button></span>
    </label>
    {#if working}<p class="hint"><span class="spin"><LoaderCircle size={12} /></span> {working}</p>{/if}
    <label>Name<input bind:value={edit.name} placeholder="e.g. Trading Journal" /></label>
    <label>Start command<input bind:value={edit.start} placeholder="docker compose up -d   ·   npm run dev   ·   start.ps1 start" spellcheck="false" /></label>
    <label>Stop command (optional: without one, Strata stops what listens on the port, or closes the app)<input bind:value={edit.stop} placeholder="docker compose down" spellcheck="false" /></label>
    <div class="two">
      <label>Address to open<input bind:value={edit.url} placeholder="http://localhost:3010" spellcheck="false" /></label>
      <label>Port<input value={edit.port ?? ''} oninput={(e) => { const v = e.currentTarget.value.trim(); edit!.port = v ? +v || null : null; if (v && !edit!.url) edit!.url = `http://localhost:${v}` }} placeholder="3010" inputmode="numeric" /></label>
    </div>
    <label>Icon (optional)<span class="row"><input bind:value={edit.icon} placeholder="An .ico, .png or .exe" spellcheck="false" /><button class="btn" onclick={pickIcon}>…</button></span></label>
    <p class="hint">Commands run hidden from the project folder; output goes to %APPDATA%\Strata\projects. Docker projects need Docker Desktop running.</p>
    {#if err}<p class="err">{err}</p>{/if}
    <div class="btns">
      {#if list.some((x) => x.id === edit!.id)}<button class="btn danger" onclick={remove}><Trash2 size={13} /> Remove</button>{/if}
      <span class="sp"></span>
      <button class="btn" onclick={() => { edit = null; err = '' }}>Cancel</button>
      <button class="btn primary" onclick={saveEdit}>Save</button>
    </div>
  {:else}
    <div class="hd"><Rocket size={16} /><strong>Projects</strong><span class="sp"></span><button class="btn" onclick={() => (edit = blank())}><Plus size={13} /> Add</button></div>
    <ul>
      {#each list as p (p.id)}
        {@const s = projState(item.uid, p.id)}
        <li>
          <span class="pi">{#if p.icon}<img src={iconSrc(p.icon, iconUrl)} alt="" onerror={(e) => ((e.currentTarget as HTMLImageElement).style.display = 'none')} />{:else}<Rocket size={16} />{/if}<i class="dot" class:on={s.running} class:busy={!!s.busy}></i></span>
          <span class="tx"><strong>{p.name}</strong><small>{statusText(s)}{p.url ? ` · ${p.url.replace(/^https?:\/\//, '')}` : ''}</small>{#if s.error}<small class="err">{s.error}</small>{/if}</span>
          {#if s.running && p.url}<button class="btn primary" onclick={() => { ipc.launch(p.url); close() }}><ExternalLink size={13} /> Open</button>{/if}
          {#if s.running || s.busy === 'starting'}
            <button class="btn danger" disabled={s.busy === 'stopping'} onclick={() => run(item.uid, p.id, 'stop')}><Square size={11} /> Stop</button>
          {:else}
            <button class="btn" class:primary={!p.url} disabled={!!s.busy} onclick={() => run(item.uid, p.id, 'start')}><Play size={12} /> Start</button>
          {/if}
          <button class="ic" aria-label="Edit {p.name}" title="Settings" onclick={() => (edit = { ...p })}><Pencil size={13} /></button>
        </li>
      {:else}
        <li class="empty">Add the apps you run yourself, e.g. a project from GitHub. Start, open and stop them from here.</li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .pc { width: 420px; max-height: 620px; overflow-y: auto; padding: 14px; display: flex; flex-direction: column; gap: 9px; }
  .hd { display: flex; align-items: center; gap: 8px; }
  .hd strong { font-size: 14px; }
  .sp { flex: 1; }
  .ic { width: 30px; height: 30px; border-radius: 8px; display: grid; place-items: center; background: var(--surface); color: var(--fg-2); flex: none; transition: background var(--t-fast), color var(--t-fast); }
  .ic:hover { background: var(--surface-hover); color: var(--fg); }
  .ic:active { background: var(--surface-press); }
  .btn { display: inline-flex; align-items: center; gap: 5px; padding: 6px 11px; border-radius: 8px; background: var(--surface); font-size: 12.5px; white-space: nowrap; flex: none;
         transition: background var(--t-fast), color var(--t-fast), filter var(--t-fast); }
  .btn:hover:not(:disabled) { background: var(--surface-hover); }
  .btn:active:not(:disabled) { background: var(--surface-press); }
  .btn:disabled { opacity: 0.45; }
  .btn.primary { background: var(--accent); color: white; }
  .btn.primary:hover:not(:disabled) { filter: brightness(1.12); background: var(--accent); }
  .btn.danger:hover:not(:disabled) { background: var(--danger); color: white; }
  ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 2px; }
  li { display: flex; align-items: center; gap: 8px; padding: 7px 6px; border-radius: 10px; transition: background var(--t-fast); }
  li:hover { background: var(--surface-hover); }
  li.empty { font-size: 12px; color: var(--fg-3); line-height: 1.45; }
  li.empty:hover { background: none; }
  .pi { position: relative; width: 32px; height: 32px; border-radius: 9px; background: var(--surface); display: grid; place-items: center; flex: none; }
  .pi img { width: 24px; height: 24px; object-fit: contain; border-radius: 5px; }
  .dot { position: absolute; bottom: -1px; right: -1px; width: 9px; height: 9px; border-radius: 5px; background: var(--fg-3); box-shadow: 0 0 0 2px var(--card-bg, #1c1c20); }
  .dot.on { background: #30d158; }
  .dot.busy { background: #ff9f0a; }
  .tx { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .tx strong { font-size: 13px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .tx small { font-size: 11px; color: var(--fg-3); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 11.5px; color: var(--fg-2); flex: 1; min-width: 0; }
  input { padding: 7px 9px; border-radius: 8px; border: 1px solid var(--border); background: var(--surface); outline: none; font-size: 12.5px; color: var(--fg); min-width: 0; flex: 1; }
  input:focus { border-color: var(--accent); }
  .row { display: flex; gap: 4px; }
  .two { display: flex; gap: 8px; }
  .two label:last-child { flex: 0 0 90px; }
  .btns { display: flex; gap: 6px; align-items: center; }
  .hint { margin: 0; font-size: 11px; color: var(--fg-3); line-height: 1.45; display: flex; align-items: center; gap: 6px; }
  .spin { display: inline-grid; animation: r 0.9s linear infinite; color: var(--accent); }
  @keyframes r { to { transform: rotate(360deg); } }
  .pick { display: flex; align-items: center; justify-content: space-between; font-size: 11.5px; color: var(--fg-2); }
  .or { margin: 0; font-size: 11px; color: var(--fg-3); text-transform: uppercase; letter-spacing: .05em; text-align: center; }
  .apps { display: flex; flex-direction: column; gap: 4px; padding: 6px; border-radius: 10px; background: var(--surface); }
  .apps ul { max-height: 200px; overflow-y: auto; }
  .ap { width: 100%; display: flex; align-items: center; gap: 8px; padding: 5px 6px; border-radius: 7px; font-size: 12.5px; text-align: left; transition: background var(--t-fast); }
  .ap:hover { background: var(--surface-hover); }
  .ap img { width: 18px; height: 18px; object-fit: contain; }
  .err { margin: 0; color: var(--danger) !important; font-size: 12px; }
</style>
