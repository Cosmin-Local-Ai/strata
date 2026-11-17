<script lang="ts">
  // Wallpaper Gallery (brief §6): Wallpaper Engine items, your pictures and Unsplash in one place; one
  // click applies either kind on the chosen screen, with a clean hand-off (see gallery.rs).
  // Auto-change: daily / weekly from Wallpaper Engine, pictures or both.
  import { Check, ExternalLink, FolderOpen, ListMusic, Monitor, Pause, Play, Search, Shuffle, SkipForward, Volume2, VolumeX, X } from '@lucide/svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { ipc } from '../../lib/ipc'
  import { settings } from '../../lib/stores.svelte'
  import type { CardProps } from '../types'

  type WeItem = { id: string; title: string; kind: string; preview: string; file: string; tags: string[] }
  type State = { weInstalled: boolean; weRunning: boolean; items: WeItem[]; weCurrent: string[]; winCurrent: string[]; monitors: string[]; playlists: string[] }
  type Photo = { id: string; alt_description: string | null; urls: { small: string; raw: string }; links: { download_location: string; html: string }; user: { name: string; links: { html: string } } }
  let { item, act }: CardProps = $props()

  let st = $state<State | null>(null)
  let tab = $state<'we' | 'pictures' | 'unsplash'>('we')
  let q = $state('')
  let kind = $state('all')
  let monitor = $state<number | null>(item.options.monitor ?? 0)
  let folder = $state<string>(item.options.folder ?? '%USERPROFILE%\\Pictures')
  let pictures = $state<string[]>([])
  let busy = $state<string | null>(null)
  let msg = $state('')

  // Unsplash (your own Access Key, kept in Windows Credential Manager).
  let usConnected = $state<boolean | null>(null)
  let usKey = $state('')
  let usQuery = $state('')
  let photos = $state<Photo[]>([])
  const sched = $derived(settings.value?.wallpaperSchedule ?? { mode: 'off', source: 'both', monitor: 0, folder: '', last: 0 })

  const load = async () => { st = await invoke<State>('gallery_state'); if (!st.weInstalled && tab === 'we') tab = 'pictures' }
  const loadPictures = async () => { pictures = await invoke<string[]>('gallery_pictures', { folder }) }
  load()
  invoke<boolean>('secret_has', { name: 'unsplash' }).then((k) => (usConnected = k))
  $effect(() => { if (tab === 'pictures') loadPictures() })
  $effect(() => { if (tab === 'unsplash' && usConnected && photos.length === 0) usSearch() })

  const thumb = (p: string, w = 320) => `http://thumb.localhost/${encodeURIComponent(p)}?w=${w}`
  const shown = $derived((st?.items ?? []).filter((i) => (kind === 'all' || i.kind === kind) && (!q || i.title.toLowerCase().includes(q.toLowerCase()))))
  const dirOf = (p: string) => p.toLowerCase().replace(/[\\/][^\\/]*$/, '')
  const isCurrentWe = (i: WeItem) => (st?.weCurrent ?? []).some((c, m) => (monitor === null || m === monitor) && c && dirOf(c) === dirOf(i.file))
  const isCurrentPic = (p: string) => (st?.winCurrent ?? []).some((c, m) => (monitor === null || m === monitor) && c.toLowerCase() === p.toLowerCase())

  async function apply(kindArg: 'we' | 'picture', file: string, key: string) {
    busy = key; msg = ''
    try { await invoke('gallery_apply', { kind: kindArg, file, monitor }) } catch (e) { msg = String(e) }
    busy = null
    setTimeout(load, 800)
  }
  const counts = $derived(['scene', 'video', 'web', 'application'].reduce((a, k) => ({ ...a, [k]: (st?.items ?? []).filter((i) => i.kind === k).length }), {} as Record<string, number>))
  async function we(action: string) {
    msg = ''
    try { await invoke('we_control', { action, monitor }) } catch (e) { msg = String(e) }
    setTimeout(load, 800)
  }
  async function playlist(name: string) {
    msg = ''
    try { await invoke('we_playlist', { name, monitor }) } catch (e) { msg = String(e) }
  }
  function setMonitor(m: number | null) { monitor = m; act('set-options', { monitor: m }) }
  function setFolder() { act('set-options', { folder }); loadPictures() }
  const monLabel = (i: number) => (i === 0 ? 'Main' : `Display ${i + 1}`)

  async function usConnect() {
    msg = ''
    const key = usKey.trim()
    if (!key) return
    await invoke('secret_set', { name: 'unsplash', value: key })
    try {
      // Validation read: one real request with the key.
      await invoke('http_get', { url: 'https://api.unsplash.com/photos/random?count=1&orientation=landscape', auth: 'unsplash' })
      usConnected = true; usKey = ''; usSearch()
    } catch (e) {
      await invoke('secret_delete', { name: 'unsplash' })
      msg = String(e)
    }
  }
  async function usDisconnect() { await invoke('secret_delete', { name: 'unsplash' }); usConnected = false; photos = [] }
  async function usSearch() {
    msg = ''
    try {
      const url = usQuery.trim()
        ? `https://api.unsplash.com/search/photos?per_page=24&orientation=landscape&query=${encodeURIComponent(usQuery.trim())}`
        : 'https://api.unsplash.com/photos/random?count=24&orientation=landscape'
      const r: any = await invoke('http_get', { url, auth: 'unsplash' })
      photos = Array.isArray(r) ? r : r.results
    } catch (e) { msg = String(e) }
  }
  async function usApply(p: Photo) {
    busy = p.id; msg = ''
    try {
      // Unsplash guideline: trigger the download endpoint when a photo is used.
      await invoke('http_get', { url: p.links.download_location, auth: 'unsplash' })
      const path = await invoke<string>('download_wallpaper', { url: `${p.urls.raw}&w=3840&q=85&fm=jpg`, name: `unsplash-${p.id}` })
      await invoke('gallery_apply', { kind: 'picture', file: path, monitor })
    } catch (e) { msg = String(e) }
    busy = null
  }
  function setSched(patch: Record<string, unknown>) {
    invoke('settings_update', { patch: { wallpaperSchedule: { ...sched, monitor, folder, ...patch } } })
  }
