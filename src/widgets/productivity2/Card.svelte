<script lang="ts">
  import { sysClock } from '../../lib/clock.svelte'
  // Calendar (month grid: two dots per day then +N; day agenda; manage iCal links) · Screen Time (hour
  // by hour; per-app for today and the week) · Day Timeline (today by app type, meetings,
  // sunrise/sunset) · WhatsApp (recent notifications; open WhatsApp).
  import { ChevronLeft, ChevronRight, Plus, Trash2 } from '@lucide/svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { iconUrl, ipc } from '../../lib/ipc'
  import { ago } from '../../lib/services.svelte'
  import type { CardProps } from '../types'
  import { DEFAULT_PLACE, watchWeather, type Weather } from '../weather/weather'
  import type { Ev } from './ical'
  import { hm, loadEvents, screenDays, totals, TYPES, typeOf, type Day } from './data.svelte'
  let { item }: CardProps = $props()
  const id = item.widget
  const now = new Date()

  // ---------------- calendar
  let events = $state<Ev[]>([])
  let errors = $state<string[]>([])
  let month = $state(new Date(now.getFullYear(), now.getMonth(), 1))
  let picked = $state(new Date(now.getFullYear(), now.getMonth(), now.getDate()))
  let managing = $state(false)
  type Cal = { id: string; name: string; color: string }
  let cals = $state<Cal[]>([])
  let nn = $state(''), nu = $state(''), ncol = $state('#0a84ff'), adding = $state(false), addErr = $state('')
  const COLORS = ['#0a84ff', '#30d158', '#ff9f0a', '#ff375f', '#bf5af2', '#64d2ff']
  async function reload(force = false) { const r = await loadEvents(force); events = r.events; errors = r.errors; cals = await invoke<Cal[]>('calendars_list') }
  if (id === 'calendar' || id === 'day-timeline') reload().catch((e) => (errors = [String(e)]))
  const grid = $derived.by(() => {
    const first = new Date(month); const off = (first.getDay() + 6) % 7
    return Array.from({ length: 42 }, (_, i) => { const d = new Date(first); d.setDate(1 - off + i); return d })
  })
  const on = (d: Date) => events.filter((e) => { const s = new Date(e.start), en = new Date(Math.max(e.end - 1, e.start)); const d0 = new Date(d.getFullYear(), d.getMonth(), d.getDate()), d1 = new Date(d0); d1.setDate(d0.getDate() + 1); return s < d1 && en >= d0 })
  const agenda = $derived(on(picked))
  // Legend: your calendars, your own events, AI limit resets (only those present).
  const legend = $derived([...new Map(events.map((e) => [e.cal, { name: e.cal, color: e.color }])).values()])
  async function addCal() {
    addErr = ''; adding = true
    try { await invoke('calendars_add', { name: nn || 'Calendar', url: nu, color: ncol }); nn = nu = ''; await reload(true) } catch (e) { addErr = String(e) } finally { adding = false }
  }
  // Your own events (kept in %APPDATA%\Strata\calendar-events.json).
  let picking = $state(false)
  let adding2 = $state(false), et = $state(''), eTime = $state('09:00'), eDur = $state(60), eAll = $state(false)
  async function addEvent() {
    if (!et.trim()) return
    const [h, m] = eTime.split(':').map(Number)
    const s = new Date(picked); if (!eAll) s.setHours(h || 0, m || 0, 0, 0)
    const ev = { id: `local:${Date.now()}`, title: et.trim(), start: +s, end: eAll ? +s + 864e5 : +s + eDur * 60_000, allDay: eAll, cal: 'My events', color: '#30d158' }
    const cur = (await ipc.dataRead<{ events: Ev[] }>('calendar-events'))?.events ?? []
    await ipc.dataWrite('calendar-events', { events: [...cur, ev] })
    et = ''; adding2 = false
    await reload()
  }
  async function delEvent(e: Ev) {
    const cur = (await ipc.dataRead<{ events: Ev[] }>('calendar-events'))?.events ?? []
    await ipc.dataWrite('calendar-events', { events: cur.filter((x) => x.id !== e.id) })
    await reload()
  }
  const tm = (ms: number) => new Date(ms).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: sysClock.h12 })

  // ---------------- screen time / timeline
  let days = $state<Record<string, Day>>({})
  let range = $state<'today' | 'week'>('today')
  if (id === 'screen-time' || id === 'day-timeline') screenDays(7).then((d) => (days = d))
  const todayKey = $derived(Object.keys(days).sort().pop())
  const today = $derived(todayKey ? days[todayKey] : undefined)
  const weekTotals = $derived.by(() => { const t: Record<string, number> = {}; for (const d of Object.values(days)) for (const [a, s] of totals(d)) t[a] = (t[a] ?? 0) + s; return Object.entries(t).sort((a, b) => b[1] - a[1]) })
  const list = $derived(range === 'today' ? totals(today) : weekTotals)
  const listTotal = $derived(list.reduce((a, [, s]) => a + s, 0))
  const hourBars = $derived(Array.from({ length: 24 }, (_, h) => Object.values(today?.hours?.[String(h)] ?? {}).reduce((a, b) => a + b, 0)))
  const byType = $derived.by(() => { const t: Record<string, number> = {}; for (const [a, s] of totals(today)) { const k = typeOf(a).id; t[k] = (t[k] ?? 0) + s } return t })
  let w = $state<Weather | null>(null)
  $effect(() => { if (id === 'day-timeline') return watchWeather(DEFAULT_PLACE, (v) => (w = v)) })
  const frac = (iso: string | number) => { const d = new Date(iso); return (d.getHours() * 60 + d.getMinutes()) / 1440 }

  // ---------------- whatsapp
  type Wa = { unread: number; notes: { sender: string; text: string; time: number }[] }
  let wa = $state<Wa | null>(null), waErr = $state('')
  // Notification Center only holds what you haven't dismissed: keep a short local history too.
  let hist = $state<Wa['notes']>(JSON.parse(localStorage.getItem('wa:hist') ?? '[]'))
  if (id === 'whatsapp') invoke<Wa>('whatsapp').then((x) => {
    wa = x
    const seen = new Set(hist.map((n) => n.sender + n.time))
    hist = [...x.notes.filter((n) => !seen.has(n.sender + n.time)), ...hist].sort((a, b) => b.time - a.time).slice(0, 30)
    localStorage.setItem('wa:hist', JSON.stringify(hist))
  }).catch((e) => (waErr = String(e)))
  const unreadKeys = $derived(new Set((wa?.notes ?? []).map((n) => n.sender + n.time)))
