<script lang="ts">
  import type { TileProps } from '../types'
  import { DEFAULT_PLACE, describe, watchWeather, type Weather } from './weather'
  let { item }: TileProps = $props()
  let w = $state<Weather | null>(null)
  let err = $state<string | null>(null)
  $effect(() => watchWeather(item.options.place ?? DEFAULT_PLACE, (v, e) => { w = v; err = e }))
  const d = $derived(w ? describe(w.current.code, w.current.day) : null)
</script>

<span class="wt" title={err ?? ''}>
  {#if w && d}
    <d.icon size={18} strokeWidth={1.9} />
    <span class="t">{Math.round(w.current.temp)}°</span>
  {:else}
    <span class="t muted">{err ? '—' : '…'}</span>
  {/if}
</span>

<style>
  .wt { display: flex; align-items: center; gap: 6px; padding: 0 12px; height: 100%; }
  .t { font: 600 15px var(--font-display); font-variant-numeric: tabular-nums; }
  .muted { color: var(--fg-3); }
</style>
