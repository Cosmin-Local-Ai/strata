<script lang="ts">
  // Day / Month / Year progress: how far along you are, what's left, and a countdown to a date you pick.
  import { clock } from '../../lib/stores.svelte'
  import type { CardProps } from '../types'
  import Ring from '../system/Ring.svelte'
  let { item, act }: CardProps = $props()
  const id = item.widget
  const c = clock(1000)
  const n = $derived(c.now)
  const y = $derived(n.getFullYear())
  const dayStart = $derived(new Date(y, n.getMonth(), n.getDate()))
  const monthDays = $derived(new Date(y, n.getMonth() + 1, 0).getDate())
  const yearLen = $derived(new Date(y, 1, 29).getDate() === 29 ? 366 : 365)
  const doy = $derived(Math.floor((+dayStart - +new Date(y, 0, 1)) / 864e5) + 1)
  const frac = $derived(id === 'day-progress' ? (+n - +dayStart) / 864e5 : id === 'month-progress' ? (n.getDate() - 1 + (+n - +dayStart) / 864e5) / monthDays : (doy - 1 + (+n - +dayStart) / 864e5) / yearLen)
  const left = $derived.by(() => {
    if (id === 'day-progress') { const ms = +dayStart + 864e5 - +n; return `${Math.floor(ms / 36e5)} h ${Math.floor((ms % 36e5) / 6e4)} min left today` }
    if (id === 'month-progress') return `${monthDays - n.getDate()} days left in ${n.toLocaleDateString(undefined, { month: 'long' })}`
    return `${yearLen - doy} days left in ${y}`
  })
  const week = $derived.by(() => { const d = new Date(Date.UTC(y, n.getMonth(), n.getDate())); const k = d.getUTCDay() || 7; d.setUTCDate(d.getUTCDate() + 4 - k); return Math.ceil(((+d - Date.UTC(d.getUTCFullYear(), 0, 1)) / 864e5 + 1) / 7) })
  const quarters = $derived([0, 1, 2, 3].map((q) => { const s = new Date(y, q * 3, 1), e = new Date(y, q * 3 + 3, 1); return Math.max(0, Math.min(1, (+n - +s) / (+e - +s))) }))
  const hours = $derived(Array.from({ length: 24 }, (_, h) => h < n.getHours() ? 1 : h === n.getHours() ? n.getMinutes() / 60 : 0))

  // Countdown to a date of your choice (kept in the widget's options).
  let target = $state<string>(item.options.until ?? '')
  let label = $state<string>(item.options.untilLabel ?? '')
  const daysTo = $derived(target ? Math.ceil((+new Date(target + 'T00:00') - +dayStart) / 864e5) : null)
</script>

<div class="pc">
  <div class="top">
    <Ring value={frac} size={64} stroke={6} label={`${Math.round(frac * 100)}%`} />
    <div class="tx">
      <strong>{id === 'day-progress' ? 'Today' : id === 'month-progress' ? n.toLocaleDateString(undefined, { month: 'long', year: 'numeric' }) : String(y)}</strong>
      <span>{left}</span>
      <small>{id === 'day-progress' ? n.toLocaleDateString(undefined, { weekday: 'long', day: 'numeric', month: 'long' }) : id === 'month-progress' ? `Day ${n.getDate()} of ${monthDays}` : `Day ${doy} of ${yearLen} · week ${week}`}</small>
    </div>
  </div>

  {#if id === 'day-progress'}
    <div class="hrs">{#each hours as f, h}<i title="{h}:00"><b style="height:{f * 100}%"></b></i>{/each}</div>
    <div class="axis"><span>0</span><span>6</span><span>12</span><span>18</span><span>24</span></div>
  {:else if id === 'month-progress'}
    <div class="days">{#each Array(monthDays) as _, i}<i class:past={i + 1 < n.getDate()} class:now={i + 1 === n.getDate()}>{i + 1}</i>{/each}</div>
  {:else}
    <div class="qs">{#each quarters as q, i}<div><small>Q{i + 1}</small><span class="bar"><b style="width:{q * 100}%"></b></span></div>{/each}</div>
    <div class="months">{#each Array(12) as _, m}<i class:past={m < n.getMonth()} class:now={m === n.getMonth()}>{new Date(y, m, 1).toLocaleDateString(undefined, { month: 'short' })}</i>{/each}</div>
  {/if}

  <div class="cd">
    <div class="cdh">Countdown</div>
    {#if daysTo !== null}<div class="cdv"><strong>{Math.abs(daysTo)}</strong> {daysTo >= 0 ? 'days until' : 'days since'} {label || new Date(target + 'T00:00').toLocaleDateString()}</div>{/if}
    <div class="cdf">
      <input bind:value={label} placeholder="What (e.g. Holiday)" onchange={() => act('set-options', { until: target, untilLabel: label })} />
      <input type="date" bind:value={target} onchange={() => act('set-options', { until: target, untilLabel: label })} />
    </div>
  </div>
</div>

<style>
  .pc { width: 330px; padding: 16px; display: flex; flex-direction: column; gap: 12px; }
  .top { display: flex; align-items: center; gap: 14px; }
  .tx { display: flex; flex-direction: column; gap: 2px; }
  .tx strong { font: 600 17px var(--font-display); }
  .tx span { font-size: 13px; }
  .tx small { font-size: 11.5px; color: var(--fg-3); }
  .hrs { display: grid; grid-template-columns: repeat(24, 1fr); gap: 2px; height: 34px; }
  .hrs i { background: var(--surface-press); border-radius: 2px; display: flex; flex-direction: column-reverse; overflow: hidden; }
  .hrs b { display: block; background: var(--accent); }
  .axis { display: flex; justify-content: space-between; font-size: 10px; color: var(--fg-3); }
  .days { display: grid; grid-template-columns: repeat(7, 1fr); gap: 4px; }
  .days i, .months i { font-style: normal; text-align: center; font-size: 11.5px; padding: 5px 0; border-radius: 7px; background: var(--surface); color: var(--fg-3); }
  .days i.past, .months i.past { background: color-mix(in srgb, var(--accent) 35%, transparent); color: var(--fg); }
  .days i.now, .months i.now { background: var(--accent); color: white; font-weight: 600; }
  .months { display: grid; grid-template-columns: repeat(6, 1fr); gap: 4px; }
  .qs { display: grid; grid-template-columns: repeat(4, 1fr); gap: 8px; }
  .qs small { font-size: 10.5px; color: var(--fg-3); }
  .bar { display: block; height: 5px; border-radius: 3px; background: var(--surface-press); overflow: hidden; margin-top: 3px; }
  .bar b { display: block; height: 100%; background: var(--accent); }
  .cd { display: flex; flex-direction: column; gap: 6px; padding-top: 10px; border-top: 1px solid var(--border); }
  .cdh { font-size: 11px; font-weight: 600; color: var(--fg-3); text-transform: uppercase; letter-spacing: .04em; }
  .cdv { font-size: 13px; }
  .cdv strong { font: 600 22px var(--font-display); }
  .cdf { display: grid; grid-template-columns: 1fr 130px; gap: 6px; }
  .cdf input { padding: 6px 8px; border-radius: 8px; border: 1px solid var(--border); background: var(--surface); outline: none; font-size: 12px; }
</style>
