// Smart notifications from widgets (Settings → Notifications → Smart notifications). Each fires once
// per event (remembered in localStorage, shared by all Strata windows), never on a schedule:
//  · a to-do's time has come (to-dos with a time, from the To-dos widget)
//  · an AI usage limit passed 80 % and again 95 % (once per reset period)
//  · a calendar event starts in 10 minutes; rain likely around an event in the next 3 hours
// Low disk space is checked by Strata itself (notices.rs).
import { notify } from './notify'
import { ipc } from './ipc'
import { settings } from './stores.svelte'
import type { Limit } from './services.svelte'
import { sysClock } from './clock.svelte'
import { loadEvents } from '../widgets/productivity2/data.svelte'
import { fetchWeather, type Place, type Weather } from '../widgets/weather/weather'

/** The place your weather widgets show (none: no rain heads-ups). Fetched at most every 30 min. */
let wx: { at: number; w: Weather | null } = { at: 0, w: null }
async function weather(): Promise<Weather | null> {
  const place = settings.value?.items.map((i) => i.options?.place as Place | undefined).find((p) => p && typeof p.lat === 'number')
  if (!place) return null
  if (Date.now() - wx.at > 30 * 60_000) wx = { at: Date.now(), w: await fetchWeather(place).catch(() => null) }
  return wx.w
}
const hm = (ms: number) => new Date(ms).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: sysClock.h12 })

function once(key: string) {
  const k = 'smart:sent:' + key
  if (localStorage.getItem(k)) return false
  localStorage.setItem(k, String(Date.now()))
  return true
}

/** One dock window runs the timed checks (several monitors = several docks). */
const me = Math.random().toString(36).slice(2)
function leader() {
  try {
    const cur = JSON.parse(localStorage.getItem('smart:leader') ?? 'null')
    if (cur && cur.id !== me && Date.now() - cur.at < 90_000) return false
  } catch { /* take over */ }
  localStorage.setItem('smart:leader', JSON.stringify({ id: me, at: Date.now() }))
  return true
}

let started = false
export function startSmart() {
  if (started) return
  started = true
  const tick = async () => {
    if (!settings.value?.notifySmart || !leader()) return
    const now = new Date(), day = now.toDateString(), hm = now.toTimeString().slice(0, 5)
    const mins = (t: string) => +t.slice(0, 2) * 60 + +t.slice(3, 5)
    const todos = await ipc.dataRead<{ todos: { id: string; text: string; done: boolean; time?: string }[] }>('todos').catch(() => null)
    for (const t of todos?.todos ?? []) {
      if (t.done || !t.time || t.time > hm || mins(hm) - mins(t.time) > 30) continue
      if (once(`todo:${t.id}:${day}`)) notify({ id: `todo-${t.id}-${day}`, app: 'To-dos', title: t.text, body: `Due at ${t.time}` })
    }
    // Calendar: 10 minutes ahead; rain around an event in the next 3 hours.
    const { events } = await loadEvents().catch(() => ({ events: [] as any[] }))
    const nowMs = Date.now()
    for (const e of events) {
      if (e.allDay || e.cal === 'AI limits' || e.start <= nowMs) continue
      const left = Math.round((e.start - nowMs) / 60000)
      if (left > 180) break
      if (left <= 10 && once(`cal:${e.id}:${e.start}`))
        notify({ id: `cal-${e.id}-${e.start}`, app: e.cal || 'Calendar', title: e.title, body: `Starts in ${Math.max(1, left)} min · ${hm(e.start)}${e.location ? ` · ${e.location}` : ''}` })
      if (left > 10 && !localStorage.getItem(`smart:sent:rain:${e.id}:${e.start}`)) {
        const w = await weather()
        const h = w?.hourly.find((x) => Math.abs(Date.parse(x.time) - e.start) <= 30 * 60_000)
        if (h && h.pop >= 50 && once(`rain:${e.id}:${e.start}`))
          notify({ id: `rain-${e.id}-${e.start}`, app: 'Weather', title: `Rain likely around ${hm(e.start)}`, body: `Take an umbrella for “${e.title}” (${h.pop}% chance of rain).` })
      }
    }
  }
  setTimeout(tick, 5000)
  setInterval(tick, 30_000)
}

/** Called when a service reports usage limits (AI widgets). */
export function checkLimits(service: string, limits: Limit[] | undefined) {
  if (!settings.value?.notifySmart || !limits?.length) return
  for (const l of limits) {
    const level = l.pct >= 95 ? 95 : l.pct >= 80 ? 80 : 0
    if (!level) continue
    const reset = l.resets ? new Date(typeof l.resets === 'number' ? l.resets : Date.parse(String(l.resets))) : null
    const when = reset && !isNaN(+reset) ? reset.toLocaleString([], { weekday: 'short', hour: '2-digit', minute: '2-digit' }) : ''
    if (once(`limit:${service}:${l.label}:${level}:${reset ? +reset : ''}`))
      notify({ id: `limit-${service}-${l.label}-${level}-${reset ? +reset : ''}`, app: service, title: `${service} · ${l.label} at ${Math.round(l.pct)}%`, body: when ? `Resets ${when}` : '' })
  }
}
