<script lang="ts">
  // Recently Opened / Downloads / Screenshots: newest first with icons or thumbnails; click opens,
  // right-click shows in folder. Commands: type a word to launch an app or run an action; edit your
  // words. Screenshot Capture: screen / window / region, optional text recognition.
  import { Camera, Copy, FolderOpen, Plus, ScanText, Scissors, SquareDashed, AppWindow, Monitor, X } from '@lucide/svelte'
  import { UTILITIES } from './utilities'
  import { invoke } from '@tauri-apps/api/core'
  import { iconUrl, ipc } from '../../lib/ipc'
  import { ago } from '../../lib/services.svelte'
  import { dataFile } from '../../lib/stores.svelte'
  import type { CardProps } from '../types'
  let { item, close }: CardProps = $props()
  const id = item.widget

  // ---- folders
  type E = { path: string; name: string; dir: boolean; size: number; modified: number }
  let entries = $state<E[]>([])
  let err = $state('')
  const which = id === 'recently-opened' ? 'recent' : id
  if (id === 'recently-opened' || id === 'downloads' || id === 'screenshots')
    invoke<E[]>('folder_list', { which, limit: id === 'screenshots' ? 12 : 20 }).then((l) => (entries = l)).catch((e) => (err = String(e)))
  const isImg = (p: string) => /\.(png|jpe?g|gif|webp|bmp)$/i.test(p)
  const size = (b: number) => b >= 1e9 ? `${(b / 1e9).toFixed(1)} GB` : b >= 1e6 ? `${(b / 1e6).toFixed(1)} MB` : `${Math.max(1, Math.round(b / 1e3))} KB`
  async function openE(e: E) { try { await ipc.launch(e.path); close() } catch (x) { err = String(x) } }
  async function revealE(e: E, ev: MouseEvent) { ev.preventDefault(); await invoke('reveal', { target: e.path }); close() }
  async function openFolder() { ipc.launch(await invoke<string>('folder_path', { which })); close() }

  // ---- commands
  type Cmd = { word: string; target: string; args?: string }
  const cmds = dataFile<{ list: Cmd[] }>('commands', { list: [
    { word: 'calc', target: 'calc.exe' }, { word: 'notepad', target: 'notepad.exe' }, { word: 'downloads', target: 'shell:Downloads' },
    { word: 'settings', target: 'ms-settings:' }, { word: 'lock', target: 'action:lock' }, { word: 'desktop', target: 'action:show-desktop' },
  ] })
  let q = $state('')
  let input = $state<HTMLInputElement>()
  $effect(() => { if (id === 'commands') input?.focus() })
  const matches = $derived(cmds.value.list.filter((c) => !q || c.word.toLowerCase().startsWith(q.trim().toLowerCase().split(' ')[0])))
  let editing = $state(false)
  let nw = $state(''), nt = $state('')
  async function run(c: Cmd) {
    try {
      if (c.target.startsWith('action:')) {
        const a = c.target.slice(7)
        if (a === 'lock') await invoke('power', { action: 'lock' })
        else await invoke('sys_action', { action: a })
      } else {
        const extra = q.trim().split(' ').slice(1).join(' ')
        await ipc.launch(c.target, [c.args, extra].filter(Boolean).join(' ') || undefined)
      }
      close()
    } catch (e) { err = String(e) }
  }

  // ---- capture
  let text = $state<boolean>(item.options.ocr === true)
  let save = $state<boolean>(item.options.save === true)
  $effect(() => { invoke('capture_save_set', { on: save }) })
  let last = $state<{ path: string; text?: string | null } | null>(null)
  let busy = $state(false)
  async function cap(kind: 'screen' | 'window' | 'region') {
    err = ''; busy = true
    try {
      if (kind === 'region') {
        await invoke('capture_region_begin')
        close()
        await invoke('overlay_open', { kind: `capture:${text ? 'text' : 'image'}` })
        return
      }
      close()
      await new Promise((r) => setTimeout(r, 250)) // let the card fade away first
      last = await invoke(kind === 'screen' ? 'capture_screen' : 'capture_window', { text })
    } catch (e) { err = String(e) } finally { busy = false }
  }
