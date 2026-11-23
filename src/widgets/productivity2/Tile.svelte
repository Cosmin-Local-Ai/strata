<script lang="ts">
  import { sysClock } from '../../lib/clock.svelte'
  // Calendar (next event today) · Screen Time (today's total) · Day Timeline (today as one line by app
  // type, now marker, meetings, sunrise/sunset) · WhatsApp (unread count and last sender).
  import { CalendarDays, Hourglass, MessageCircle } from '@lucide/svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { clock } from '../../lib/stores.svelte'
  import type { TileProps } from '../types'
  import type { Ev } from './ical'
  import { hm, loadEvents, screenDays, totals, typeOf, type Day } from './data.svelte'
  let { item }: TileProps = $props()
  const id = $derived(item.widget)
  const c = clock(60_000)

  let events = $state<Ev[]>([])
  $effect(() => {
    if (id !== 'calendar' && id !== 'day-timeline') return
    const load = () => loadEvents().then((r) => (events = r.events)).catch(() => {})
    load()
    const t = setInterval(load, 15 * 60_000)
    return () => clearInterval(t)
  })
  const next = $derived(events.find((e) => e.end > +c.now && new Date(e.start).toDateString() === c.now.toDateString() && !e.allDay))
  const todayCount = $derived(events.filter((e) => new Date(e.start).toDateString() === c.now.toDateString()).length)

  let day = $state<Day | undefined>()
  $effect(() => {
    if (id !== 'screen-time' && id !== 'day-timeline') return
    c.now
    screenDays(1).then((d) => (day = Object.values(d)[0])).catch(() => {})
  })
  const total = $derived(totals(day).reduce((a, [, s]) => a + s, 0))
  // 24 hour columns, each filling from the bottom with stacked colours (minutes per app type).
  const hours = $derived(Array.from({ length: 24 }, (_, h) => {
    const sums: Record<string, { s: number; color: string }> = {}
    for (const [a, sec] of Object.entries(day?.hours?.[String(h)] ?? {})) { const t = typeOf(a); sums[t.id] = { s: (sums[t.id]?.s ?? 0) + sec, color: t.color } }
    return Object.values(sums).sort((x, y) => y.s - x.s)
  }))
  const nowFrac = $derived((c.now.getHours() * 60 + c.now.getMinutes()) / 1440)

  type Wa = { unread: number; notes: { sender: string; text: string; time: number }[] }
  let wa = $state<Wa | null>(null)
  let waErr = $state(false)
  $effect(() => {
    if (id !== 'whatsapp') return
    const load = () => invoke<Wa>('whatsapp').then((w) => { wa = w; waErr = false }).catch(() => (waErr = true))
    load()
    const t = setInterval(load, 20_000)
    return () => clearInterval(t)
  })
  const t2 = (ms: number) => new Date(ms).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: sysClock.h12 })
</script>

{#if id === 'calendar'}
  <span class="row">
    <span class="cal"><small>{c.now.toLocaleDateString(undefined, { month: 'short' })}</small><b>{c.now.getDate()}</b></span>
    <span class="col">{#if next}<strong>{next.title}</strong><small>{t2(next.start)}{next.start > +c.now ? '' : ' · now'}</small>{:else}<strong>{todayCount ? `${todayCount} today` : 'No events'}</strong><small>{c.now.toLocaleDateString(undefined, { weekday: 'long' })}</small>{/if}</span>
  </span>
{:else if id === 'screen-time'}
  <span class="row"><Hourglass size={16} /><span class="col"><strong>{total ? hm(total) : '0 min'}</strong><small>{totals(day)[0]?.[0] ?? 'today'}</small></span></span>
{:else if id === 'day-timeline'}
  <span class="row">
    <span class="tl">
      {#each hours as h, i}<i title="{i}:00 · {hm(h.reduce((a, x) => a + x.s, 0))}">{#each h as seg}<b style="height:{Math.min(100, (seg.s / 3600) * 100)}%; background:{seg.color}"></b>{/each}</i>{/each}
      {#each events.filter((e) => !e.allDay && new Date(e.start).toDateString() === c.now.toDateString()) as e}
        <u style="left:{((new Date(e.start).getHours() * 60 + new Date(e.start).getMinutes()) / 1440) * 100}%"></u>
      {/each}
      <em style="left:{nowFrac * 100}%"></em>
    </span>
  </span>
{:else if id === 'whatsapp'}
  <span class="row" title={waErr ? 'Could not read notifications' : ''}>
    <span class="wa"><MessageCircle size={17} />{#if wa?.unread}<b>{wa.unread > 99 ? '99+' : wa.unread}</b>{/if}</span>
    <span class="col"><strong>{wa?.notes[0]?.sender ?? 'WhatsApp'}</strong><small>{wa?.notes[0] ? (wa.notes[0].text || 'New message') : 'No new messages'}</small></span>
  </span>
{/if}

<style>
  .row { display: flex; align-items: center; gap: 8px; padding: 0 12px; height: 100%; white-space: nowrap; }
  .col { display: flex; flex-direction: column; line-height: 1.15; min-width: 0; max-width: 150px; }
  strong { font: 600 12.5px var(--font-display); overflow: hidden; text-overflow: ellipsis; }
  small { font-size: 10.5px; color: var(--fg-2); overflow: hidden; text-overflow: ellipsis; }
  .cal { display: flex; flex-direction: column; align-items: center; width: 26px; border-radius: 6px; overflow: hidden; background: var(--surface); line-height: 1; }
  .cal small { width: 100%; text-align: center; background: var(--danger); color: white; font-size: 8px; font-weight: 700; padding: 1px 0; text-transform: uppercase; }
  .cal b { font: 600 13px var(--font-display); padding: 2px 0 3px; }
  .tl { position: relative; display: grid; grid-template-columns: repeat(24, 1fr); gap: 1px; width: 132px; height: 16px; }
  .tl i { border-radius: 2px; background: var(--surface-press); display: flex; flex-direction: column-reverse; overflow: hidden; }
  .tl b { display: block; width: 100%; flex: none; }
  .tl u { position: absolute; top: -3px; width: 2px; height: 18px; background: var(--fg); border-radius: 1px; opacity: 0.7; }
  .tl em { position: absolute; top: -4px; width: 2px; height: 20px; background: var(--danger); border-radius: 1px; }
  .wa { position: relative; display: grid; }
  .wa b { position: absolute; top: -6px; right: -8px; min-width: 15px; height: 15px; padding: 0 3px; border-radius: 8px; background: #25d366; color: #000; font: 700 9px/15px var(--font); text-align: center; }
</style>
