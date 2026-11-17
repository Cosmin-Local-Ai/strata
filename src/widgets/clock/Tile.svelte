<script lang="ts">
  // Clock (seconds optional, date, 12/24 h). Ticks once a minute unless seconds are shown.
  import { clock } from '../../lib/stores.svelte'
  import type { TileProps } from '../types'
  let { item }: TileProps = $props()
  const seconds = $derived(!!item.options.seconds)
  const c = clock(item.options.seconds ? 1000 : 60_000)
  const h12 = $derived(item.options.hour12 === true)
  const time = $derived(c.now.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: seconds ? '2-digit' : undefined, hour12: h12 }))
  const date = $derived(c.now.toLocaleDateString([], { weekday: 'short', day: 'numeric', month: 'short' }))
</script>

<span class="clock">
  <span class="time">{time}</span>
  <span class="date">{date}</span>
</span>

<style>
  .clock { display: flex; flex-direction: column; align-items: flex-end; justify-content: center; padding: 0 14px; height: 100%; line-height: 1.15; }
  .time { font: 600 15px var(--font-display); font-variant-numeric: tabular-nums; }
  .date { font-size: 11px; color: var(--fg-2); }
</style>
