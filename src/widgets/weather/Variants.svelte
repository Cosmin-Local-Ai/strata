<script lang="ts">
  // Weather Details · Today & Forecast (three days) · Weather Status (in words) · Hourly (six hours) ·
  // Sunrise & Sunset. All share the Weather data and open the Weather card (next hours, week ahead).
  import { Droplets, Sunrise, Sunset, Wind } from '@lucide/svelte'
  import type { TileProps } from '../types'
  import { DEFAULT_PLACE, describe, watchWeather, type Weather } from './weather'
  let { item }: TileProps = $props()
  let w = $state<Weather | null>(null)
  let err = $state<string | null>(null)
  $effect(() => watchWeather(item.options.place ?? DEFAULT_PLACE, (v, e) => { w = v; err = e }))
  const id = $derived(item.widget)
  const d = $derived(w ? describe(w.current.code, w.current.day) : null)
  const hm = (iso: string) => new Date(iso).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
  const hh = (iso: string) => new Date(iso).toLocaleTimeString([], { hour: '2-digit' })
  const dayName = (iso: string) => new Date(iso + 'T12:00').toLocaleDateString(undefined, { weekday: 'short' })
  // Sun position along today's arc (0..1), for Sunrise & Sunset.
  const sunT = $derived.by(() => {
    if (!w) return 0
    const a = Date.parse(w.daily[0].sunrise), b = Date.parse(w.daily[0].sunset)
    return Math.max(0, Math.min(1, (Date.now() - a) / (b - a)))
  })
</script>

{#if !w || !d}
  <span class="row muted" title={err ?? ''}>{err ? '—' : '…'}</span>
{:else if id === 'weather-details'}
  <span class="row">
    <d.icon size={18} strokeWidth={1.9} />
    <span class="col"><strong>{Math.round(w.current.temp)}°</strong><small>feels {Math.round(w.current.feels)}°</small></span>
    <span class="col"><small><Droplets size={10} /> {w.current.humidity}%</small><small><Wind size={10} /> {Math.round(w.current.wind)} km/h</small></span>
  </span>
{:else if id === 'today-forecast'}
  <span class="row">
    {#each w.daily.slice(0, 4) as day, i}
      {@const dd = describe(day.code)}
      <span class="col c"><small>{i === 0 ? 'Today' : dayName(day.date)}</small><dd.icon size={14} /><small class="t">{Math.round(day.max)}° <i>{Math.round(day.min)}°</i></small></span>
    {/each}
  </span>
{:else if id === 'weather-status'}
  <span class="row"><d.icon size={18} strokeWidth={1.9} /><span class="col"><strong>{d.text}</strong><small>{Math.round(w.current.temp)}° · high {Math.round(w.daily[0].max)}° · low {Math.round(w.daily[0].min)}°</small></span></span>
{:else if id === 'hourly'}
  <span class="row">
    {#each w.hourly.slice(0, 6) as h}
      {@const dh = describe(h.code, new Date(h.time).getHours() > 6 && new Date(h.time).getHours() < 20)}
      <span class="col c"><small>{hh(h.time)}</small><dh.icon size={14} /><small class="t">{Math.round(h.temp)}°</small></span>
    {/each}
  </span>
{:else if id === 'sunrise-sunset'}
  <span class="row">
    <svg viewBox="0 0 40 20" width="40" height="20"><path d="M2 19 A18 18 0 0 1 38 19" fill="none" stroke="var(--surface-press)" stroke-width="1.6" stroke-dasharray="2 2" />
      <circle cx={20 - 18 * Math.cos(Math.PI * sunT)} cy={19 - 18 * Math.sin(Math.PI * sunT)} r="3" fill="#ffcc00" /></svg>
    <span class="col"><small><Sunrise size={10} /> {hm(w.daily[0].sunrise)}</small><small><Sunset size={10} /> {hm(w.daily[0].sunset)}</small></span>
  </span>
{/if}

<style>
  .row { display: flex; align-items: center; gap: 9px; padding: 0 12px; height: 100%; white-space: nowrap; }
  .col { display: flex; flex-direction: column; line-height: 1.2; }
  .col.c { align-items: center; gap: 1px; }
  strong { font: 600 13px var(--font-display); font-variant-numeric: tabular-nums; }
  small { font-size: 10.5px; color: var(--fg-2); display: inline-flex; align-items: center; gap: 3px; font-variant-numeric: tabular-nums; }
  small.t { color: var(--fg); }
  small i { font-style: normal; color: var(--fg-3); }
  .muted { color: var(--fg-3); }
</style>
