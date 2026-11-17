<script lang="ts">
  // A pinned app, file, folder or shortcut.
  import { iconUrl } from '../../lib/ipc'
  import { running, startWindows, windowsForPinned } from '../apps/apps.svelte'
  import { previewHover } from '../apps/hover'
  import type { TileProps } from '../types'
  let { item }: TileProps = $props()
  startWindows()
  const wins = $derived(windowsForPinned(item.options.path ?? '', running.value))
  const hover = previewHover(() => 'preview:' + item.uid, () => ({ pinned: item.options.path }))
  let host: HTMLSpanElement | undefined = $state()
  let failed = $state(false)
  const src = $derived(iconUrl(item.options.icon || item.options.path, 96))
</script>

<span class="hoverhost" bind:this={host} role="presentation" onmouseenter={() => { if (wins.length && host) hover.enter(host.parentElement ?? host) }} onmouseleave={() => hover.leave()} onmousedown={() => hover.cancel()}></span>
{#if wins.length}<span class="dot" class:active={wins.some((w) => w.foreground)}></span>{/if}
{#if !failed}
  <img {src} alt="" draggable="false" onerror={() => (failed = true)} />
{:else}
  <span class="fallback">{(item.options.name || '?').slice(0, 1)}</span>
{/if}

<style>
  img { width: 72%; height: 72%; object-fit: contain; pointer-events: none; }
  .hoverhost { position: absolute; inset: 0; }
  .dot { position: absolute; bottom: 2px; left: 50%; width: 4px; height: 4px; margin-left: -2px; border-radius: 2px; background: var(--fg-2); }
  .dot.active { width: 12px; margin-left: -6px; background: var(--fg); }
  .fallback { font: 600 18px var(--font-display); }
</style>
