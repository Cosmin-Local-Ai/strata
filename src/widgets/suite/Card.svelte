<script lang="ts">
  // Microsoft Office / Google Workspace card: every app, "create new", your pinned documents (files for
  // Office, links for Google) and recent ones (Office: Windows' recent items; Google: Drive, when the
  // Google sign-in is connected). Choose which apps sit on the dock tile.
  import { Check, ExternalLink, FilePlus2, Pin, Plus, Settings2, X } from '@lucide/svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { iconUrl, ipc } from '../../lib/ipc'
  import { ago, api } from '../../lib/services.svelte'
  import type { CardProps } from '../types'
  import { officeApps } from './apps.svelte'
  import { docColor, GOOGLE, GOOGLE_DEFAULT, OFFICE_DEFAULT, OFFICE_EXT, OFFICE_FILTER, type Doc } from './suite'
  let { item, act, close }: CardProps = $props()
  const google = item.widget === 'google-workspace'
  const office = officeApps()

  let chosen = $state<string[]>(item.options.apps ?? (google ? GOOGLE_DEFAULT : OFFICE_DEFAULT))
  let docs = $state<Doc[]>(item.options.docs ?? [])
  let editing = $state(false)
  let err = $state('')
  const saveOpts = () => act('set-options', { apps: $state.snapshot(chosen), docs: $state.snapshot(docs) })
  $effect(() => { invoke('card_sticky', { on: editing }); return () => { invoke('card_sticky', { on: false }) } })

  function toggle(id: string) { chosen = chosen.includes(id) ? chosen.filter((x) => x !== id) : [...chosen, id]; saveOpts() }
  async function launch(target: string) { try { await ipc.launch(target); close() } catch (e) { err = String(e) } }

  // ---- pinned documents
  let newUrl = $state(''), newName = $state('')
  async function addFile() {
    const path = await invoke<string | null>('pick_file', { save: false, title: 'Pin a document', filters: OFFICE_FILTER, defaultName: null })
    if (!path) return
    docs = [...docs.filter((d) => d.path !== path), { path, name: path.split(/[\\/]/).pop() ?? path }]; saveOpts()
  }
  function addLink() {
    const u = newUrl.trim()
    if (!/^https:\/\//.test(u)) { err = 'Paste a link starting with https://'; return }
    docs = [...docs, { path: u, name: newName.trim() || u.replace(/^https:\/\/(www\.)?/, '').slice(0, 40) }]; newUrl = newName = ''; err = ''; saveOpts()
  }
  function unpin(d: Doc) { docs = docs.filter((x) => x !== d); saveOpts() }
  function pinRecent(path: string, name: string) { if (!docs.some((d) => d.path === path)) { docs = [...docs, { path, name }]; saveOpts() } }

  // ---- recent
  type R = { path: string; name: string; modified: number; sub?: string; href?: string }
  let recent = $state<R[]>([])
  let recentNote = $state('')
  if (!google) {
    invoke<{ path: string; name: string; modified: number }[]>('folder_list', { which: 'recent', limit: 300 })
      .then((l) => (recent = l.filter((e) => OFFICE_EXT.test(e.name)).slice(0, 12)))
      .catch((e) => (recentNote = String(e)))
  } else {
    invoke<boolean>('google_status').then(async (on) => {
      if (!on) { recentNote = 'Connect the Google Drive (or Gmail) widget to see your recent Docs, Sheets and Slides here.'; return }
      const q = encodeURIComponent("trashed = false and (mimeType contains 'vnd.google-apps.document' or mimeType contains 'vnd.google-apps.spreadsheet' or mimeType contains 'vnd.google-apps.presentation' or mimeType contains 'vnd.google-apps.form')")
      try {
        const r = await api<any>('google', `https://www.googleapis.com/drive/v3/files?pageSize=12&orderBy=${encodeURIComponent('viewedByMeTime desc')}&q=${q}&fields=${encodeURIComponent('files(id,name,mimeType,viewedByMeTime,modifiedTime,webViewLink)')}`)
        recent = (r.files ?? []).map((f: any) => ({ path: f.webViewLink, name: f.name, modified: Date.parse(f.viewedByMeTime ?? f.modifiedTime), sub: /spreadsheet/.test(f.mimeType) ? 'Sheet' : /presentation/.test(f.mimeType) ? 'Slides' : /form/.test(f.mimeType) ? 'Form' : 'Doc' }))
      } catch { recentNote = 'Sign in to Google again (Drive widget) to allow the recent-files list.' }
    })
  }
  const gColor = (sub?: string) => sub === 'Sheet' ? '#0f9d58' : sub === 'Slides' ? '#f4b400' : sub === 'Form' ? '#7248b9' : '#4285f4'
</script>

<div class="sc">
  <div class="hd">
    <strong>{google ? 'Google Workspace' : 'Microsoft Office'}</strong>
    <button class="ic" class:on={editing} title="Choose the apps on the dock" onclick={() => (editing = !editing)}>{#if editing}<Check size={14} />{:else}<Settings2 size={14} />{/if}</button>
  </div>
  {#if editing}<p class="hint">Tick the apps you want on the dock tile.</p>{/if}

  <div class="apps">
    {#if google}
      {#each GOOGLE as a (a.id)}
        <button class="app" onclick={() => (editing ? toggle(a.id) : launch(a.url))} style="--c:{a.color}">
          <span class="ico"><a.icon size={20} strokeWidth={2} /></span><span>{a.name}</span>
          {#if editing}<span class="tick" class:on={chosen.includes(a.id)}><Check size={10} /></span>{/if}
        </button>
      {/each}
    {:else}
      {#each office.value as a (a.id)}
        <button class="app" onclick={() => (editing ? toggle(a.id) : launch(a.target))}>
          <span class="ico"><img src={iconUrl(a.target, 64)} alt="" /></span><span>{a.name}</span>
          {#if editing}<span class="tick" class:on={chosen.includes(a.id)}><Check size={10} /></span>{/if}
        </button>
      {:else}
        <p class="hint">No Microsoft Office apps found on this PC.</p>
      {/each}
    {/if}
  </div>

  {#if google}
    <h4>Create</h4>
    <div class="row">
      {#each GOOGLE.filter((a) => a.create) as a (a.id)}<button class="chip" style="--c:{a.color}" onclick={() => launch(a.create!)}><FilePlus2 size={12} /> {a.name}</button>{/each}
    </div>
  {/if}

  <h4>Pinned <span class="sp"></span>
    {#if !google}<button class="link" onclick={addFile}><Plus size={12} /> Add file</button>{/if}
  </h4>
  {#if google}
    <form class="add" onsubmit={(e) => { e.preventDefault(); addLink() }}>
      <input bind:value={newName} placeholder="Name" />
      <input bind:value={newUrl} placeholder="Paste a Docs / Sheets / Drive link" spellcheck="false" />
      <button type="submit" aria-label="Pin"><Plus size={14} /></button>
    </form>
  {/if}
  <ul>
    {#each docs as d (d.path)}
      <li>
        <button class="doc" onclick={() => launch(d.path)} title={d.path}>
          {#if google}<span class="dot" style="--c:{gColor(/spreadsheets/.test(d.path) ? 'Sheet' : /presentation/.test(d.path) ? 'Slides' : /forms/.test(d.path) ? 'Form' : 'Doc')}"></span>{:else}<img src={iconUrl(d.path, 32)} alt="" onerror={(e) => ((e.currentTarget as HTMLImageElement).style.visibility = 'hidden')} />{/if}
          <span class="nm">{d.name}</span>
        </button>
        <button class="x" aria-label="Unpin" onclick={() => unpin(d)}><X size={12} /></button>
      </li>
    {:else}
      <li class="empty">{google ? 'Pin the Docs and Sheets you open most.' : 'Pin the documents and spreadsheets you open most.'}</li>
    {/each}
  </ul>

  <h4>Recent</h4>
  <ul>
    {#each recent as r (r.path)}
      <li>
        <button class="doc" onclick={() => launch(r.path)} title={r.name}>
          {#if google}<span class="dot" style="--c:{gColor(r.sub)}"></span>{:else}<span class="dot" style="--c:{docColor(r.name)}"></span>{/if}
          <span class="nm">{r.name}</span><small>{r.sub ? `${r.sub} · ` : ''}{ago(r.modified)}</small>
        </button>
        <button class="x" aria-label="Pin" title="Pin" onclick={() => pinRecent(r.path, r.name)}><Pin size={12} /></button>
      </li>
    {:else}
      <li class="empty">{recentNote || 'Nothing opened lately.'}</li>
    {/each}
  </ul>
  {#if google}<button class="link" onclick={() => launch('https://drive.google.com/drive/recent')}><ExternalLink size={12} /> All recent files in Drive</button>{/if}
  {#if err}<p class="err">{err}</p>{/if}
</div>

<style>
  .sc { width: 380px; max-height: 620px; overflow-y: auto; padding: 14px; display: flex; flex-direction: column; gap: 8px; }
  .hd { display: flex; align-items: center; justify-content: space-between; }
  .hd strong { font-size: 14px; }
  .ic { width: 28px; height: 28px; border-radius: 8px; display: grid; place-items: center; background: var(--surface); }
  .ic.on { background: var(--accent); color: white; }
  .hint { margin: 0; font-size: 11.5px; color: var(--fg-3); }
  .apps { display: grid; grid-template-columns: repeat(4, 1fr); gap: 4px; }
  .app { position: relative; display: flex; flex-direction: column; align-items: center; gap: 4px; padding: 8px 2px; border-radius: 10px; font-size: 11px; color: var(--fg-2); }
  .app:hover { background: var(--surface-hover); color: var(--fg); }
  .ico { width: 34px; height: 34px; border-radius: 9px; display: grid; place-items: center; color: var(--c, var(--fg)); background: color-mix(in srgb, var(--c, transparent) 14%, transparent); }
  .ico img { width: 30px; height: 30px; object-fit: contain; }
  .tick { position: absolute; top: 4px; right: 10px; width: 15px; height: 15px; border-radius: 8px; display: grid; place-items: center; border: 1.5px solid var(--fg-3); color: transparent; }
  .tick.on { background: var(--accent); border-color: var(--accent); color: white; }
  h4 { margin: 4px 0 0; font-size: 11.5px; font-weight: 600; color: var(--fg-3); text-transform: uppercase; letter-spacing: .04em; display: flex; align-items: center; }
  .sp { flex: 1; }
  .row { display: flex; flex-wrap: wrap; gap: 4px; }
  .chip { display: flex; align-items: center; gap: 4px; padding: 4px 9px; border-radius: 999px; font-size: 11.5px; background: color-mix(in srgb, var(--c) 16%, transparent); color: var(--fg); }
  .chip:hover { background: color-mix(in srgb, var(--c) 28%, transparent); }
  ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 1px; }
  li { display: flex; align-items: center; border-radius: 8px; }
  li:hover { background: var(--surface-hover); }
  li.empty { font-size: 11.5px; color: var(--fg-3); padding: 4px 6px; }
  li.empty:hover { background: none; }
  .doc { flex: 1; min-width: 0; display: flex; align-items: center; gap: 8px; padding: 6px; text-align: left; }
  .doc img { width: 18px; height: 18px; flex: none; }
  .dot { width: 10px; height: 13px; border-radius: 2px; background: var(--c); flex: none; margin: 0 4px; }
  .nm { flex: 1; min-width: 0; font-size: 12.5px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .doc small { font-size: 11px; color: var(--fg-3); flex: none; }
  .x { width: 24px; height: 24px; border-radius: 6px; display: grid; place-items: center; color: var(--fg-3); opacity: 0; flex: none; margin-right: 4px; }
  li:hover .x { opacity: 1; }
  .x:hover { background: var(--surface-press); color: var(--fg); }
  .link { color: var(--accent); font-size: 11.5px; display: inline-flex; align-items: center; gap: 4px; text-transform: none; letter-spacing: 0; font-weight: 500; }
  .add { display: flex; gap: 4px; }
  .add input { flex: 1; min-width: 0; padding: 6px 8px; border-radius: 8px; border: 1px solid var(--border); background: var(--surface); outline: none; font-size: 12px; color: var(--fg); }
  .add input:first-child { flex: 0 0 90px; }
  .add button { width: 30px; border-radius: 8px; display: grid; place-items: center; background: var(--surface); }
  .err { margin: 0; color: var(--danger); font-size: 12px; }
</style>
