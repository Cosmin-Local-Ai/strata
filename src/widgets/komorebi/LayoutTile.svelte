<script lang="ts">
  import { komorebi } from '../../lib/stores.svelte'
  import type { TileProps } from '../types'
  import LayoutGlyph from './LayoutGlyph.svelte'
  import { LAYOUTS, monitorFor } from './komorebi'
  let { ctx }: TileProps = $props()
  const mon = $derived(monitorFor(ctx.layout.hmonitor))
  const ws = $derived(mon?.workspaces[mon.focusedWorkspace])
  const shown = $derived(komorebi.value.paused ? 'Paused' : ws?.monocle ? 'Monocle' : (ws?.layout ?? 'BSP'))
  const label = $derived(shown === 'Paused' || shown === 'Monocle' ? shown : (LAYOUTS.find((l) => l.state === shown)?.label ?? shown))
</script>

<span class="lt" class:off={!komorebi.value.online}>
  <LayoutGlyph layout={shown} size={18} />
  <span class="name">{komorebi.value.online ? label : 'Offline'}</span>
</span>

<style>
  .lt { display: flex; align-items: center; gap: 8px; padding: 0 12px; height: 100%; font-size: 12.5px; font-weight: 600; }
  .lt.off { opacity: 0.45; }
  .name { white-space: nowrap; }
</style>
