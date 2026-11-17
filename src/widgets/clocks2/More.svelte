<script lang="ts">
  // World Time (digital times for your cities) · Today (weekday, date, day progress) · Segment Clock
  // (seven-segment digits) · Clock & Weather (time with the current temperature).
  import { clock } from '../../lib/stores.svelte'
  import type { TileProps } from '../types'
  import { DEFAULT_PLACE, describe, watchWeather, type Weather } from '../weather/weather'
  let { item }: TileProps = $props()
  const id = $derived(item.widget)
  const c = clock(id === 'segment-clock' && item.options.seconds ? 1000 : 60_000)
  const n = $derived(c.now)
  const h12 = $derived(item.options.hour12 === true)
  const hh = $derived(String(h12 ? ((n.getHours() + 11) % 12) + 1 : n.getHours()).padStart(2, '0'))
  const mm = $derived(String(n.getMinutes()).padStart(2, '0'))
  const zones: { label: string; tz: string }[] = $derived(item.options.zones ?? [
    { label: 'NYC', tz: 'America/New_York' }, { label: 'LON', tz: 'Europe/London' }, { label: 'TYO', tz: 'Asia/Tokyo' },
  ])
  const zt = (tz: string) => n.toLocaleTimeString('en-GB', { timeZone: tz, hour: '2-digit', minute: '2-digit', hour12: h12 })
  const dayFrac = $derived((n.getHours() * 60 + n.getMinutes()) / 1440)

  // Seven-segment: segments a..g per digit.
  const SEG: Record<string, string> = { '0': 'abcdef', '1': 'bc', '2': 'abged', '3': 'abgcd', '4': 'fgbc', '5': 'afgcd', '6': 'afgedc', '7': 'abc', '8': 'abcdefg', '9': 'abcdfg' }
  const digits = $derived(`${hh}${mm}${item.options.seconds ? String(n.getSeconds()).padStart(2, '0') : ''}`.split(''))

  let w = $state<Weather | null>(null)
  $effect(() => { if (id === 'clock-weather') return watchWeather(item.options.place ?? DEFAULT_PLACE, (v) => (w = v)) })
  const wd = $derived(w ? describe(w.current.code, w.current.day) : null)
</script>

{#if id === 'world-time'}
  <span class="row">{#each zones as z}<span class="col"><small>{z.label}</small><strong>{zt(z.tz)}</strong></span>{/each}</span>
{:else if id === 'today'}
  <span class="row">
    <span class="col"><small>{n.toLocaleDateString(undefined, { weekday: 'long' })}</small><strong>{n.toLocaleDateString(undefined, { day: 'numeric', month: 'long' })}</strong></span>
    <span class="prog" title="{Math.round(dayFrac * 100)}% of today"><i style="width:{dayFrac * 100}%"></i></span>
  </span>
{:else if id === 'segment-clock'}
  <span class="seg">
    {#each digits as dg, i}
      {#if i === 2 || i === 4}<span class="colon"><b></b><b></b></span>{/if}
      <svg viewBox="0 0 12 20" width="11" height="19">
        {#each [['a', 'M2 1h8'], ['b', 'M11 2v7'], ['c', 'M11 11v7'], ['d', 'M2 19h8'], ['e', 'M1 11v7'], ['f', 'M1 2v7'], ['g', 'M2 10h8']] as [s, p]}
          <path d={p} stroke={SEG[dg].includes(s) ? 'var(--accent)' : 'var(--surface-press)'} stroke-width="1.8" stroke-linecap="round" />
        {/each}
      </svg>
    {/each}
  </span>
{:else if id === 'clock-weather'}
  <span class="row"><strong class="big">{hh}:{mm}</strong>{#if w && wd}<span class="wx"><wd.icon size={15} strokeWidth={1.9} />{Math.round(w.current.temp)}°</span>{/if}</span>
{/if}

<style>
  .row { display: flex; align-items: center; gap: 10px; padding: 0 12px; height: 100%; white-space: nowrap; }
  .col { display: flex; flex-direction: column; line-height: 1.15; }
  small { font-size: 10px; color: var(--fg-3); font-weight: 600; letter-spacing: .03em; }
  strong { font: 600 13px var(--font-display); font-variant-numeric: tabular-nums; }
  .big { font-size: 15px; }
  .prog { width: 46px; height: 5px; border-radius: 3px; background: var(--surface-press); overflow: hidden; }
  .prog i { display: block; height: 100%; background: var(--accent); }
  .seg { display: flex; align-items: center; gap: 2px; padding: 0 12px; height: 100%; }
  .colon { display: flex; flex-direction: column; gap: 5px; margin: 0 2px; }
  .colon b { width: 2.5px; height: 2.5px; border-radius: 2px; background: var(--accent); }
  .wx { display: flex; align-items: center; gap: 4px; font: 600 13px var(--font-display); color: var(--fg-2); }
</style>
