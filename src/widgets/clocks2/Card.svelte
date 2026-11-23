<script lang="ts">
  import { sysClock } from '../../lib/clock.svelte'
  // Clock card (all clock styles): the time with seconds, today's date, week and day of the year,
  // this week, display options for this tile, and your world clocks (add/remove/rename zones).
  import { Plus, X } from '@lucide/svelte'
  import { clock } from '../../lib/stores.svelte'
  import Combo from '../../lib/ui/Combo.svelte'
  import type { CardProps } from '../types'
  let { item, act }: CardProps = $props()
  const c = clock(1000)
  const n = $derived(c.now)

  let hour12 = $state<boolean>(item.options.hour12 ?? sysClock.h12)
  let seconds = $state<boolean>(item.options.seconds === true)
  type Zone = { label: string; tz: string }
  let zones = $state<Zone[]>(item.options.zones ?? [
    { label: 'NYC', tz: 'America/New_York' }, { label: 'LON', tz: 'Europe/London' }, { label: 'TYO', tz: 'Asia/Tokyo' },
  ])
  const save = () => act('set-options', { hour12, seconds, zones: $state.snapshot(zones) })

  const time = $derived(n.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit', second: '2-digit', hour12 }))
  const date = $derived(n.toLocaleDateString(undefined, { weekday: 'long', day: 'numeric', month: 'long', year: 'numeric' }))
  // ISO week number and day of the year.
  const week = $derived.by(() => {
    const d = new Date(Date.UTC(n.getFullYear(), n.getMonth(), n.getDate()))
    const day = d.getUTCDay() || 7
    d.setUTCDate(d.getUTCDate() + 4 - day)
    const y0 = new Date(Date.UTC(d.getUTCFullYear(), 0, 1))
    return Math.ceil(((+d - +y0) / 864e5 + 1) / 7)
  })
  const doy = $derived(Math.floor((+new Date(n.getFullYear(), n.getMonth(), n.getDate()) - +new Date(n.getFullYear(), 0, 0)) / 864e5))
  const ylen = $derived(new Date(n.getFullYear(), 1, 29).getDate() === 29 ? 366 : 365)
  const days = $derived.by(() => {
    const mon = new Date(n); mon.setDate(n.getDate() - ((n.getDay() + 6) % 7))
    return Array.from({ length: 7 }, (_, i) => { const d = new Date(mon); d.setDate(mon.getDate() + i); return d })
  })

  function zoneTime(tz: string) {
    const t = n.toLocaleTimeString(undefined, { timeZone: tz, hour: '2-digit', minute: '2-digit', hour12 })
    // Offset from local time in hours.
    const local = new Date(n.toLocaleString('en-US'))
    const there = new Date(n.toLocaleString('en-US', { timeZone: tz }))
    const diff = Math.round((+there - +local) / 36e5 * 2) / 2
    const day = there.getDate() !== local.getDate() ? (there > local ? ' · tomorrow' : ' · yesterday') : ''
    return { t, diff: (diff === 0 ? 'same time' : `${diff > 0 ? '+' : ''}${diff} h`) + day }
  }
  const allZones = (Intl as any).supportedValuesOf?.('timeZone') as string[] ?? []
  const zoneOpts = allZones.map((z) => ({ value: z, label: z.split('/').pop()!.replace(/_/g, ' '), hint: z }))
  let adding = $state(false)
  function addZone(tz: string) {
    const city = tz.split('/').pop()!.replace(/_/g, ' ')
    zones = [...zones, { tz, label: city.slice(0, 3).toUpperCase() }]
    adding = false
    save()
  }
</script>

<div class="cc">
  <div class="now">
    <div class="time">{time}</div>
    <div class="date">{date}</div>
    <div class="meta">Week {week} · Day {doy} of {ylen}</div>
  </div>

  <div class="week">
    {#each days as d}
      {@const today = d.toDateString() === n.toDateString()}
      <div class="day" class:today><small>{d.toLocaleDateString(undefined, { weekday: 'short' })}</small><span>{d.getDate()}</span></div>
    {/each}
  </div>

  <div class="opts">
    <label><span>24-hour clock</span><input type="checkbox" class="sw" checked={!hour12} onchange={(e) => { hour12 = !e.currentTarget.checked; save() }} /></label>
    {#if item.widget === 'analog'}
      <label><span>Seconds hand</span><input type="checkbox" class="sw" checked={seconds} onchange={(e) => { seconds = e.currentTarget.checked; save() }} /></label>
    {/if}
  </div>

  <div class="head">World clocks{item.widget === 'world-clocks' ? '' : ' (shown by the World Clocks widget)'}</div>
  <ul class="zones">
    {#each zones as z, i (z.tz + i)}
      {@const zt = zoneTime(z.tz)}
      <li>
        <input class="lbl" value={z.label} maxlength="4" onchange={(e) => { zones[i].label = e.currentTarget.value.toUpperCase(); save() }} title="Short label shown on the dock" />
        <span class="zn" title={z.tz}>{z.tz.split('/').pop()!.replace(/_/g, ' ')}<small>{zt.diff}</small></span>
        <span class="zt">{zt.t}</span>
        <button aria-label="Remove" onclick={() => { zones = zones.filter((_, j) => j !== i); save() }}><X size={12} /></button>
      </li>
    {/each}
  </ul>
  {#if adding}
    <Combo value="" options={zoneOpts} bind:open={adding} onpick={addZone} />
  {:else if zones.length < 6}
    <button class="add" onclick={() => (adding = true)}><Plus size={13} /> Add a city</button>
  {/if}
</div>

<style>
  .cc { width: 330px; padding: 16px; display: flex; flex-direction: column; gap: 12px; }
  .now { text-align: center; }
  .time { font: 600 40px var(--font-display); font-variant-numeric: tabular-nums; letter-spacing: -0.02em; }
  .date { font-size: 13.5px; color: var(--fg); margin-top: 2px; }
  .meta { font-size: 11.5px; color: var(--fg-3); margin-top: 2px; }
  .week { display: grid; grid-template-columns: repeat(7, 1fr); gap: 4px; }
  .day { display: flex; flex-direction: column; align-items: center; padding: 6px 0; border-radius: 9px; background: var(--surface); }
  .day small { font-size: 10px; color: var(--fg-3); }
  .day span { font: 600 14px var(--font-display); }
  .day.today { background: var(--accent); color: white; }
  .day.today small { color: rgba(255, 255, 255, 0.8); }
  .opts { display: flex; flex-direction: column; gap: 6px; }
  .opts label { display: flex; justify-content: space-between; align-items: center; font-size: 12.5px; }
  .head { font-size: 11px; font-weight: 600; color: var(--fg-3); text-transform: uppercase; letter-spacing: .04em; }
  .zones { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 4px; }
  .zones li { display: grid; grid-template-columns: 44px 1fr auto 18px; gap: 8px; align-items: center; padding: 5px 6px; border-radius: 9px; background: var(--surface); }
  .lbl { width: 100%; padding: 3px 4px; border-radius: 6px; border: 1px solid transparent; background: none; font-size: 11px; font-weight: 700; color: var(--fg-2); outline: none; text-align: center; }
  .lbl:hover, .lbl:focus { border-color: var(--border); }
  .zn { display: flex; flex-direction: column; font-size: 12.5px; min-width: 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .zn small { font-size: 10.5px; color: var(--fg-3); }
  .zt { font: 600 14px var(--font-display); font-variant-numeric: tabular-nums; }
  .zones button { color: var(--fg-3); }
  .zones button:hover { color: var(--fg); }
  .add { display: flex; align-items: center; gap: 6px; align-self: flex-start; font-size: 12px; color: var(--fg-2); padding: 5px 10px; border-radius: 8px; background: var(--surface-press); }
  .add:hover { color: var(--fg); }
</style>