</script>

<div class="wg">
  <header>
    <div class="tabs" role="tablist">
      <button class:on={tab === 'we'} onclick={() => (tab = 'we')}>Wallpaper Engine</button>
      <button class:on={tab === 'pictures'} onclick={() => (tab = 'pictures')}>Pictures</button>
      <button class:on={tab === 'unsplash'} onclick={() => (tab = 'unsplash')}>Unsplash</button>
    </div>
  </header>
  <div class="screenrow">
    <span class="lbl"><Monitor size={14} /> Screen</span>
    <div class="mons">
      {#each st?.monitors ?? [] as _, i}<button class:on={monitor === i} onclick={() => setMonitor(i)}>{monLabel(i)}</button>{/each}
      <button class:on={monitor === null} onclick={() => setMonitor(null)}>All</button>
    </div>
  </div>

  {#if tab === 'we'}
    {#if st && !st.weInstalled}
      <p class="muted">Wallpaper Engine isn't installed (Steam). Pictures and Unsplash still work.</p>
    {:else}
      <div class="filters">
        <label class="search"><Search size={14} /><input bind:value={q} placeholder="Search {st?.items.length ?? ''} wallpapers" /></label>
        <button class="chip" class:on={kind === 'all'} onclick={() => (kind = 'all')}>All {st?.items.length ?? ''}</button>
        {#each ['scene', 'video', 'web', 'application'] as k}{#if counts[k]}<button class="chip" class:on={kind === k} onclick={() => (kind = k)}>{k} {counts[k]}</button>{/if}{/each}
      </div>
      <div class="controls">
        {#if st && !st.weRunning}<span class="muted small">Wallpaper Engine isn't running; it starts when you pick a wallpaper.</span>
        {:else}
          <button title="Play" onclick={() => we('play')}><Play size={14} /></button>
          <button title="Pause" onclick={() => we('pause')}><Pause size={14} /></button>
          <button title="Mute" onclick={() => we('mute')}><VolumeX size={14} /></button>
          <button title="Unmute" onclick={() => we('unmute')}><Volume2 size={14} /></button>
          <button title="Next wallpaper (in a playlist)" onclick={() => we('next')}><SkipForward size={14} /></button>
          <button title="Remove the Wallpaper Engine wallpaper from this screen" onclick={() => we('close')}><X size={14} /></button>
          {#if st?.playlists.length}<span class="sep"></span><ListMusic size={14} />{#each st.playlists as p}<button class="chip" onclick={() => playlist(p)}>{p}</button>{/each}{/if}
        {/if}
      </div>
      <div class="grid">
        {#each shown as i (i.file)}
          <button class="w" class:cur={isCurrentWe(i)} class:busy={busy === i.id} onclick={() => apply('we', i.file, i.id)} title={i.title}>
            <img src={thumb(i.preview)} alt="" loading="lazy" draggable="false" />
            <span class="cap"><span>{i.title}</span><em>{i.kind}</em></span>
            {#if isCurrentWe(i)}<span class="badge"><Check size={12} strokeWidth={3} /></span>{/if}
          </button>
        {/each}
      </div>
    {/if}
  {:else if tab === 'pictures'}
    <div class="filters">
      <FolderOpen size={14} />
      <input class="folder" bind:value={folder} onchange={setFolder} spellcheck="false" />
      <span class="muted small">{pictures.length} pictures</span>
    </div>
    <div class="grid">
      {#each pictures as p (p)}
        <button class="w" class:cur={isCurrentPic(p)} class:busy={busy === p} onclick={() => apply('picture', p, p)} title={p}>
          <img src={thumb(p)} alt="" loading="lazy" draggable="false" />
          {#if isCurrentPic(p)}<span class="badge"><Check size={12} strokeWidth={3} /></span>{/if}
        </button>
      {:else}
        <p class="muted">No pictures in this folder.</p>
      {/each}
    </div>
  {:else if usConnected === false}
    <div class="connect">
      <p>Browse Unsplash with <strong>your own</strong> Access Key (free; new apps get 50 requests an hour). It's stored in Windows Credential Manager on this PC only.</p>
      <ol>
        <li>Open <button class="link" onclick={() => ipc.launch('https://unsplash.com/oauth/applications')}>unsplash.com/oauth/applications <ExternalLink size={11} /></button> and create a "New Application".</li>
        <li>Copy its <strong>Access Key</strong> (not the Secret Key) and paste it here.</li>
      </ol>
      <div class="row"><input bind:value={usKey} placeholder="Access Key" spellcheck="false" /><button class="primary" onclick={usConnect}>Connect</button></div>
    </div>
  {:else if usConnected}
    <div class="filters">
      <label class="search"><Search size={14} /><input bind:value={usQuery} placeholder="Search Unsplash (empty = random)" onkeydown={(e) => e.key === 'Enter' && usSearch()} /></label>
      <button class="chip on" onclick={usSearch}>Search</button>
      <button class="chip" onclick={usDisconnect}>Disconnect</button>
    </div>
    <div class="grid">
      {#each photos as p (p.id)}
        <button class="w" class:busy={busy === p.id} onclick={() => usApply(p)} title={p.alt_description ?? ''}>
          <img src={p.urls.small} alt="" loading="lazy" draggable="false" />
          <span class="cap"><span>Photo by {p.user.name} on Unsplash</span></span>
        </button>
      {/each}
    </div>
  {/if}

  <footer>
    <Shuffle size={14} />
    <span>Auto-change</span>
    <div class="seg">
      {#each ['off', 'daily', 'weekly'] as m}<button class:on={sched.mode === m} onclick={() => setSched({ mode: m, last: Math.floor(Date.now() / 1000) })}>{m}</button>{/each}
    </div>
    {#if sched.mode !== 'off'}
      <span class="muted small">from</span>
      <div class="seg">
        {#each [['we', 'Wallpaper Engine'], ['pictures', 'Pictures'], ['both', 'Both']] as [v, l]}<button class:on={sched.source === v} onclick={() => setSched({ source: v })}>{l}</button>{/each}
      </div>
      <button class="chip" onclick={() => invoke('wallpaper_change_now').then(() => setTimeout(load, 800)).catch((e) => (msg = String(e)))}>Change now</button>
    {/if}
  </footer>
  {#if msg}<p class="err">{msg}</p>{/if}
</div>

<style>
  .wg { width: 780px; padding: 14px; display: flex; flex-direction: column; gap: 10px; }
  header { display: flex; align-items: center; gap: 10px; }
  .screenrow { display: flex; align-items: center; gap: 8px; }
  .lbl { display: flex; align-items: center; gap: 6px; font-size: 12px; color: var(--fg-2); }
  .controls { display: flex; align-items: center; gap: 4px; min-height: 28px; flex-wrap: wrap; }
  .controls > button:not(.chip) { width: 28px; height: 28px; border-radius: 8px; display: grid; place-items: center; background: var(--surface); }
  .controls > button:not(.chip):hover { background: var(--surface-hover); }
  .sep { width: 1px; height: 18px; background: var(--border); margin: 0 6px; }
  .tabs, .mons, .seg { display: flex; align-items: center; gap: 2px; background: var(--surface); border-radius: 9px; padding: 2px; }

  .tabs button, .mons button, .seg button { padding: 5px 11px; border-radius: 7px; color: var(--fg-2); font-size: 12px; text-transform: capitalize; }
  .tabs button.on, .mons button.on, .seg button.on { background: var(--accent); color: white; }
  .filters { display: flex; align-items: center; gap: 6px; color: var(--fg-2); }
  .search { display: flex; align-items: center; gap: 6px; flex: 1; max-width: 300px; padding: 5px 9px; border-radius: 8px; background: var(--surface); border: 1px solid var(--border); }
  .search input, .folder, .row input { flex: 1; background: none; border: 0; outline: none; color: var(--fg); font-size: 12.5px; }
  .folder { padding: 5px 9px; border-radius: 8px; background: var(--surface); border: 1px solid var(--border); max-width: 420px; }
  .chip { padding: 4px 10px; border-radius: 12px; font-size: 11.5px; color: var(--fg-2); text-transform: capitalize; }
  .chip.on, .chip:hover { background: var(--surface-press); color: var(--fg); }
  .grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 8px; max-height: 440px; overflow-y: auto; padding-right: 2px; }
  .w { position: relative; aspect-ratio: 16 / 10; border-radius: 10px; overflow: hidden; background: var(--surface); border: 2px solid transparent;
       transition: transform var(--t-fast) var(--ease-out), border-color var(--t-fast); }
  .w:hover { transform: scale(1.02); }
  .w.cur { border-color: var(--accent); }
  .w.busy { opacity: 0.5; }
  .w img { width: 100%; height: 100%; object-fit: cover; display: block; }
  .cap { position: absolute; inset: auto 0 0 0; display: flex; justify-content: space-between; gap: 6px; padding: 14px 7px 5px; font-size: 11px;
         background: linear-gradient(transparent, rgba(0, 0, 0, 0.75)); color: white; text-align: left; }
  .cap span { overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .cap em { font-style: normal; opacity: 0.6; flex: none; }
  .badge { position: absolute; top: 6px; right: 6px; width: 20px; height: 20px; border-radius: 10px; background: var(--accent); color: white; display: grid; place-items: center; }
  .connect { font-size: 12.5px; line-height: 1.5; color: var(--fg-2); }
  .connect p { margin: 0 0 6px; }
  .connect ol { margin: 0 0 10px; padding-left: 18px; }
  .connect .row { display: flex; gap: 8px; }
  .connect .row input { padding: 7px 10px; border-radius: 8px; background: var(--surface); border: 1px solid var(--border); }
  .link { color: var(--accent); display: inline-flex; align-items: center; gap: 3px; }
  .primary { padding: 7px 14px; border-radius: 8px; background: var(--accent); color: white; }
  footer { display: flex; align-items: center; gap: 8px; padding-top: 10px; border-top: 1px solid var(--border); color: var(--fg-2); font-size: 12px; }
  .muted { color: var(--fg-2); margin: 0; }
  .small { font-size: 11.5px; }
  .err { color: var(--danger); margin: 0; font-size: 12px; }
</style>
