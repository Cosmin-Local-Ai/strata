<script lang="ts">
  // Recycle Bin (count, size, empty) and Language (layouts; click to switch).
  import { Check, Trash2 } from '@lucide/svelte'
  import { invoke } from '@tauri-apps/api/core'
  import type { CardProps } from '../types'
  import { act } from './actions.svelte'
  let { item, close }: CardProps = $props()
  const id = item.widget
  $effect(() => act.watch(id))
  let armed = $state(false)
  const size = (b: number) => (b >= 1024 ** 3 ? (b / 1024 ** 3).toFixed(1) + ' GB' : (b / 1024 ** 2).toFixed(0) + ' MB')
  async function empty() {
    if (!armed) { armed = true; setTimeout(() => (armed = false), 3000); return }
    await invoke('recycle_bin_empty')
    act.bin = { items: 0, bytes: 0 }
    close()
  }
  const name = (loc: string) => { try { return new Intl.DisplayNames([navigator.language], { type: 'language' }).of(loc) ?? loc } catch { return loc } }
</script>

<div class="ac">
  {#if id === 'recycle-bin'}
    <div class="big"><Trash2 size={30} strokeWidth={1.5} /><div><strong>{act.bin.items} item{act.bin.items === 1 ? '' : 's'}</strong><span>{size(act.bin.bytes)}</span></div></div>
    <button class="empty" class:armed disabled={act.bin.items === 0} onclick={empty}>{armed ? 'Click again to empty' : 'Empty Recycle Bin'}</button>
    <button class="link" onclick={() => { invoke('launch', { target: 'shell:RecycleBinFolder' }); close() }}>Open Recycle Bin</button>
  {:else if id === 'language'}
    <ul>
      {#each act.layouts as l (l.id)}
        <li><button class:cur={l.current} onclick={() => { invoke('kb_set_layout', { id: l.id }); close() }}><span class="tick">{#if l.current}<Check size={14} />{/if}</span>{name(l.locale)} <small>{l.locale}</small></button></li>
      {/each}
    </ul>
    <button class="link" onclick={() => { invoke('launch', { target: 'ms-settings:regionlanguage' }); close() }}>Language settings</button>
  {/if}
</div>

<style>
  .ac { width: 260px; padding: 14px; display: flex; flex-direction: column; gap: 10px; }
  .big { display: flex; align-items: center; gap: 12px; }
  .big strong { display: block; font: 600 15px var(--font-display); }
  .big span { font-size: 12px; color: var(--fg-2); }
  .empty { padding: 8px; border-radius: 9px; background: var(--surface-press); font-weight: 600; }
  .empty:hover:not(:disabled) { background: var(--danger); color: white; }
  .empty.armed { background: var(--danger); color: white; }
  .empty:disabled { opacity: 0.4; }
  ul { list-style: none; margin: 0; padding: 0; }
  li button { width: 100%; display: flex; align-items: center; gap: 8px; padding: 7px 8px; border-radius: 8px; text-align: left; font-size: 12.5px; }
  li button:hover { background: var(--surface-hover); }
  li button.cur { background: var(--surface); }
  .tick { width: 14px; color: var(--accent); }
  small { color: var(--fg-3); margin-left: auto; }
  .link { color: var(--fg-3); font-size: 12px; align-self: flex-start; }
  .link:hover { color: var(--fg); }
</style>
