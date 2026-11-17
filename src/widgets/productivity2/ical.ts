// Minimal iCalendar reader for read-only calendars: VEVENTs with DTSTART/DTEND/DURATION, all-day
// events, TZID (IANA; Windows zone names fall back to local time), RRULE (DAILY/WEEKLY/MONTHLY/YEARLY
// with INTERVAL, COUNT, UNTIL, BYDAY) and EXDATE. Expanded into a window of dates.

export type Ev = { id: string; title: string; start: number; end: number; allDay: boolean; location?: string; cal: string; color: string; local?: boolean }

function unfold(ics: string): string[] { return ics.replace(/\r\n[ \t]/g, '').replace(/\n[ \t]/g, '').split(/\r?\n/) }
function unescape(s: string) { return s.replace(/\\n/gi, ' ').replace(/\\,/g, ',').replace(/\\;/g, ';').replace(/\\\\/g, '\\') }

/** UTC ms for a wall-clock time in an IANA zone. */
function zoned(y: number, mo: number, d: number, h: number, mi: number, s: number, tz?: string): number {
  const guess = Date.UTC(y, mo, d, h, mi, s)
  if (!tz) return new Date(y, mo, d, h, mi, s).getTime()
  try {
    const f = new Intl.DateTimeFormat('en-US', { timeZone: tz, hourCycle: 'h23', year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', second: '2-digit' })
    const p = Object.fromEntries(f.formatToParts(new Date(guess)).map((x) => [x.type, x.value]))
    const asTz = Date.UTC(+p.year, +p.month - 1, +p.day, +p.hour, +p.minute, +p.second)
    return guess - (asTz - guess)
  } catch { return new Date(y, mo, d, h, mi, s).getTime() }
}

function parseDate(v: string, params: string): { t: number; allDay: boolean } {
  const m = v.match(/^(\d{4})(\d{2})(\d{2})(?:T(\d{2})(\d{2})(\d{2})(Z)?)?/)
  if (!m) return { t: NaN, allDay: false }
  const [, y, mo, d, h, mi, s, z] = m
  if (!h) return { t: new Date(+y, +mo - 1, +d).getTime(), allDay: true }
  if (z) return { t: Date.UTC(+y, +mo - 1, +d, +h, +mi, +s), allDay: false }
  const tz = params.match(/TZID=([^;:]+)/)?.[1]?.replace(/"/g, '')
  return { t: zoned(+y, +mo - 1, +d, +h, +mi, +s, tz && tz.includes('/') ? tz : undefined), allDay: false }
}

function duration(v: string): number {
  const m = v.match(/P(?:(\d+)W)?(?:(\d+)D)?(?:T(?:(\d+)H)?(?:(\d+)M)?(?:(\d+)S)?)?/)
  if (!m) return 0
  return ((+(m[1] ?? 0) * 7 + +(m[2] ?? 0)) * 86400 + +(m[3] ?? 0) * 3600 + +(m[4] ?? 0) * 60 + +(m[5] ?? 0)) * 1000
}

const DAYS = ['SU', 'MO', 'TU', 'WE', 'TH', 'FR', 'SA']

export function parseIcs(ics: string, cal: string, color: string, from: number, to: number): Ev[] {
  const out: Ev[] = []
  let cur: Record<string, { v: string; p: string }[]> | null = null
  for (const line of unfold(ics)) {
    if (line === 'BEGIN:VEVENT') { cur = {}; continue }
    if (line === 'END:VEVENT' && cur) { expand(cur); cur = null; continue }
    if (!cur) continue
    const i = line.indexOf(':')
    if (i < 0) continue
    const head = line.slice(0, i), v = line.slice(i + 1)
    const [name, ...ps] = head.split(';')
    ;(cur[name] ??= []).push({ v, p: ps.join(';') })
  }
  return out

  function expand(e: Record<string, { v: string; p: string }[]>) {
    if (!e.DTSTART || e.STATUS?.[0]?.v === 'CANCELLED') return
    const s = parseDate(e.DTSTART[0].v, e.DTSTART[0].p)
    if (!isFinite(s.t)) return
    const end = e.DTEND ? parseDate(e.DTEND[0].v, e.DTEND[0].p).t : s.t + (e.DURATION ? duration(e.DURATION[0].v) : s.allDay ? 864e5 : 0)
    const len = Math.max(0, end - s.t)
    const title = unescape(e.SUMMARY?.[0]?.v ?? '(no title)')
    const location = e.LOCATION ? unescape(e.LOCATION[0].v) : undefined
    const uid = e.UID?.[0]?.v ?? title
    const ex = new Set((e.EXDATE ?? []).flatMap((x) => x.v.split(',').map((d) => parseDate(d, x.p).t)))
    const push = (t: number) => { if (!ex.has(t) && t + len >= from && t <= to) out.push({ id: `${cal}:${uid}:${t}`, title, start: t, end: t + len, allDay: s.allDay, location, cal, color }) }
    const rr = e.RRULE?.[0]?.v
    if (!rr) { push(s.t); return }
    const r = Object.fromEntries(rr.split(';').map((kv) => kv.split('=')))
    const freq = r.FREQ, interval = +(r.INTERVAL ?? 1)
    const count = r.COUNT ? +r.COUNT : Infinity
    const until = r.UNTIL ? parseDate(r.UNTIL, '').t : Infinity
    const byday: number[] = r.BYDAY ? r.BYDAY.split(',').map((d: string) => DAYS.indexOf(d.slice(-2))).filter((x: number) => x >= 0) : []
    const base = new Date(s.t)
    let n = 0
    for (let k = 0; k < 2000 && n < count; k++) {
      const d = new Date(base)
      if (freq === 'DAILY') d.setDate(base.getDate() + k * interval)
      else if (freq === 'WEEKLY') d.setDate(base.getDate() + k * 7 * interval)
      else if (freq === 'MONTHLY') d.setMonth(base.getMonth() + k * interval)
      else if (freq === 'YEARLY') d.setFullYear(base.getFullYear() + k * interval)
      else { push(s.t); return }
      if (d.getTime() > Math.min(until, to)) break
      if (freq === 'WEEKLY' && byday.length) {
        const weekStart = new Date(d); weekStart.setDate(d.getDate() - d.getDay())
        for (const wd of byday) {
          const x = new Date(weekStart); x.setDate(weekStart.getDate() + wd)
          if (x.getTime() < s.t || x.getTime() > until || n >= count) continue
          n++; push(x.getTime())
        }
      } else { n++; push(d.getTime()) }
    }
  }
}
