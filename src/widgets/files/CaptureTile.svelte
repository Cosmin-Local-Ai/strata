<script lang="ts">
  // Three one-click captures on the dock itself: window, region, screen (to the clipboard).
  import { AppWindow, Crop, Monitor } from '@lucide/svelte'
  import { invoke } from '@tauri-apps/api/core'
  import type { TileProps } from '../types'
  let { item }: TileProps = $props()
  async function go(kind: 'window' | 'region' | 'screen', e: MouseEvent) {
    e.stopPropagation()
    const text = item.options.ocr === true
    if (kind === 'region') { await invoke('capture_region_begin'); await invoke('overlay_open', { kind: `capture:${text ? 'text' : 'image'}` }); return }
    await invoke(kind === 'window' ? 'capture_window' : 'capture_screen', { text })
  }
</script>

<span class="caps">
  <button class="b1" title="Capture the window in front" onclick={(e) => go('window', e)}><AppWindow size={15} /></button>
  <button class="b2" title="Capture a region" onclick={(e) => go('region', e)}><Crop size={15} /></button>
  <button class="b3" title="Capture the screen" onclick={(e) => go('screen', e)}><Monitor size={15} /></button>
</span>

<style>
  .caps { display: flex; align-items: center; gap: 6px; padding: 0 8px; height: 100%; }
  button { width: calc(var(--tile) - 12px); height: calc(var(--tile) - 12px); border-radius: 50%; display: grid; place-items: center; transition: transform var(--t-fast) var(--ease-out); }
  button:hover { transform: scale(1.08); }
  button:active { transform: scale(0.94); }
  .b1 { background: rgba(10, 132, 255, 0.22); color: #4aa3ff; }
  .b2 { background: rgba(48, 209, 88, 0.22); color: #4cd471; }
  .b3 { background: color-mix(in srgb, var(--accent) 25%, transparent); color: var(--accent); }
</style>
