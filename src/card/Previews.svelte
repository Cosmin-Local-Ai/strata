<script lang="ts">
  // Hover previews for an app's windows: title, close button and a live DWM thumbnail each. The page
  // only reserves the slots; Windows composes the live thumbnails into them (previews.rs).
  import { X } from '@lucide/svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { iconUrl } from '../lib/ipc'
  import { groupKey, running, startWindows, windowsForPinned } from '../widgets/apps/apps.svelte'

  let { payload, uid }: { payload: any; uid: string } = $props()
  startWindows()
  const wins = $derived(
    payload?.pinned ? windowsForPinned(payload.pinned, running.value) : running.value.filter((w) => groupKey(w) === payload?.key)
  )
  const slots: Record<number, HTMLElement> = {}

  function place() {
    const list = wins.filter((w) => !w.minimized && slots[w.hwnd]).map((w) => {
      const r = slots[w.hwnd].getBoundingClientRect()
      return { hwnd: w.hwnd, x: r.left, y: r.top, w: r.width, h: r.height }
    })
    invoke('previews_set', { slots: list })
  }
  $effect(() => { void wins.length; requestAnimationFrame(place) })
  $effect(() => { if (wins.length === 0) invoke('card_close_if', { uid }) })

  const leave = () => { invoke('preview_hover', { on: false }); invoke('card_close_if', { uid }) }
</script>

<div class="pv" role="list" onmouseenter={() => invoke('preview_hover', { on: true })} onmouseleave={leave}>
  {#each wins as w (w.hwnd)}
    <div class="p" role="listitem">
      <div class="head">
        <img src={iconUrl(w.aumid.includes('!') ? `shell:AppsFolder\\${w.aumid}` : w.exe, 32)} alt="" />
        <span class="t">{w.title}</span>
        <button class="x" aria-label="Close window" onclick={() => invoke('window_close', { hwnd: w.hwnd })}><X size={13} /></button>
      </div>
      <button class="slot" bind:this={slots[w.hwnd]} onclick={() => { invoke('window_activate', { hwnd: w.hwnd, toggle: false }); invoke('card_close_if', { uid }) }}>
        {#if w.minimized}<span class="min">Minimised</span>{/if}
      </button>
    </div>
  {/each}
</div>

<style>
  .pv { display: flex; gap: 10px; padding: 10px; }
  .p { width: 240px; display: flex; flex-direction: column; gap: 6px; }
  .head { display: flex; align-items: center; gap: 7px; padding: 0 2px; }
  .head img { width: 16px; height: 16px; }
  .t { flex: 1; min-width: 0; font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .x { padding: 3px; border-radius: 6px; color: var(--fg-3); }
  .x:hover { background: var(--danger); color: white; }
  .slot { width: 240px; height: 140px; border-radius: 8px; background: rgba(255, 255, 255, 0.04); display: grid; place-items: center; }
  .slot:hover { background: rgba(255, 255, 255, 0.08); }
  .min { font-size: 11.5px; color: var(--fg-3); }
</style>
