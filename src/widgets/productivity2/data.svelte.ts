// Shared data for Calendar, Day Timeline and Screen Time.
import { invoke } from '@tauri-apps/api/core'
import { ipc } from '../../lib/ipc'
import { isoDay } from '../../lib/services.svelte'
import { parseIcs, type Ev } from './ical'

type CalData = { id: string; name: string; color: string; ics: string; error: string | null }

/** Events from ±60 days (iCal links, refreshed every 15 min) plus AI limit resets from AI widgets. */
export async function loadEvents(force = false): Promise<{ events: Ev[]; errors: string[] }> {
  const key = 'cal:cache'
  const cached = JSON.parse(localStorage.getItem(key) ?? 'null') as { at: number; events: Ev[]; errors: string[] } | null
  let events: Ev[], errors: string[]
  if (!force && cached && Date.now() - cached.at < 15 * 60_000) ({ events, errors } = cached)
  else {
    const from = Date.now() - 60 * 864e5, to = Date.now() + 60 * 864e5
    const data = await invoke<CalData[]>('calendars_fetch')
    events = data.flatMap((c) => parseIcs(c.ics, c.name, c.color, from, to))
    errors = data.filter((c) => c.error).map((c) => `${c.name}: ${c.error}`)
    localStorage.setItem(key, JSON.stringify({ at: Date.now(), events, errors }))
  }
  const mine = ((await ipc.dataRead<{ events: Ev[] }>('calendar-events'))?.events ?? []).map((e) => ({ ...e, cal: 'My events', color: e.color || '#30d158', local: true }))
  return { events: [...events, ...mine, ...aiResets()].sort((a, b) => a.start - b.start), errors }
}

/** AI limit resets, read from what the AI widgets last fetched (brief §4.5: unified calendar). */
function aiResets(): Ev[] {
  const out: Ev[] = []
  for (let i = 0; i < localStorage.length; i++) {
    const k = localStorage.key(i)!
    if (!k.startsWith('svc:') || !/claude|chatgpt|glm/.test(k)) continue
    try {
      const v = JSON.parse(localStorage.getItem(k)!).view
      // Only weekly limits: 5-hour windows reset several times a day and would clutter the calendar.
      for (const l of (v?.limits ?? []).filter((x: any) => /week/i.test(x.label))) {
        const t = typeof l.resets === 'number' ? (l.resets < 1e12 ? l.resets * 1000 : l.resets) : Date.parse(l.resets)
        if (isFinite(t) && t > Date.now()) out.push({ id: `ai:${k}:${l.label}`, title: `${/claude/.test(k) ? 'Claude' : /chatgpt/.test(k) ? 'ChatGPT' : 'GLM'} ${l.label.toLowerCase()} limit resets`, start: t, end: t, allDay: false, cal: 'AI limits', color: '#bf5af2' })
      }
    } catch { /* ignore */ }
  }
  const seen = new Set<string>()
  return out.filter((e) => { const k = e.title + Math.round(e.start / 60000); return !seen.has(k) && !!seen.add(k) })
}

// ---------------------------------------------------------------- screen time
export type Day = { hours: Record<string, Record<string, number>> }
export async function screenDays(n: number): Promise<Record<string, Day>> {
  const days = Array.from({ length: n }, (_, i) => { const d = new Date(); d.setDate(d.getDate() - i); return isoDay(d) })
  return invoke('screen_time', { days })
}
export function totals(day: Day | undefined): [string, number][] {
  const t: Record<string, number> = {}
  for (const apps of Object.values(day?.hours ?? {})) for (const [a, s] of Object.entries(apps)) t[a] = (t[a] ?? 0) + s
  return Object.entries(t).sort((a, b) => b[1] - a[1])
}
export const hm = (s: number) => s >= 3600 ? `${Math.floor(s / 3600)} h ${Math.round((s % 3600) / 60)} min` : `${Math.max(1, Math.round(s / 60))} min`

// App types for the Day Timeline.
export const TYPES: { id: string; label: string; color: string; re: RegExp }[] = [
  { id: 'work', label: 'Work', color: '#0a84ff', re: /^(code|devenv|idea64|pycharm64|rider64|windowsterminal|wt|powershell|pwsh|cmd|claude|cursor|notion|obsidian|excel|winword|powerpnt|figma|tradingview|ninjatrader|gimp)/i },
  { id: 'web', label: 'Web', color: '#64d2ff', re: /^(chrome|msedge|firefox|vivaldi|brave|opera|arc|zen)/i },
  { id: 'chat', label: 'Chat', color: '#30d158', re: /^(discord|slack|teams|ms-teams|whatsapp|telegram|zoom|signal)/i },
  { id: 'media', label: 'Media', color: '#ff375f', re: /^(spotify|vlc|mpv|foobar|plex|netflix)/i },
  { id: 'games', label: 'Games', color: '#ff9f0a', re: /^(steam|playnite|epicgameslauncher|battle\.net|riotclient|.*-win64-shipping|.*game)/i },
]
export const typeOf = (app: string) => TYPES.find((t) => t.re.test(app)) ?? { id: 'other', label: 'Other', color: '#8e8e93', re: /./ }
