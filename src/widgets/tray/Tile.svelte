<script lang="ts">
  // Tray: one arrow that opens a grid of notification-area icons. The arrow turns while open.
  import { ChevronUp } from '@lucide/svelte'
  import type { TileProps } from '../types'
  import { startTray, tray } from './tray.svelte'
  let { active, ctx }: TileProps = $props()
  startTray()
</script>

<span class="tr" class:open={active} class:top={ctx.layout.edge === 'top'} title={`${tray.value.length} tray icons`}>
  <ChevronUp size={18} strokeWidth={2} />
</span>

<style>
  .tr { display: grid; place-items: center; width: 100%; height: 100%; transition: transform var(--t-med) var(--ease-out); }
  .tr.top { transform: rotate(180deg); }
  .tr.open { transform: rotate(180deg); }
  .tr.top.open { transform: rotate(0deg); }
</style>
