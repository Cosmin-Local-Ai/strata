<script lang="ts">
  // Everything (voidtools): instant file search from the dock. Results come straight from your running
  // Everything. Enter opens the top result, Ctrl+Enter shows it in Explorer, ↑/↓ choose,
  // right-click for more. Supports Everything's syntax (ext:pdf, folder:, *.png, …).
  import { File, Folder, Search } from '@lucide/svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { iconUrl, ipc } from '../../lib/ipc'
  import type { CardProps } from '../types'
  import { fitAt } from '../../lib/fit'
  let { close }: CardProps = $props()

  type Hit = { path: string; name: string; folder: boolean }
  const EVERYTHING = 'C:\\Program Files\\Everything\\Everything.exe'
  let q = $state('')
  let hits = $state<Hit[]>([])
  let sel = $state(0)
  let err = $state('')
  let sort = $state<number>(+(localStorage.getItem('ev:sort') ?? '20'))
  let input: HTMLInputElement
  $effect(() => { input?.focus() })
  invoke<boolean>('everything_running').then((r) => { if (!r) err = 'Everything isn\'t running. Start it and try again.' })

  let seq = 0
  $effect(() => {
    const term = q.trim(), s = sort
    localStorage.setItem('ev:sort', String(s))
    if (!term) { hits = []; return }
    const my = ++seq
    const t = setTimeout(async () => {
      try { const r = await invoke<Hit[]>('everything_search', { query: term, max: 40, sort: s }); if (my === seq) { hits = r; sel = 0; err = '' } }
      catch (e) { if (my === seq) err = String(e) }
    }, 90)
    return () => clearTimeout(t)
  })
  const dir = (h: Hit) => h.path.slice(0, h.path.length - h.name.length - 1)
  async function open(h: Hit) { try { await ipc.launch(h.path); close() } catch (e) { err = String(e) } }
  async function reveal(h: Hit) { await invoke('reveal', { target: h.path }); close() }
  function key(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') { e.preventDefault(); sel = Math.min(sel + 1, hits.length - 1); scroll() }
    else if (e.key === 'ArrowUp') { e.preventDefault(); sel = Math.max(sel - 1, 0); scroll() }
    else if (e.key === 'Enter' && hits[sel]) { e.ctrlKey ? reveal(hits[sel]) : open(hits[sel]) }
  }
  let list: HTMLElement
  const scroll = () => queueMicrotask(() => list?.children[sel]?.scrollIntoView({ block: 'nearest' }))
  let menu = $state<{ x: number; y: number; h: Hit } | null>(null)
</script>

<svelte:window onclick={() => (menu = null)} />

<div class="ev">
  <label class="bar">
    <Search size={16} />
    <input bind:this={input} bind:value={q} onkeydown={key} placeholder="Search files and folders with Everything" spellcheck="false" />
    <select bind:value={sort} title="Sort">
      <option value={20}>Most used</option><option value={14}>Recently modified</option><option value={1}>Name</option>
    </select>
  </label>
  {#if err}<p class="msg">{err}</p>
  {:else if q.trim() && !hits.length}<p class="msg">No results.</p>
  {/if}
  {#if hits.length}
    <ul bind:this={list}>
      {#each hits as h, i (h.path)}
        <li>
          <button class:sel={i === sel} onmouseenter={() => (sel = i)} onclick={() => open(h)}
                  oncontextmenu={(e) => { e.preventDefault(); e.stopPropagation(); menu = { x: e.clientX, y: e.clientY, h } }} title={h.path}>
            {#if h.folder}<Folder size={18} />{:else}<img src={iconUrl(h.path, 32)} alt="" onerror={(e) => ((e.currentTarget as HTMLImageElement).style.display = 'none')} />{/if}
            <span class="t"><span class="n">{h.name}</span><span class="p">{dir(h)}</span></span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
  <div class="foot">
    <span>Enter open · Ctrl+Enter show in folder</span>
    <button onclick={() => { ipc.launch(EVERYTHING, q.trim() ? `-search "${q.trim().replace(/"/g, '')}"` : undefined); close() }}>Open in Everything</button>
  </div>
  {#if menu}
    <ul class="menu" use:fitAt={{ x: menu.x, y: menu.y }}>
      <li><button onclick={() => open(menu!.h)}>Open</button></li>
      {#if !menu.h.folder}<li><button onclick={() => { invoke('launch_admin', { target: menu!.h.path }); close() }}>Run as administrator</button></li>{/if}
      <li><button onclick={() => reveal(menu!.h)}>Show in folder</button></li>
      <li><button onclick={() => { navigator.clipboard.writeText(menu!.h.path); menu = null }}>Copy path</button></li>
    </ul>
  {/if}
</div>

<style>
  .ev { position: relative; width: 560px; padding: 12px; display: flex; flex-direction: column; gap: 8px; }
  .bar { display: flex; align-items: center; gap: 10px; padding: 8px 12px; border-radius: 11px; background: var(--surface); border: 1px solid var(--border); color: var(--fg-2); }
  .bar input { flex: 1; background: none; border: 0; outline: none; font-size: 15px; color: var(--fg); min-width: 0; }
  .bar select { appearance: none; background: var(--surface-press); border: 0; border-radius: 7px; padding: 3px 8px; font-size: 11.5px; color: var(--fg-2); outline: none; }
  .bar select option { background: #222; color: #eee; }
  ul { list-style: none; margin: 0; padding: 0; max-height: 420px; overflow-y: auto; }
  li button { width: 100%; display: flex; align-items: center; gap: 10px; padding: 6px 8px; border-radius: 9px; text-align: left; color: var(--fg-2); }
  li button.sel { background: var(--surface-hover); color: var(--fg); }
  li img { width: 18px; height: 18px; flex: none; }
  .t { display: flex; flex-direction: column; min-width: 0; }
  .n { font-size: 13px; color: var(--fg); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .p { font-size: 11px; color: var(--fg-3); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .msg { color: var(--fg-2); font-size: 12.5px; margin: 4px 6px; }
  .foot { display: flex; justify-content: space-between; align-items: center; font-size: 11px; color: var(--fg-3); padding: 0 4px; }
  .foot button { font-size: 11.5px; color: var(--fg-2); padding: 3px 9px; border-radius: 7px; background: var(--surface-press); }
  .foot button:hover { color: var(--fg); }
  .menu { position: absolute; z-index: 10; min-width: 180px; padding: 5px; border-radius: 10px; max-height: none; background: rgba(32, 32, 38, 0.98); border: 1px solid var(--border); box-shadow: 0 10px 30px rgba(0, 0, 0, 0.45); }
  .menu button { padding: 6px 10px; font-size: 12.5px; border-radius: 6px; }
  .menu button:hover { background: var(--accent); color: white; }
</style>
