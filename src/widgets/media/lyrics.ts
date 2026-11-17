// Synced lyrics from LRCLIB (https://lrclib.net, free, no key). Opt-in (Now Playing "Lyrics mode"):
// only then is the current track's title/artist/album/duration sent to lrclib.net. Cached locally.

export type Line = { t: number; text: string }
export type Lyrics = { synced: Line[]; plain: string; instrumental: boolean }

const mem = new Map<string, Lyrics | null>()

function parseLrc(lrc: string): Line[] {
  const out: Line[] = []
  for (const raw of lrc.split('\n')) {
    const stamps = [...raw.matchAll(/\[(\d+):(\d+(?:\.\d+)?)\]/g)]
    const text = raw.replace(/\[[^\]]*\]/g, '').trim()
    for (const s of stamps) out.push({ t: +s[1] * 60 + +s[2], text })
  }
  return out.sort((a, b) => a.t - b.t)
}

export async function getLyrics(title: string, artist: string, album: string, duration: number): Promise<Lyrics | null> {
  const key = `${artist}|${title}|${Math.round(duration)}`.toLowerCase()
  if (mem.has(key)) return mem.get(key)!
  const stored = localStorage.getItem('lyrics:' + key)
  if (stored) {
    const v = JSON.parse(stored) as Lyrics | null
    mem.set(key, v)
    return v
  }
  const q = new URLSearchParams({ track_name: title, artist_name: artist })
  if (album) q.set('album_name', album)
  if (duration > 0) q.set('duration', String(Math.round(duration)))
  let j: any = null
  try {
    const r = await fetch(`https://lrclib.net/api/get?${q}`)
    if (r.ok) j = await r.json()
    else if (r.status === 404) {
      // Fuzzy fallback: search by title + artist and take the closest duration.
      const s = await fetch(`https://lrclib.net/api/search?${new URLSearchParams({ track_name: title, artist_name: artist })}`)
      const list = s.ok ? ((await s.json()) as any[]) : []
      j = list.sort((a, b) => Math.abs(a.duration - duration) - Math.abs(b.duration - duration))[0] ?? null
    }
  } catch {
    return null // offline: try again on the next track change, don't cache the failure
  }
  const v: Lyrics | null = j ? { synced: j.syncedLyrics ? parseLrc(j.syncedLyrics) : [], plain: j.plainLyrics ?? '', instrumental: !!j.instrumental } : null
  mem.set(key, v)
  localStorage.setItem('lyrics:' + key, JSON.stringify(v))
  return v
}

/** Index of the line being sung at `pos` seconds (-1 before the first line). */
export function lineAt(lines: Line[], pos: number): number {
  let lo = 0, hi = lines.length - 1, ans = -1
  while (lo <= hi) {
    const mid = (lo + hi) >> 1
    if (lines[mid].t <= pos + 0.15) { ans = mid; lo = mid + 1 } else hi = mid - 1
  }
  return ans
}
