// Timer / Pomodoro / Stopwatch state, shared by the dock tile and its card through localStorage (same
// origin in every Strata window; 'storage' events keep them in sync). Times are stored as timestamps,
// so they stay right across sleep and restarts.

export type TimerState = { duration: number; endsAt: number | null; remaining: number; label?: string }
export type PomoState = { phase: 'work' | 'break' | 'long'; round: number; endsAt: number | null; remaining: number }
export type SwState = { startedAt: number | null; before: number; laps: number[] }

const boxes = new Map<string, { value: any }>()

export function persisted<T>(key: string, init: T): { value: T } {
  let b = boxes.get(key)
  if (b) return b as { value: T }
  const read = () => { try { return JSON.parse(localStorage.getItem('st:' + key) ?? 'null') ?? init } catch { return init } }
  const box = $state({ value: read() as T })
  window.addEventListener('storage', (e) => { if (e.key === 'st:' + key) box.value = read() })
  b = {
    get value() { return box.value },
    set value(v: T) { box.value = v; localStorage.setItem('st:' + key, JSON.stringify(v)) },
  }
  boxes.set(key, b)
  return b as { value: T }
}

export const fmt = (ms: number, showHours = false) => {
  const s = Math.max(0, Math.round(ms / 1000))
  const h = Math.floor(s / 3600), m = Math.floor((s % 3600) / 60), x = s % 60
  return (h || showHours ? `${h}:${String(m).padStart(2, '0')}` : `${m}`) + `:${String(x).padStart(2, '0')}`
}

/** Short two-tone chime (WebAudio, no sound files). */
export function chime() {
  try {
    const ctx = new AudioContext()
    ;[880, 1320].forEach((f, i) => {
      const o = ctx.createOscillator(), g = ctx.createGain()
      o.frequency.value = f
      o.connect(g); g.connect(ctx.destination)
      const t = ctx.currentTime + i * 0.18
      g.gain.setValueAtTime(0.0001, t); g.gain.exponentialRampToValueAtTime(0.25, t + 0.02); g.gain.exponentialRampToValueAtTime(0.0001, t + 0.5)
      o.start(t); o.stop(t + 0.55)
    })
    setTimeout(() => ctx.close(), 1500)
  } catch { /* no audio */ }
}

export const POMO = { work: 25 * 60_000, break: 5 * 60_000, long: 15 * 60_000 }
export function pomoNext(p: PomoState): PomoState {
  if (p.phase === 'work') {
    const long = (p.round + 1) % 4 === 0
    return { phase: long ? 'long' : 'break', round: p.round + 1, endsAt: Date.now() + POMO[long ? 'long' : 'break'], remaining: POMO[long ? 'long' : 'break'] }
  }
  return { phase: 'work', round: p.round, endsAt: Date.now() + POMO.work, remaining: POMO.work }
}
