// Shared polling with caching, backoff and idling (brief §7 performance rules):
// * one fetch per key shared by every tile/card that asks for it;
// * results cached in localStorage so tiles show the last value immediately after a restart;
// * pauses while the window is hidden (fullscreen/game mode, scrolled-out) and while offline;
// * exponential backoff on errors, capped, so failures never spam a service.

type Entry<T> = { at: number; value: T }
type Sub<T> = (v: T | null, err: string | null) => void

const subs = new Map<string, Set<Sub<any>>>()
const timers = new Map<string, ReturnType<typeof setTimeout>>()
const failures = new Map<string, number>()

export function cached<T>(key: string): Entry<T> | null {
  try { return JSON.parse(localStorage.getItem('poll:' + key) ?? 'null') } catch { return null }
}

export function poll<T>(key: string, intervalMs: number, fetcher: () => Promise<T>, onValue: Sub<T>): () => void {
  let set = subs.get(key)
  if (!set) subs.set(key, (set = new Set()))
  set.add(onValue)
  const c = cached<T>(key)
  if (c) onValue(c.value, null)

  const run = async () => {
    clearTimeout(timers.get(key))
    if (document.visibilityState !== 'visible' || !navigator.onLine) return // resumed by listeners below
    const age = Date.now() - (cached<T>(key)?.at ?? 0)
    if (age < intervalMs) {
      timers.set(key, setTimeout(run, intervalMs - age))
      return
    }
    try {
      const v = await fetcher()
      localStorage.setItem('poll:' + key, JSON.stringify({ at: Date.now(), value: v }))
      failures.set(key, 0)
      subs.get(key)?.forEach((s) => s(v, null))
      timers.set(key, setTimeout(run, intervalMs))
    } catch (e) {
      const n = (failures.get(key) ?? 0) + 1
      failures.set(key, n)
      subs.get(key)?.forEach((s) => s(cached<T>(key)?.value ?? null, String(e)))
      timers.set(key, setTimeout(run, Math.min(intervalMs, 15_000 * 2 ** n)))
    }
  }
  const wake = () => run()
  document.addEventListener('visibilitychange', wake)
  window.addEventListener('online', wake)
  if (set.size === 1) run()

  return () => {
    set!.delete(onValue)
    document.removeEventListener('visibilitychange', wake)
    window.removeEventListener('online', wake)
    if (set!.size === 0) clearTimeout(timers.get(key))
  }
}