</script>

<div class="pc" class:wide={id === 'calendar'}>
  {#if id === 'calendar'}
    <div class="calw">
      <div class="calh">
        <strong class="ctitle">{month.toLocaleDateString(undefined, { month: 'long', year: 'numeric' })}</strong>
        <span class="sp"></span>
        <button class="pill" class:on={managing} onclick={() => (managing = !managing)}>Calendars</button>
        <button class="today-b" onclick={() => { month = new Date(now.getFullYear(), now.getMonth(), 1); picked = new Date(now.getFullYear(), now.getMonth(), now.getDate()) }}>Today</button>
        <button class="nav" aria-label="Previous month" onclick={() => (month = new Date(month.getFullYear(), month.getMonth() - 1, 1))}><ChevronLeft size={16} /></button>
        <button class="nav" aria-label="Next month" onclick={() => (month = new Date(month.getFullYear(), month.getMonth() + 1, 1))}><ChevronRight size={16} /></button>
      </div>
      <div class="legend2">{#each legend as l}<span><i style="background:{l.color}"></i>{l.name}</span>{/each}</div>
      <div class="panes">
        <div class="left">
          <div class="grid2">
            {#each ['MON', 'TUE', 'WED', 'THU', 'FRI', 'SAT', 'SUN'] as d, i}<span class="wd2" class:cur={(now.getDay() + 6) % 7 === i}>{d}</span>{/each}
            {#each grid as d}
              {@const evs = on(d)}
              {#if d.getMonth() !== month.getMonth()}<span class="cell blank"></span>{:else}
                <button class="cell" class:today={d.toDateString() === now.toDateString()} class:sel={d.toDateString() === picked.toDateString()} onclick={() => (picked = d)}>
                  <span class="marks">{#each evs.slice(0, 2) as e}<i style="background:{e.color}"></i>{/each}{#if evs.length > 2}<em>+{evs.length - 2}</em>{/if}</span>
                  <span class="num">{d.getDate()}</span>
                </button>
              {/if}
            {/each}
          </div>
        </div>
        <div class="right">
          {#if managing}
            <div class="ph"><strong>Calendars</strong></div>
            <ul class="cals">{#each cals as c (c.id)}<li><i style="background:{c.color}"></i>{c.name}<button aria-label="Remove" onclick={async () => { await invoke('calendars_remove', { id: c.id }); reload(true) }}><Trash2 size={12} /></button></li>{:else}<li class="muted">No calendars linked yet.</li>{/each}</ul>
            <div class="addc">
              <input bind:value={nn} placeholder="Name (e.g. Work)" />
              <input bind:value={nu} placeholder="Secret iCal address (https:// or webcal://)" type="password" />
              <div class="cols">{#each COLORS as col}<button class:on={ncol === col} style="background:{col}" aria-label={col} onclick={() => (ncol = col)}></button>{/each}
                <button class="primary" disabled={adding || !nu} onclick={addCal}><Plus size={13} /> Add</button></div>
              {#if addErr}<p class="err">{addErr}</p>{/if}
              <p class="muted">Google: Settings → your calendar → "Secret address in iCal format". Outlook: Settings → Calendar → Shared calendars → Publish → ICS. iCloud: share the calendar as Public. Read-only, refreshed every 15 minutes; links stay in Windows Credential Manager.</p>
            </div>
          {:else}
            <div class="ph"><strong>{picked.toDateString() === now.toDateString() ? 'Today' : picked.toLocaleDateString(undefined, { weekday: 'long', day: 'numeric', month: 'short' })}</strong><small>{agenda.length} event{agenda.length === 1 ? '' : 's'}</small></div>
            <ul class="ag2">
              {#each agenda as e (e.id)}
                <li><span class="at2">{e.allDay ? 'All day' : tm(e.start)}</span><span class="bar" style="background:{e.color}"></span>
                  <span class="an"><strong>{e.title}</strong><small>{e.cal}{e.location ? ` · ${e.location}` : ''}</small></span>
                  {#if e.local}<button class="del" aria-label="Delete event" onclick={() => delEvent(e)}><Trash2 size={12} /></button>{/if}</li>
              {:else}<li class="muted">Nothing scheduled.</li>{/each}
            </ul>
            {#each errors as er}<p class="err">{er}</p>{/each}
            <div class="quick">
              <button class="qt" class:on={picking} onclick={() => (picking = !picking)}>{eAll ? 'All day' : eTime}</button>
              <input class="qn" bind:value={et} placeholder="New event {picked.toDateString() === now.toDateString() ? 'today' : 'on this day'}" onkeydown={(e) => e.key === 'Enter' && addEvent()} />
              <button class="qa" aria-label="Add event" title="Add (Enter). Duration {eDur} min" onclick={addEvent}><Plus size={15} /></button>
            </div>
            {#if picking}
              <div class="tp">
                <div class="tph"><span>Hour</span><button class="allday" class:on={eAll} onclick={() => { eAll = !eAll; if (eAll) picking = false }}>All day</button></div>
                <div class="hgrid">{#each Array(24) as _, h}<button class:on={!eAll && +eTime.slice(0, 2) === h} onclick={() => { eAll = false; eTime = String(h).padStart(2, '0') + eTime.slice(2) }}>{String(h).padStart(2, '0')}</button>{/each}</div>
                <div class="tph"><span>Minutes</span></div>
                <div class="mgrid">{#each ['00', '15', '30', '45'] as m}<button class:on={!eAll && eTime.slice(3) === m} onclick={() => { eAll = false; eTime = eTime.slice(0, 3) + m; picking = false }}>:{m}</button>{/each}</div>
              </div>
            {/if}
            {#if !eAll}<div class="durs">{#each [[15, '15 min'], [30, '30 min'], [60, '1 h'], [90, '1.5 h'], [120, '2 h'], [240, '4 h']] as [v, l]}<button class:on={eDur === v} onclick={() => (eDur = +v)}>{l}</button>{/each}</div>{/if}
          {/if}
        </div>
      </div>
    </div>
  {:else if id === 'screen-time'}
    <div class="seg"><button class:on={range === 'today'} onclick={() => (range = 'today')}>Today</button><button class:on={range === 'week'} onclick={() => (range = 'week')}>7 days</button></div>
    <div class="big">{hm(listTotal)}</div>
    {#if range === 'today'}
      <div class="bars">{#each hourBars as s, h}<i style="height:{Math.max(2, (s / 3600) * 56)}px" title="{h}:00 · {hm(s)}"></i>{/each}</div>
      <div class="axis"><span>0</span><span>6</span><span>12</span><span>18</span><span>24</span></div>
    {/if}
    <ul class="apps">
      {#each list.slice(0, 12) as [a, s]}
        <li><img src={iconUrl(`${a}.exe`, 32)} alt="" onerror={(e) => ((e.currentTarget as HTMLImageElement).style.visibility = 'hidden')} /><span class="n">{a}</span><span class="b"><i style="width:{(s / (list[0]?.[1] || 1)) * 100}%; background:{typeOf(a).color}"></i></span><span class="v">{hm(s)}</span></li>
      {:else}<li class="muted">Tracking starts now: Strata samples the app in front every 10 seconds (idle time isn't counted). Data stays on this PC.</li>{/each}
    </ul>
  {:else if id === 'day-timeline'}
    <div class="tlw">
      <div class="tl">
        {#each Array(24) as _, h}
          {@const apps = today?.hours?.[String(h)] ?? {}}
          {@const tot = Object.values(apps).reduce((a, b) => a + b, 0)}
          <span class="col">{#each Object.entries(apps).sort((a, b) => b[1] - a[1]) as [a, s]}<i style="flex:{s}; background:{typeOf(a).color}"></i>{/each}<i style="flex:{Math.max(0, 3600 - tot)}"></i></span>
        {/each}
        {#if w}<span class="sun" style="left:{frac(w.daily[0].sunrise) * 100}%" title="Sunrise {tm(Date.parse(w.daily[0].sunrise))}">☀</span><span class="sun set" style="left:{frac(w.daily[0].sunset) * 100}%" title="Sunset {tm(Date.parse(w.daily[0].sunset))}">☾</span>{/if}
        {#each events.filter((e) => !e.allDay && new Date(e.start).toDateString() === now.toDateString()) as e}<span class="mt" style="left:{frac(e.start) * 100}%; width:{Math.max(0.4, ((e.end - e.start) / 864e5) * 100)}%" title="{tm(e.start)} {e.title}"></span>{/each}
        <span class="now" style="left:{frac(Date.now()) * 100}%"></span>
      </div>
      <div class="axis"><span>0</span><span>6</span><span>12</span><span>18</span><span>24</span></div>
    </div>
    <div class="legend">{#each [...TYPES, { id: 'other', label: 'Other', color: '#8e8e93' }] as t}{#if byType[t.id]}<span><i style="background:{t.color}"></i>{t.label} {hm(byType[t.id])}</span>{/if}{/each}<span><i class="mtl"></i>Meetings</span></div>
    {#if w}<p class="muted">Sunrise {tm(Date.parse(w.daily[0].sunrise))} · Sunset {tm(Date.parse(w.daily[0].sunset))}</p>{/if}
    <p class="muted">Each hour shows which apps were in front, from Screen Time (Strata checks the app in front every 10 seconds while you're active; it started when Strata was installed, so earlier hours are empty). Apps are grouped by type: Work (code, terminals, office, trading), Web (browsers), Chat, Media, Games, Other. Meetings come from your calendars.</p>
  {:else if id === 'whatsapp'}
    <div class="wah"><strong>WhatsApp</strong>{#if wa?.unread}<span class="badge">{wa.unread} unread</span>{/if}<button class="link" onclick={() => ipc.launch('whatsapp:')}>Open</button></div>
    {#if waErr}<p class="err">{waErr}</p>{/if}
    <ul class="ag">
      {#each hist as n (n.sender + n.time)}<li class="wa" class:unread={unreadKeys.has(n.sender + n.time)}><span class="an"><strong>{n.sender}</strong><small class="wrap">{n.text}</small></span><span class="at">{ago(n.time)}</span></li>
      {:else}<li class="muted">No WhatsApp messages yet. Strata reads WhatsApp's Windows notifications (unread = still in Notification Center), so WhatsApp's notifications must be on.</li>{/each}
    </ul>
  {/if}
</div>

<style>
  .pc { width: 340px; padding: 14px; display: flex; flex-direction: column; gap: 9px; max-height: 640px; overflow-y: auto; }
  .pc.wide { width: 720px; }
  .calh { display: flex; align-items: center; gap: 6px; }
  .ctitle { font: 700 22px var(--font-display); letter-spacing: -0.01em; }
  .sp { flex: 1; }
  .pill { padding: 6px 13px; border-radius: 999px; background: var(--surface); font-size: 12.5px; }
  .pill.on, .pill:hover { background: var(--surface-press); }
  .today-b { padding: 6px 10px; font-size: 12.5px; color: var(--accent); }
  .nav { width: 28px; height: 28px; border-radius: 8px; display: grid; place-items: center; }
  .nav:hover { background: var(--surface-hover); }
  .legend2 { display: flex; flex-wrap: wrap; gap: 12px; font-size: 11.5px; color: var(--fg-2); }
  .legend2 span { display: flex; align-items: center; gap: 5px; }
  .legend2 i { width: 7px; height: 7px; border-radius: 4px; }
  .panes { display: grid; grid-template-columns: 1fr 280px; gap: 14px; }
  .grid2 { display: grid; grid-template-columns: repeat(7, 1fr); gap: 5px; }
  .wd2 { text-align: center; font-size: 10px; font-weight: 600; color: var(--fg-3); padding: 4px 0; border-radius: 999px; background: var(--surface); }
  .wd2.cur { color: var(--fg); }
  .cell { position: relative; aspect-ratio: 1 / 0.92; border-radius: 10px; background: var(--surface); display: flex; flex-direction: column; justify-content: flex-end; align-items: center; padding-bottom: 5px; font: 500 13px var(--font-display); }
  .cell.blank { background: none; }
  .cell:hover { background: var(--surface-hover); }
  .cell.today .num { color: var(--accent); font-weight: 700; }
  .cell.sel { box-shadow: inset 0 0 0 2px var(--accent); }
  .marks { position: absolute; top: 5px; right: 6px; display: flex; gap: 2px; align-items: center; }
  .marks i { width: 6px; height: 6px; border-radius: 3px; }
  .marks em { font-size: 8.5px; font-style: normal; color: var(--fg-3); }
  .right { display: flex; flex-direction: column; gap: 8px; padding: 12px; border-radius: 14px; background: var(--surface); min-height: 300px; }
  .ph { display: flex; align-items: baseline; justify-content: space-between; font: 600 15px var(--font-display); }
  .ph small { font-size: 11.5px; color: var(--fg-3); font-weight: 400; }
  .ag2 { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 8px; flex: 1; }
  .ag2 li { display: flex; align-items: flex-start; gap: 8px; font-size: 12.5px; }
  .at2 { width: 56px; flex: none; font-size: 11.5px; color: var(--fg-3); font-variant-numeric: tabular-nums; padding-top: 1px; }
  .bar { width: 3px; align-self: stretch; border-radius: 2px; flex: none; }
  .quick { display: flex; align-items: center; gap: 6px; padding: 4px; border-radius: 999px; background: var(--surface-press); }
  .tp { display: flex; flex-direction: column; gap: 5px; padding: 8px; border-radius: 12px; background: var(--surface-press); }
  .tph { display: flex; justify-content: space-between; align-items: center; font-size: 10.5px; color: var(--fg-3); text-transform: uppercase; letter-spacing: .04em; }
  .hgrid { display: grid; grid-template-columns: repeat(8, 1fr); gap: 3px; }
  .mgrid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 3px; }
  .hgrid button, .mgrid button, .durs button, .allday { padding: 4px 0; border-radius: 7px; font-size: 11.5px; background: var(--surface); font-variant-numeric: tabular-nums; }
  .allday { padding: 3px 9px; text-transform: none; letter-spacing: 0; }
  .hgrid button:hover, .mgrid button:hover, .durs button:hover { background: var(--surface-hover); }
  .hgrid button.on, .mgrid button.on, .durs button.on, .allday.on, .qt.on { background: var(--accent); color: white; }
  .durs { display: grid; grid-template-columns: repeat(6, 1fr); gap: 3px; }
  .qt { width: 72px; cursor: pointer; padding: 5px 8px; border-radius: 999px; border: 0; background: var(--surface); font-size: 12px; outline: none; }
  .qn { flex: 1; min-width: 0; background: none; border: 0; outline: none; font-size: 12.5px; }
  .qa { width: 28px; height: 28px; border-radius: 14px; display: grid; place-items: center; background: var(--surface); }
  .qa:hover { background: var(--accent); color: white; }
  .qopts { display: flex; align-items: center; gap: 8px; font-size: 11.5px; color: var(--fg-3); }
  .qopts select { padding: 3px 6px; border-radius: 7px; border: 1px solid var(--border); background: var(--surface); font-size: 11.5px; outline: none; }
  .mh { display: flex; align-items: center; justify-content: space-between; font: 600 14px var(--font-display); }
  .mh button { width: 26px; height: 26px; border-radius: 8px; display: grid; place-items: center; }
  .mh button:hover { background: var(--surface-hover); }
  .grid { display: grid; grid-template-columns: repeat(7, 1fr); gap: 2px; }
  .wd { text-align: center; font-size: 10.5px; color: var(--fg-3); padding-bottom: 2px; }
  .day { display: flex; flex-direction: column; align-items: center; gap: 2px; padding: 4px 0 3px; border-radius: 8px; font-size: 12.5px; }
  .day:hover { background: var(--surface-hover); }
  .day.out { color: var(--fg-3); }
  .day.today > span:first-child { background: var(--danger); color: white; border-radius: 10px; padding: 0 6px; }
  .day.sel { background: var(--surface-press); }
  .dots { display: flex; gap: 2px; height: 5px; align-items: center; }
  .dots i { width: 5px; height: 5px; border-radius: 3px; }
  .dots em { font-size: 8px; font-style: normal; color: var(--fg-3); }
  .head { font-size: 11px; font-weight: 600; color: var(--fg-3); text-transform: uppercase; letter-spacing: .04em; }
  .ag { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 4px; }
  .ag li { display: flex; align-items: flex-start; gap: 8px; padding: 6px 8px; border-radius: 9px; background: var(--surface); font-size: 12.5px; }
  .ag li > i { width: 4px; align-self: stretch; border-radius: 2px; flex: none; }
  .at { font-size: 11.5px; color: var(--fg-2); font-variant-numeric: tabular-nums; flex: none; min-width: 42px; }
  .an { display: flex; flex-direction: column; min-width: 0; flex: 1; }
  .an strong { font-weight: 500; overflow: hidden; text-overflow: ellipsis; }
  .an small { font-size: 11px; color: var(--fg-3); overflow: hidden; text-overflow: ellipsis; }
  .link { align-self: flex-start; font-size: 12px; color: var(--fg-2); padding: 4px 9px; border-radius: 7px; background: var(--surface-press); }
  .del { color: var(--fg-3); margin-left: auto; }
  .del:hover { color: var(--danger); }
  .chk { display: flex; align-items: center; gap: 4px; font-size: 12px; }
  .cols input[type='time'], .cols select { padding: 4px 6px; border-radius: 7px; border: 1px solid var(--border); background: var(--surface); font-size: 12px; outline: none; }
  .cols select option { background: #222; }
  .cals { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 3px; }
  .cals li { display: flex; align-items: center; gap: 8px; font-size: 12.5px; }
  .cals li i { width: 9px; height: 9px; border-radius: 5px; }
  .cals li button { margin-left: auto; color: var(--fg-3); }
  .addc { display: flex; flex-direction: column; gap: 6px; }
  .addc input { padding: 7px 9px; border-radius: 8px; border: 1px solid var(--border); background: var(--surface); outline: none; font-size: 12.5px; }
  .cols { display: flex; gap: 6px; align-items: center; }
  .cols button:not(.primary) { width: 18px; height: 18px; border-radius: 9px; }
  .cols button.on { box-shadow: 0 0 0 2px var(--fg); }
  .cols .primary { margin-left: auto; display: flex; align-items: center; gap: 4px; padding: 5px 12px; border-radius: 8px; background: var(--accent); color: white; font-size: 12px; }
  .seg { display: flex; gap: 4px; padding: 3px; border-radius: 9px; background: var(--surface); }
  .seg button { flex: 1; padding: 5px 0; border-radius: 7px; font-size: 12px; color: var(--fg-2); }
  .seg button.on { background: var(--surface-press); color: var(--fg); }
  .big { font: 600 28px var(--font-display); }
  .bars { display: flex; align-items: flex-end; gap: 2px; height: 58px; }
  .bars i { flex: 1; background: var(--accent); border-radius: 2px 2px 0 0; }
  .axis { display: flex; justify-content: space-between; font-size: 10px; color: var(--fg-3); }
  .apps { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 5px; }
  .apps li { display: grid; grid-template-columns: 18px 90px 1fr 64px; gap: 8px; align-items: center; font-size: 12px; }
  .apps img { width: 16px; height: 16px; }
  .apps .n { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .apps .b { height: 6px; border-radius: 3px; background: var(--surface-press); overflow: hidden; }
  .apps .b i { display: block; height: 100%; border-radius: 3px; }
  .apps .v { text-align: right; color: var(--fg-2); font-variant-numeric: tabular-nums; }
  .tlw { display: flex; flex-direction: column; gap: 3px; }
  .tl { position: relative; display: grid; grid-template-columns: repeat(24, 1fr); gap: 1px; height: 46px; margin-top: 12px; }
  .tl .col { display: flex; flex-direction: column-reverse; border-radius: 3px; overflow: hidden; background: var(--surface); }
  .mt { position: absolute; top: -8px; height: 4px; border-radius: 2px; background: var(--fg); opacity: 0.8; }
  .now { position: absolute; top: -10px; bottom: -2px; width: 2px; background: var(--danger); border-radius: 1px; }
  .sun { position: absolute; top: -14px; transform: translateX(-50%); font-size: 10px; color: #ffcc00; }
  .sun.set { color: #9aa0ff; }
  .legend { display: flex; flex-wrap: wrap; gap: 8px; font-size: 11.5px; color: var(--fg-2); }
  .legend span { display: flex; align-items: center; gap: 4px; }
  .legend i { width: 8px; height: 8px; border-radius: 2px; }
  .legend .mtl { background: var(--fg); height: 3px; }
  .muted { color: var(--fg-3); font-size: 11.5px; margin: 0; line-height: 1.4; }
  .wrap { white-space: normal !important; overflow-wrap: anywhere; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; }
  li.wa { opacity: 0.7; }
  li.wa.unread { opacity: 1; box-shadow: inset 3px 0 #25d366; }
  .wah { display: flex; align-items: center; gap: 8px; font: 600 14px var(--font-display); }
  .wah .link { margin-left: auto; flex: none; }
  .badge { font-size: 11px; font-weight: 600; color: #25d366; margin-left: 6px; }
  .err { color: var(--danger); font-size: 12px; margin: 0; }
</style>
