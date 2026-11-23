<script lang="ts">
  import { sysClock } from '../../lib/clock.svelte'
  // Next hours and the week ahead.
  // Fixed width and a placeholder of the same height while loading, so nothing jumps or overflows.
  import { MapPin, Sunrise, Sunset } from '@lucide/svelte'
  import type { CardProps } from '../types'
  import { DEFAULT_PLACE, describe, searchPlaces, watchWeather, type Place, type Weather } from './weather'
  let { item, act }: CardProps = $props()
  let place = $state<Place>(item.options.place ?? DEFAULT_PLACE)
  let w = $state<Weather | null>(null)
  let err = $state<string | null>(null)
  $effect(() => watchWeather(place, (v, e) => { w = v; err = e }))
  let picking = $state(false)
  let q = $state('')
  let results = $state<Place[]>([])
  const now = $derived(w ? describe(w.current.code, w.current.day) : null)
  const hm = (iso: string) => new Date(iso).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: sysClock.h12 })
  const hh = (iso: string) => new Date(iso).toLocaleTimeString([], { hour: '2-digit', hour12: sysClock.h12 })
  const dayName = (d: string, i: number) => (i === 0 ? 'Today' : new Date(d).toLocaleDateString([], { weekday: 'short' }))
  // Range bar for the week: position of each day's min/max within the week's overall range.
  const lo = $derived(w ? Math.min(...w.daily.map((d) => d.min)) : 0)
  const hi = $derived(w ? Math.max(...w.daily.map((d) => d.max)) : 1)
  const pct = (t: number) => ((t - lo) / Math.max(1, hi - lo)) * 100
  async function search() { results = q.trim() ? await searchPlaces(q) : [] }
  function choose(p: Place) { place = p; picking = false; q = ''; results = []; act('set-options', { place: p }) }
</script>

<div class="wc">
  <button class="place" onclick={() => (picking = !picking)} title="Change location"><MapPin size={13} /> <span>{place.name}</span></button>
  {#if picking}
    <div class="pick">
      <input placeholder="Search a city" bind:value={q} oninput={search} />
      {#each results as r}<button class="result" onclick={() => choose(r)}>{r.name}</button>{/each}
    </div>
  {:else if w && now}
    <div class="now">
      <now.icon size={46} strokeWidth={1.4} />
      <div class="nowtext">
        <div class="big">{Math.round(w.current.temp)}°</div>
        <div class="muted">{now.text}</div>
      </div>
      <div class="facts">
        <span>Feels {Math.round(w.current.feels)}°</span>
        <span>Humidity {w.current.humidity}%</span>
        <span>Wind {Math.round(w.current.wind)} km/h</span>
      </div>
    </div>
    <div class="hours">
      {#each w.hourly.slice(0, 6) as h}
        {@const d = describe(h.code)}
        <div class="h">
          <span class="muted">{hh(h.time)}</span>
          <d.icon size={18} strokeWidth={1.7} />
          <span class="t">{Math.round(h.temp)}°</span>
          <span class="pop">{h.pop >= 20 ? `${h.pop}%` : ''}</span>
        </div>
      {/each}
    </div>
    <div class="days">
      {#each w.daily as d, i}
        {@const x = describe(d.code)}
        <div class="d">
          <span class="dn">{dayName(d.date, i)}</span>
          <x.icon size={16} strokeWidth={1.7} />
          <span class="lo">{Math.round(d.min)}°</span>
          <span class="bar"><span style="left:{pct(d.min)}%; right:{100 - pct(d.max)}%"></span></span>
          <span class="hi">{Math.round(d.max)}°</span>
        </div>
      {/each}
    </div>
    <div class="sun muted"><Sunrise size={13} /> {hm(w.daily[0].sunrise)} <span class="gap"></span><Sunset size={13} /> {hm(w.daily[0].sunset)}</div>
  {:else}
    <div class="placeholder"><p class="muted">{err ? `Couldn't load the weather: ${err}` : 'Loading…'}</p></div>
  {/if}
</div>

<style>
  .wc { width: 340px; padding: 14px; display: flex; flex-direction: column; gap: 12px; overflow: hidden; }
  .place { display: flex; align-items: center; gap: 6px; color: var(--fg-2); font-size: 12px; padding: 3px 6px; border-radius: 7px; align-self: flex-start; max-width: 100%; }
  .place span { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .place:hover { background: var(--surface-hover); color: var(--fg); }
  .pick { display: flex; flex-direction: column; gap: 4px; min-height: 120px; }
  .pick input { padding: 7px 10px; border-radius: 9px; border: 1px solid var(--border); background: var(--surface); outline: none; }
  .result { text-align: left; padding: 6px 8px; border-radius: 8px; font-size: 12.5px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .result:hover { background: var(--surface-hover); }
  .now { display: grid; grid-template-columns: auto auto 1fr; align-items: center; gap: 12px; }
  .big { font: 600 36px/1 var(--font-display); }
  .facts { display: flex; flex-direction: column; align-items: flex-end; gap: 2px; font-size: 11.5px; color: var(--fg-2); }
  .hours { display: grid; grid-template-columns: repeat(6, 1fr); padding: 10px 0; border-block: 1px solid var(--border); }
  .h { display: flex; flex-direction: column; align-items: center; gap: 5px; font-size: 12px; }
  .h .t { font-weight: 600; }
  .pop { font-size: 10px; color: var(--accent); min-height: 12px; }
  .days { display: flex; flex-direction: column; gap: 4px; }
  .d { display: grid; grid-template-columns: 52px 22px 30px 1fr 30px; align-items: center; gap: 8px; font-size: 12.5px; }
  .lo { color: var(--fg-2); text-align: right; }
  .hi { text-align: left; }
  .bar { position: relative; height: 4px; border-radius: 2px; background: var(--surface-press); }
  .bar span { position: absolute; top: 0; bottom: 0; border-radius: 2px; background: linear-gradient(90deg, #64d2ff, #ffd60a); }
  .sun { display: flex; align-items: center; gap: 5px; font-size: 11.5px; }
  .gap { width: 10px; }
  .muted { color: var(--fg-2); }
  .placeholder { min-height: 330px; display: grid; place-items: center; }
  p { margin: 0; }
</style>
