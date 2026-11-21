// The bundled CHANGELOG.md and a tiny renderer for its Markdown (headings, bullets, bold), used for
// "What's new" in Settings and for update notes. Text is escaped before any markup is added.
import changelog from '../../CHANGELOG.md?raw'

const esc = (s: string) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
const inline = (s: string) => esc(s).replace(/\*\*(.+?)\*\*/g, '<b>$1</b>')

export function renderNotes(md: string): string {
  const out: string[] = []
  let list = false
  for (const raw of md.replace(/\r/g, '').split('\n')) {
    const l = raw.trim()
    const bullet = /^[-*] /.test(l)
    if (list && !bullet) { out.push('</ul>'); list = false }
    if (!l) continue
    if (l.startsWith('### ')) out.push(`<h5>${inline(l.slice(4))}</h5>`)
    else if (l.startsWith('## ')) out.push(`<h4>${inline(l.slice(3))}</h4>`)
    else if (bullet) { if (!list) { out.push('<ul>'); list = true } out.push(`<li>${inline(l.slice(2))}</li>`) }
    else out.push(`<p>${inline(l)}</p>`)
  }
  if (list) out.push('</ul>')
  return out.join('')
}

/** The changelog section of `version` (without its heading), or ''. */
export function notesFor(version: string): string {
  const t = changelog.replace(/\r/g, '')
  const i = t.search(new RegExp(`^## ${version.replace(/\./g, '\\.')}\\b`, 'm'))
  if (i < 0) return ''
  const rest = t.slice(t.indexOf('\n', i) + 1)
  const j = rest.search(/^## /m)
  return (j < 0 ? rest : rest.slice(0, j)).trim()
}