</script>

<div class="fc">
  {#if id === 'commands'}
    <form class="bar" onsubmit={(e) => { e.preventDefault(); if (matches[0]) run(matches[0]) }}>
      <input bind:this={input} bind:value={q} placeholder="Type a command word" spellcheck="false" />
    </form>
    <ul class="list">
      {#each matches as c (c.word)}
        <li><button onclick={() => run(c)}>
          {#if c.target.startsWith('action:')}<span class="ph"></span>{:else}<img src={iconUrl(c.target, 32)} alt="" onerror={(e) => ((e.currentTarget as HTMLImageElement).style.visibility = 'hidden')} />{/if}
          <span class="nm"><strong>{c.word}</strong><small>{c.target}{c.args ? ' ' + c.args : ''}</small></span>
          {#if editing}<span class="x" role="button" tabindex="-1" onclick={(e) => { e.stopPropagation(); cmds.save({ list: cmds.value.list.filter((x) => x.word !== c.word) }) }} onkeydown={() => {}}><X size={12} /></span>{/if}
        </button></li>
      {/each}
    </ul>
    {#if editing}
      <form class="add" onsubmit={(e) => { e.preventDefault(); if (nw.trim() && nt.trim()) { cmds.save({ list: [...cmds.value.list.filter((x) => x.word !== nw.trim()), { word: nw.trim(), target: nt.trim() }] }); nw = ''; nt = '' } }}>
        <input bind:value={nw} placeholder="word" /><input bind:value={nt} placeholder="app, file, folder, URL or action:lock" /><button type="submit"><Plus size={13} /></button>
      </form>
    {/if}
    <button class="link" onclick={() => (editing = !editing)}>{editing ? 'Done' : 'Edit commands'}</button>
    <p class="muted">Words after the command are passed along (e.g. "notepad notes.txt").</p>
  {:else if id === 'quick-utilities'}
    <div class="utils">
      {#each UTILITIES as u}
        <button onclick={() => { ipc.launch(u.target, u.args).catch((e) => (err = String(e))); close() }} title={u.target}><u.icon size={18} /><span>{u.label}</span></button>
      {/each}
    </div>
  {:else if id === 'screenshot-capture'}
    <div class="caps">
      <button disabled={busy} onclick={() => cap('screen')}><Monitor size={20} /><span>Screen</span></button>
      <button disabled={busy} onclick={() => cap('window')}><AppWindow size={20} /><span>Window</span></button>
      <button disabled={busy} onclick={() => cap('region')}><SquareDashed size={20} /><span>Region</span></button>
    </div>
    <button class="snip" onclick={() => { ipc.launch('ms-screenclip:'); close() }}><Scissors size={15} /> Snipping Tool <small>Win+Shift+S</small></button>
    <label class="opt"><span><ScanText size={13} /> Copy text instead of the image</span><input type="checkbox" class="sw" checked={text} onchange={(e) => (text = e.currentTarget.checked)} /></label>
    <label class="opt"><span><FolderOpen size={13} /> Also save to Pictures\Screenshots</span><input type="checkbox" class="sw" checked={save} onchange={(e) => { save = e.currentTarget.checked; invoke('capture_save_set', { on: save }) }} /></label>
    <p class="muted">Captures go to the clipboard, ready to paste. Text recognition runs on this PC.</p>
    {#if last?.path}<p class="muted">Saved {last.path.split('\\').pop()}</p>{/if}
  {:else}
    <div class="hdr"><strong>{id === 'recently-opened' ? 'Recently opened' : id === 'downloads' ? 'Downloads' : 'Screenshots'}</strong>
      <button onclick={openFolder}><FolderOpen size={13} /> Open folder</button></div>
    {#if id === 'screenshots'}
      <div class="thumbs">
        {#each entries.filter((e) => isImg(e.path)) as e (e.path)}
          <button onclick={() => openE(e)} oncontextmenu={(ev) => revealE(e, ev)} title={`${e.name} · ${ago(e.modified)}`}><img src={`http://thumb.localhost/${encodeURIComponent(e.path)}?w=220`} alt="" /></button>
        {/each}
      </div>
    {:else}
      <ul class="list">
        {#each entries as e (e.path)}
          <li><button onclick={() => openE(e)} oncontextmenu={(ev) => revealE(e, ev)} title={e.path}>
            <img src={iconUrl(e.path, 32)} alt="" />
            <span class="nm"><strong>{e.name}</strong><small>{ago(e.modified)}{!e.dir && id === 'downloads' ? ` · ${size(e.size)}` : ''}</small></span>
            {#if id === 'downloads'}<span class="x" role="button" tabindex="-1" title="Copy path" onclick={(ev) => { ev.stopPropagation(); navigator.clipboard.writeText(e.path) }} onkeydown={() => {}}><Copy size={12} /></span>{/if}
          </button></li>
        {:else}<li class="muted">Nothing here yet.</li>{/each}
      </ul>
    {/if}
    <p class="muted">Right-click an item to show it in its folder.</p>
  {/if}
  {#if err}<p class="err">{err}</p>{/if}
</div>

<style>
  .fc { width: 360px; padding: 12px; display: flex; flex-direction: column; gap: 8px; max-height: 600px; overflow-y: auto; }
  .hdr { display: flex; justify-content: space-between; align-items: center; font: 600 14px var(--font-display); }
  .hdr button, .link { display: flex; align-items: center; gap: 5px; font-size: 12px; color: var(--fg-2); padding: 4px 9px; border-radius: 7px; background: var(--surface-press); align-self: flex-start; }
  .hdr button:hover, .link:hover { color: var(--fg); }
  .list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 1px; }
  .list button { width: 100%; display: flex; align-items: center; gap: 10px; padding: 6px 8px; border-radius: 8px; text-align: left; }
  .list button:hover { background: var(--surface-hover); }
  .list img, .ph { width: 20px; height: 20px; flex: none; }
  .nm { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .nm strong { font-size: 12.5px; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .nm small { font-size: 11px; color: var(--fg-3); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .x { color: var(--fg-3); padding: 3px; }
  .x:hover { color: var(--fg); }
  .thumbs { display: grid; grid-template-columns: repeat(3, 1fr); gap: 6px; }
  .thumbs button { aspect-ratio: 16 / 10; border-radius: 8px; overflow: hidden; background: var(--surface); }
  .thumbs img { width: 100%; height: 100%; object-fit: cover; transition: transform var(--t-fast) var(--ease-out); }
  .thumbs button:hover img { transform: scale(1.05); }
  .bar input, .add input { width: 100%; padding: 9px 11px; border-radius: 10px; border: 1px solid var(--border); background: var(--surface); outline: none; font-size: 14px; }
  .add { display: grid; grid-template-columns: 90px 1fr 30px; gap: 5px; }
  .add input { padding: 6px 8px; font-size: 12px; }
  .add button { border-radius: 8px; background: var(--accent); color: white; display: grid; place-items: center; }
  .caps { display: grid; grid-template-columns: repeat(3, 1fr); gap: 6px; }
  .caps button { display: flex; flex-direction: column; align-items: center; gap: 6px; padding: 14px 6px; border-radius: 12px; background: var(--surface); font-size: 12.5px; }
  .caps button:hover { background: var(--accent); color: white; }
  .snip { display: flex; align-items: center; gap: 8px; padding: 9px 11px; border-radius: 10px; background: var(--surface); font-size: 12.5px; }
  .snip:hover { background: var(--surface-hover); }
  .snip small { margin-left: auto; color: var(--fg-3); }
  .utils { display: grid; grid-template-columns: repeat(4, 1fr); gap: 6px; }
  .utils button { display: flex; flex-direction: column; align-items: center; gap: 6px; padding: 10px 4px; border-radius: 11px; background: var(--surface); font-size: 11px; text-align: center; }
  .utils button:hover { background: var(--accent); color: white; }
  .opt { display: flex; justify-content: space-between; align-items: center; font-size: 12.5px; }
  .opt span { display: flex; align-items: center; gap: 6px; }
  .muted { color: var(--fg-3); font-size: 11.5px; margin: 0; }
  .err { color: var(--danger); font-size: 12px; margin: 0; }
</style>
