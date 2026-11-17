<script lang="ts">
  // Clock family: Flip Clock, Analog Minimal, World Clocks (three analog faces),
  // Date, Day Number, 7 Days, Today. Ticks once a minute (seconds hand only on Analog when enabled).
  import { clock } from '../../lib/stores.svelte'
  import type { TileProps } from '../types'
  import WorldMap, { DEFAULT_CITIES } from './WorldMap.svelte'
  let { item }: TileProps = $props()
  const id = $derived(item.widget)
  const c = clock(item.options.seconds ? 1000 : 60_000)
  const n = $derived(c.now)
  const h12 = $derived(item.options.hour12 === true)
  const hh = $derived(String(h12 ? ((n.getHours() + 11) % 12) + 1 : n.getHours()).padStart(2, '0'))
  const mm = $derived(String(n.getMinutes()).padStart(2, '0'))
  const zones: { label: string; tz: string }[] = $derived(item.options.zones ?? [
    { label: 'NYC', tz: 'America/New_York' }, { label: 'LON', tz: 'Europe/London' }, { label: 'TYO', tz: 'Asia/Tokyo' },
  ])
  function inZone(tz: string) {
    const p = new Intl.DateTimeFormat('en-GB', { timeZone: tz, hour: 'numeric', minute: 'numeric', hour12: false }).formatToParts(n)
    return { h: +(p.find((x) => x.type === 'hour')?.value ?? 0), m: +(p.find((x) => x.type === 'minute')?.value ?? 0) }
  }
  const week = $derived(Array.from({ length: 7 }, (_, i) => { const d = new Date(n); d.setDate(n.getDate() - ((n.getDay() + 6) % 7) + i); return d }))
</script>

{#snippet face(h: number, m: number, s: number | null, size: number)}
  <svg viewBox="0 0 40 40" width={size} height={size} class="face">
    <circle cx="20" cy="20" r="18.5" class="rim" />
    {#each Array(12) as _, i}<line x1="20" y1="3.5" x2="20" y2={i % 3 ? 5.5 : 7} transform="rotate({i * 30} 20 20)" class="tick" />{/each}
    <line x1="20" y1="20" x2="20" y2="10.5" class="hand h" transform="rotate({(h % 12) * 30 + m / 2} 20 20)" />
    <line x1="20" y1="20" x2="20" y2="6.5" class="hand m" transform="rotate({m * 6} 20 20)" />
    {#if s !== null}<line x1="20" y1="22" x2="20" y2="5.5" class="hand s" transform="rotate({s * 6} 20 20)" />{/if}
    <circle cx="20" cy="20" r="1.4" class="pin" />
  </svg>
{/snippet}

{#if id === 'flip-clock'}
  <span class="flip">
    {#each (hh + mm).split('') as d, i}
      {#key d}<span class="card">{d}</span>{/key}{#if i === 1}<span class="colon">:</span>{/if}
    {/each}
  </span>
{:else if id === 'analog'}
  <span class="an">{@render face(n.getHours(), n.getMinutes(), item.options.seconds ? n.getSeconds() : null, 34)}</span>
{:else if id === 'world-clocks'}
  <span class="world">
    {#each zones as z}{@const t = inZone(z.tz)}<span class="wz">{@render face(t.h, t.m, null, 26)}<small>{z.label}</small></span>{/each}
  </span>
{:else if id === 'world-map'}
  <span class="wm"><WorldMap width={Math.round(44 * 2.6)} labels={false} cities={item.options.cities ?? DEFAULT_CITIES} /></span>
{:else if id === 'date'}
  <span class="date"><small>{n.toLocaleDateString([], { weekday: 'short' })}</small><strong>{n.toLocaleDateString([], { day: 'numeric', month: 'short' })}</strong></span>
{:else if id === 'day-number'}
  <span class="dn"><small>{n.toLocaleDateString([], { month: 'short' }).toUpperCase()}</small><strong>{n.getDate()}</strong></span>
{:else if id === 'week'}
  <span class="week">
    {#each week as d}<span class="wd" class:today={d.toDateString() === n.toDateString()}><small>{d.toLocaleDateString([], { weekday: 'narrow' })}</small>{d.getDate()}</span>{/each}
  </span>
{/if}

<style>
  .flip { display: flex; align-items: center; gap: 2px; padding: 0 10px; height: 100%; }
  .card { display: inline-grid; place-items: center; width: 17px; height: 26px; border-radius: 5px; background: var(--surface-press);
          font: 700 15px var(--font-display); font-variant-numeric: tabular-nums; animation: flip 320ms var(--ease-out); transform-origin: center; }
  @keyframes flip { from { transform: rotateX(90deg); opacity: 0.3; } }
  .colon { font-weight: 700; padding: 0 1px; opacity: 0.6; }
  .an { display: grid; place-items: center; width: 100%; height: 100%; }
  .face .rim { fill: var(--surface); stroke: var(--border); stroke-width: 1; }
  .face .tick { stroke: var(--fg-3); stroke-width: 1; }
  .face .hand { stroke: var(--fg); stroke-linecap: round; }
  .face .h { stroke-width: 2.4; } .face .m { stroke-width: 1.6; } .face .s { stroke: var(--accent); stroke-width: 0.9; }
  .face .pin { fill: var(--accent); }
  .world { display: flex; gap: 8px; padding: 0 10px; height: 100%; align-items: center; }
  .wz { display: flex; flex-direction: column; align-items: center; gap: 1px; }
  .wz small { font-size: 8.5px; color: var(--fg-2); font-weight: 600; }
  .date, .dn { display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 0 10px; height: 100%; line-height: 1.1; }
  .date small, .dn small { font-size: 10px; color: var(--accent); font-weight: 700; text-transform: uppercase; }
  .date strong { font-size: 13px; }
  .dn strong { font: 600 18px var(--font-display); }
  .week { display: flex; gap: 3px; padding: 0 8px; height: 100%; align-items: center; }
  .wd { display: flex; flex-direction: column; align-items: center; width: 20px; padding: 2px 0; border-radius: 6px; font-size: 11.5px; font-weight: 600; font-variant-numeric: tabular-nums; }
  .wd small { font-size: 9px; color: var(--fg-3); font-weight: 600; }
  .wd.today { background: var(--accent); color: white; }
  .wd.today small { color: rgba(255, 255, 255, 0.8); }
  .wm { display: grid; place-items: center; padding: 0 8px; height: 100%; }
</style>
