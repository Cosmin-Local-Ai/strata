<script lang="ts">
  // Notifications (with Strata notifications on): Windows' notification list plus Strata's own, newest
  // first, in Strata's style. Click one to open its app; Windows' list is one click away.
  import { BellOff, ExternalLink, X } from '@lucide/svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import { iconUrl, ipc } from '../../lib/ipc'
  import type { Toast } from '../../lib/notify'
  import type { CardProps } from '../types'
  let { close }: CardProps = $props()
  type Item = Toast & { at: number }
  let list = $state<Item[] | null>(null)
  const load = () => invoke<Item[]>('notices_list').then((l) => (list = l))
  load()
  $effect(() => { const un = listen('toast:push', () => setTimeout(load, 400)); return () => { un.then((f) => f()) } })
  const ago = (ms: number) => {
    const m = Math.round((Date.now() - ms) / 60000)
    return m < 1 ? 'now' : m < 60 ? `${m} min` : m < 1440 ? `${Math.round(m / 60)} h` : new Date(ms).toLocaleDateString([], { day: 'numeric', month: 'short' })
  }
  async function clearAll() { list = []; await invoke('notices_clear', { all: true }); load() }
  async function clearOne(t: Item) { list = (list ?? []).filter((x) => x.id !== t.id); await invoke('notices_clear', { ids: [t.id] }) }
  function open(t: Item) { if (t.target) ipc.launch(t.target).catch(() => {}); close() }
</script>

<div class="nc">
  <div class="hd"><strong>Notifications</strong><span class="sp"></span>
    {#if list?.length}<button class="link" onclick={clearAll}>Clear all</button>{/if}
    <button class="link" title="Windows' notification list" onclick={() => { invoke('sys_action', { action: 'notifications' }); close() }}><ExternalLink size={12} /> Windows</button>
  </div>
  {#if list && !list.length}
    <div class="empty"><BellOff size={22} /><span>No notifications</span></div>
  {:else}
    <ul>
      {#each list ?? [] as t (t.id)}
        <li><button onclick={() => open(t)}>
          <span class="ic">{#if t.target}<img src={iconUrl(t.target, 32)} alt="" onerror={(e) => ((e.currentTarget as HTMLImageElement).style.visibility = 'hidden')} />{/if}</span>
          <span class="tx"><span class="top"><small>{t.app}</small><small>{ago(t.at)}</small></span><strong>{t.title}</strong>{#if t.body}<span class="b">{t.body}</span>{/if}</span>
        </button><button class="rm" aria-label="Clear" title="Clear" onclick={() => clearOne(t)}><X size={13} /></button></li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .nc { width: 400px; padding: 14px; display: flex; flex-direction: column; gap: 8px; }
  .hd { display: flex; align-items: center; gap: 8px; }
  .hd strong { font: 600 15px var(--font-display); }
  .sp { flex: 1; }
  .link { display: inline-flex; align-items: center; gap: 5px; font-size: 12px; color: var(--fg-2); padding: 4px 9px; border-radius: 7px; background: var(--surface); }
  .link:hover { background: var(--surface-hover); color: var(--fg); }
  ul { list-style: none; margin: 0; padding: 0; max-height: 460px; overflow-y: auto; display: flex; flex-direction: column; gap: 4px; }
  li { position: relative; }
  .rm { position: absolute; top: 7px; right: 7px; width: 22px; height: 22px; border-radius: 11px; display: grid; place-items: center; color: var(--fg-3); background: var(--surface-press); opacity: 0; transition: opacity var(--t-fast); }
  li:hover .rm { opacity: 1; }
  .rm:hover { color: var(--fg); background: var(--surface-hover); }
  li > button:first-child { width: 100%; display: flex; gap: 10px; align-items: flex-start; text-align: left; padding: 9px 10px; border-radius: 12px; background: var(--surface); }
  li > button:first-child:hover { background: var(--surface-hover); }
  .top small:last-child { margin-right: 22px; }
  .ic { width: 22px; height: 22px; flex: none; display: grid; place-items: center; margin-top: 1px; }
  .ic img { width: 20px; height: 20px; object-fit: contain; }
  .tx { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  .top { display: flex; justify-content: space-between; gap: 8px; }
  .top small { font-size: 11px; color: var(--fg-3); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .tx strong { font-size: 13px; font-weight: 600; overflow-wrap: anywhere; }
  .b { font-size: 12px; color: var(--fg-2); white-space: pre-line; overflow: hidden; display: -webkit-box; -webkit-line-clamp: 3; line-clamp: 3; -webkit-box-orient: vertical; }
  .empty { display: flex; flex-direction: column; align-items: center; gap: 8px; padding: 26px 0; color: var(--fg-3); font-size: 12.5px; }
</style>
