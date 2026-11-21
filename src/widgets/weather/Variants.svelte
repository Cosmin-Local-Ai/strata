<script lang="ts">
  // Weather Details · Today & Forecast (three days) · Weather Status (in words) · Hourly (six hours) ·
  // Sunrise & Sunset. All share the Weather data and open the Weather card (next hours, week ahead).
  import { Droplets, Sunrise, Sunset, Wind } from '@lucide/svelte'
  import type { TileProps } from '../types'
  import { DEFAULT_PLACE, describe, watchWeather, type Weather } from './weather'
  import ColorIcon from './ColorIcon.svelte'
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
  // Weather Card: sky colour follows the conditions and day/night.
  const sky = $derived.by(() => {
    if (!w) return ''
    const c = w.current.code, day = w.current.day
    if (!day) return 'linear-gradient(135deg, #1c2a4a, #0d1528)'
    if (c >= 95) return 'linear-gradient(135deg, #4b5563, #1f2937)'
    if (c >= 51) return 'linear-gradient(135deg, #5b7290, #3a4d66)'
    if (c === 3 || c === 45 || c === 48) return 'linear-gradient(135deg, #8197b3, #5d7392)'
    return 'linear-gradient(135deg, #4a9cf0, #2f6fd6)'
  })
  const city = $derived(String((item.options.place ?? DEFAULT_PLACE).name).split(',')[0])
  const sunT = $derived.by(() => {
    if (!w) return 0
    const a = Date.parse(w.daily[0].sunrise), b = Date.parse(w.daily[0].sunset)
    return Math.max(0, Math.min(1, (Date.now() - a) / (b - a)))
  })
</script>

{#if !w || !d}
  <span class="row muted" title={err ?? ''}>{err ? '—' : '…'}</span>
{:else if id === 'weather-card'}
  <span class="wc" style="background:{sky}">
    <span class="col"><strong class="cond">{d.text}</strong><small class="city">{city}</small></span>
    <span class="big">{Math.round(w.current.temp)}°</span>
    <span class="gl"><ColorIcon code={w.current.code} day={w.current.day} size={34} /></span>
  </span>
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
  .wc { display: flex; align-items: center; gap: 10px; height: 100%; padding: 0 6px 0 12px; border-radius: var(--tile-r); color: white; white-space: nowrap;
        box-shadow: inset 0 1px 0 rgba(255,255,255,.18); text-shadow: none; }
  .wc .cond { font: 700 12.5px var(--font-display); color: white; }
  .wc .city { font-size: 11px; color: rgba(255,255,255,.82); }
  .wc .big { font: 200 calc(var(--tile) * 0.56) / 1 var(--font-display); letter-spacing: -0.02em; margin-left: 4px; }
  .wc .gl { display: grid; place-items: center; margin-left: -4px; filter: drop-shadow(0 2px 3px rgba(0,0,0,.18)); }
</style>
