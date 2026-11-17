// Shared system stats for the system tiles: one 2 s sampler per window, paused while hidden.
import { invoke } from '@tauri-apps/api/core'

export type Proc = { name: string; value: number }
export type Drive = { name: string; mount: string; total: number; free: number; removable: boolean }
export type Stats = {
  cpu: number; cores: number; ramUsed: number; ramTotal: number; topCpu: Proc[]; topRam: Proc[]
  netDown: number; netUp: number; drives: Drive[]
}

export const stats = $state<{ value: Stats | null; down: number[]; up: number[]; cpuHist: number[] }>({ value: null, down: [], up: [], cpuHist: [] })
const wants = { procs: 0, drives: 0, users: 0 }
let timer: ReturnType<typeof setTimeout> | undefined

async function tick() {
  clearTimeout(timer)
  if (wants.users === 0) return
  if (document.visibilityState === 'visible') {
    try {
      const s = await invoke<Stats>('sys_stats', { procs: wants.procs > 0, drives: wants.drives > 0 })
      stats.value = s
      stats.down = [...stats.down, s.netDown].slice(-60)
      stats.up = [...stats.up, s.netUp].slice(-60)
      stats.cpuHist = [...stats.cpuHist, s.cpu].slice(-60)
    } catch { /* keep last value */ }
  }
  timer = setTimeout(tick, 2000)
}
document.addEventListener('visibilitychange', () => { if (document.visibilityState === 'visible') tick() })

/** Subscribe from a component ($effect): returns the unsubscribe function. */
export function useStats(opts: { procs?: boolean; drives?: boolean } = {}) {
  wants.users++
  if (opts.procs) wants.procs++
  if (opts.drives) wants.drives++
  if (wants.users === 1 || opts.procs || opts.drives) tick()
  return () => {
    wants.users--
    if (opts.procs) wants.procs--
    if (opts.drives) wants.drives--
  }
}

export const gb = (b: number) => (b / 1024 ** 3).toFixed(b >= 100 * 1024 ** 3 ? 0 : 1) + ' GB'
export function rate(bps: number) {
  if (bps >= 1024 ** 2) return (bps / 1024 ** 2).toFixed(1) + ' MB/s'
  if (bps >= 1024) return (bps / 1024).toFixed(0) + ' KB/s'
  return bps.toFixed(0) + ' B/s'
}
/** SVG path for a sparkline of `v` in a w×h box. */
export function spark(v: number[], w: number, h: number, max?: number) {
  if (v.length < 2) return ''
  const m = Math.max(max ?? 0, ...v, 1)
  return v.map((x, i) => `${i ? 'L' : 'M'}${((i / (v.length - 1)) * w).toFixed(1)},${(h - (x / m) * h).toFixed(1)}`).join(' ')
}
