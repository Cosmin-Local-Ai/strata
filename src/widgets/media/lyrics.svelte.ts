import { media } from '../../lib/stores.svelte'
import { getLyrics, lineAt, type Lyrics } from './lyrics'

/** Reactive lyrics for the current track. Wakes up only when the sung line changes (one timer per
 *  line), never on a fixed tick. Call inside a component; returns a reactive box. */
export function useLyrics(enabled: () => boolean) {
  const box = $state<{ lyrics: Lyrics | null; index: number; loading: boolean }>({ lyrics: null, index: -1, loading: false })

  $effect(() => {
    const m = media.value
    if (!enabled() || !m?.active || !m.title) { box.lyrics = null; return }
    let cancelled = false
    box.loading = true
    getLyrics(m.title, m.artist, m.album, m.duration).then((l) => { if (!cancelled) { box.lyrics = l; box.loading = false } })
    return () => { cancelled = true }
  })

  $effect(() => {
    const m = media.value
    const lines = box.lyrics?.synced ?? []
    if (!m || !lines.length) { box.index = -1; return }
    let timer: ReturnType<typeof setTimeout> | undefined
    const step = () => {
      const pos = m.position + (m.playing ? (Date.now() - m.sampledAt) / 1000 : 0)
      box.index = lineAt(lines, pos)
      const next = lines[box.index + 1]
      if (m.playing && next && document.visibilityState === 'visible') timer = setTimeout(step, Math.max(50, (next.t - pos) * 1000))
    }
    step()
    const vis = () => { clearTimeout(timer); step() }
    document.addEventListener('visibilitychange', vis)
    return () => { clearTimeout(timer); document.removeEventListener('visibilitychange', vis) }
  })

  return box
}
