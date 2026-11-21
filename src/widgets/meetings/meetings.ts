// Meeting notes: one record per meeting, kept in %APPDATA%\Strata\meetings.json.
export type Action = { id: string; text: string; owner: string; done: boolean }
export type Meeting = {
  id: string
  title: string
  /** Start (ms). */
  when: number
  attendees: string
  agenda: string
  notes: string
  actions: Action[]
  /** Running timer: started at (ms) or 0; elapsed ms before that. */
  timerStart: number
  elapsed: number
  updated: number
}
export type MeetingsFile = { list: Meeting[] }

export const uid = (p: string) => p + Date.now().toString(36) + Math.random().toString(36).slice(2, 6)
export const blank = (): Meeting => ({ id: uid('m'), title: '', when: Date.now(), attendees: '', agenda: '', notes: '', actions: [], timerStart: 0, elapsed: 0, updated: Date.now() })
export const elapsedOf = (m: Meeting, now = Date.now()) => m.elapsed + (m.timerStart ? now - m.timerStart : 0)
export const mmss = (ms: number) => { const s = Math.floor(ms / 1000); const h = Math.floor(s / 3600); return `${h ? h + ':' : ''}${String(Math.floor((s % 3600) / 60)).padStart(h ? 2 : 1, '0')}:${String(s % 60).padStart(2, '0')}` }

export function toMarkdown(m: Meeting): string {
  const d = new Date(m.when)
  const lines = [`# ${m.title || 'Meeting'}`, '', `**When:** ${d.toLocaleDateString(undefined, { weekday: 'long', day: 'numeric', month: 'long', year: 'numeric' })}, ${d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })}${m.elapsed || m.timerStart ? ` (${mmss(elapsedOf(m))})` : ''}`]
  if (m.attendees.trim()) lines.push(`**Attendees:** ${m.attendees.trim()}`)
  if (m.agenda.trim()) lines.push('', '## Agenda', '', ...m.agenda.trim().split('\n').map((l) => (/^\s*[-*]/.test(l) ? l : `- ${l}`)))
  if (m.notes.trim()) lines.push('', '## Notes', '', m.notes.trim())
  if (m.actions.length) lines.push('', '## Action items', '', ...m.actions.map((a) => `- [${a.done ? 'x' : ' '}] ${a.text}${a.owner ? ` (${a.owner})` : ''}`))
  return lines.join('\n') + '\n'
}
