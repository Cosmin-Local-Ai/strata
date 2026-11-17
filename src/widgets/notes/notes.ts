export type Note = { id: string; title: string; html?: string; text?: string; updated: number | string | null }
export type NotesFile = { notes: Note[] }

/** Older notes may be stored as HTML; Strata edits plain text. Convert once, on first edit. */
export function noteText(n: Note): string {
  if (n.text !== undefined) return n.text
  const div = document.createElement('div')
  // Rich text with a <head>/<style> block: keep only the body text.
  const html = (n.html ?? '').replace(/<head[\s\S]*?<\/head>/gi, '').replace(/<style[\s\S]*?<\/style>/gi, '')
  div.innerHTML = html.replace(/<br\s*\/?>/gi, '\n').replace(/<\/p>/gi, '\n')
  return (div.textContent ?? '').trim()
}
export const ts = (n: Note) => (typeof n.updated === 'number' ? n.updated : Date.parse(String(n.updated ?? '')) || 0)
