<script lang="ts">
  // Windows of one app: click to bring forward, × to close. (Live DWM thumbnails: next step.)
  import { X } from '@lucide/svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { iconUrl } from '../../lib/ipc'
  import type { CardProps } from '../types'
  import { groupKey, running, startWindows } from './apps.svelte'
  let { payload, close }: CardProps = $props()
  startWindows()
  const wins = $derived(running.value.filter((w) => groupKey(w) === payload?.key))
  $effect(() => { if (wins.length === 0) close() })
</script>

<ul class="wl">
  {#each wins as w (w.hwnd)}
    <li class:fg={w.foreground} class:away={w.cloaked}>
      <button class="row" onclick={() => { invoke('window_activate', { hwnd: w.hwnd, toggle: false }); close() }}>
        <img src={iconUrl(w.aumid.includes('!') ? `shell:AppsFolder\\${w.aumid}` : w.exe, 32)} alt="" />
        <span class="t">{w.title}</span>
        {#if w.cloaked}<span class="tag">other workspace</span>{:else if w.minimized}<span class="tag">minimised</span>{/if}
      </button>
      <button class="x" aria-label="Close window" onclick={() => invoke('window_close', { hwnd: w.hwnd })}><X size={14} /></button>
    </li>
  {/each}
</ul>

<style>
  .wl { list-style: none; margin: 0; padding: 6px; width: 360px; }
  li { display: flex; align-items: center; border-radius: 8px; }
  li:hover { background: var(--surface-hover); }
  li.fg { background: var(--surface); }
  .row { flex: 1; min-width: 0; display: flex; align-items: center; gap: 9px; padding: 7px 8px; text-align: left; }
  .row img { width: 18px; height: 18px; }
  .t { flex: 1; min-width: 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; font-size: 12.5px; }
  .away .t { color: var(--fg-2); }
  .tag { font-size: 10.5px; color: var(--fg-3); flex: none; }
  .x { padding: 6px; margin-right: 4px; border-radius: 6px; color: var(--fg-3); }
  .x:hover { background: var(--danger); color: white; }
</style>
