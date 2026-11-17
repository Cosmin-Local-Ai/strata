// Online / data widgets as *adapters*: an adapter fetches data and returns a View (what the tile and the
// card show). One Tile and one Card render every adapter, so a new service is a few lines of data code.
import type { Component } from 'svelte'
import { invoke } from '@tauri-apps/api/core'

export class NotConnected extends Error { constructor(m = 'not-connected') { super(m) } }

/** Call a service through Strata (which adds the credential and checks the host). */
export async function api<T = any>(service: string, url: string, opts: { method?: string; body?: unknown; headers?: Record<string, string> } = {}): Promise<T> {
  let r: { status: number; body: any }
  try {
    r = await invoke('api', { service, url, method: opts.method ?? 'GET', body: opts.body ?? null, headers: opts.headers ?? null })
  } catch (e) {
    const m = String(e)
    if (m.includes('not-connected')) throw new NotConnected()
    throw new Error(m.replace(/^Error: /, ''))
  }
  if (r.status === 401 || r.status === 403) throw new Error(`The key was rejected (${r.status}). Check it has read access.`)
  if (r.status === 429) throw new Error('Rate limited by the service; will retry.')
  if (r.status >= 400) {
    const b = r.body
    const msg = typeof b === 'string' ? b : b?.error?.message ?? b?.message ?? b?.error_description ?? (typeof b?.error === 'string' ? b.error : JSON.stringify(b).slice(0, 200))
    throw new Error(`${r.status}: ${msg}`)
  }
  return r.body as T
}
export const hasSecret = (name: string) => invoke<boolean>('secret_has', { name })
export const setSecret = (name: string, value: string) => invoke('secret_set', { name, value })
export const deleteSecret = (name: string) => invoke('secret_delete', { name })

export type Limit = { label: string; pct: number; resets?: string | number | null; detail?: string }
export type Row = { title: string; sub?: string; value?: string; href?: string; icon?: string; dim?: boolean; tone?: 'good' | 'bad' }
export type View = {
  /** Tile: main number/text and a short label. */
  value: string
  label?: string
  /** Percent change (e.g. vs the previous period), shown green/red. */
  delta?: number | null
  /** 0..1 for a ring on the tile (AI limits, goals). */
  ring?: number | null
  series?: number[]
  seriesLabel?: string
  /** false: scale the chart between min and max (prices). Default: from 0. */
  seriesFromZero?: boolean
  /** Use over a limit window: readings from its start to its reset, projected at the current pace. */
  window?: { points: [number, number][]; start: number; end: number; label: string }
  limits?: Limit[]
  stats?: { label: string; value: string }[]
  rows?: Row[]
  rowsTitle?: string
  grid?: number[]
  note?: string
  open?: string
  updated?: number
  /** The card shows a warning banner (e.g. expired sign-in) but keeps the last values. */
  warning?: string
}

export type Field = { key: string; label: string; placeholder?: string }
export type Adapter = {
  /** Secret name in Credential Manager (= service id), if a pasted key is needed. */
  secret?: string
  /** Custom sign-in: 'google' | 'yahoo' (handled by the card). */
  signIn?: 'google' | 'yahoo'
  title: string
  icon: Component<any>
  /** How to get the key, shown on the connect form. */
  help?: string
  link?: string
  keyLabel?: string
  /** Non-secret settings (store domain, username, property id …), kept in the widget's options. */
  fields?: Field[]
  /** Refresh interval in ms. */
  every: number
  /** Card tabs (e.g. Today/Week/30 days, Inbox/Unread). The tile uses `tileTab` or the first. */
  tabs?: { id: string; label: string }[]
  tileTab?: string
  search?: boolean
  /** Sample data adapters are labelled "Preview". */
  preview?: boolean
  fetch: (o: Record<string, any>, ctx: { tab?: string; query?: string }) => Promise<View>
}

/** Cached results shared between the dock and the card (localStorage is shared by both windows). */
type Cached = { at: number; view?: View; error?: string; connected: boolean }
const KEY = (id: string, opts: Record<string, any>, tab?: string, q?: string) => `svc:${id}:${JSON.stringify(opts)}:${tab ?? ''}:${q ?? ''}`
export function cachedView(id: string, opts: Record<string, any>, tab?: string): Cached | null {
  try { return JSON.parse(localStorage.getItem(KEY(id, opts, tab)) ?? 'null') } catch { return null }
}

/** Reactive state for one adapter: loads (cache first), refreshes on its interval while visible. */
export function useService(id: string, a: Adapter, opts: () => Record<string, any>, ctx: () => { tab?: string; query?: string } = () => ({})) {
  const st = $state<{ view: View | null; error: string; connected: boolean; loading: boolean }>({ view: null, error: '', connected: true, loading: false })
  let seq = 0
  async function load(force = false) {
    const o = opts(), c = ctx()
    const key = KEY(id, o, c.tab, c.query)
    const cached: Cached | null = (() => { try { return JSON.parse(localStorage.getItem(key) ?? 'null') } catch { return null } })()
    if (cached) { st.view = cached.view ?? st.view; st.error = cached.error ?? ''; st.connected = cached.connected }
    if (!force && cached && Date.now() - cached.at < a.every * 0.9) return
    const my = ++seq
    st.loading = true
    try {
      const v = await a.fetch(o, c)
      if (my !== seq) return
      v.updated = Date.now()
      st.view = v; st.error = ''; st.connected = true
      localStorage.setItem(key, JSON.stringify({ at: Date.now(), view: v, connected: true }))
    } catch (e) {
      if (my !== seq) return
      const nc = e instanceof NotConnected
      st.connected = !nc
      st.error = nc ? '' : String((e as Error).message ?? e)
      if (nc) st.view = null
      localStorage.setItem(key, JSON.stringify({ at: Date.now(), view: nc ? undefined : st.view ?? undefined, error: st.error, connected: !nc }))
    } finally {
      if (my === seq) st.loading = false
    }
  }
  $effect(() => {
    opts(); ctx()
    load()
    const t = setInterval(() => { if (document.visibilityState === 'visible') load() }, Math.max(15_000, a.every))
    const vis = () => { if (document.visibilityState === 'visible') load() }
    document.addEventListener('visibilitychange', vis)
    // The card connected/disconnected this widget: refresh now instead of at the next interval.
    const sig = (e: StorageEvent) => { if (e.key === `svc-signal:${id}`) load(true) }
    window.addEventListener('storage', sig)
    return () => { clearInterval(t); document.removeEventListener('visibilitychange', vis); window.removeEventListener('storage', sig) }
  })
  return { get state() { return st }, refresh: () => load(true) }
}

// ------------------------------------------------------------------ formatting
export function abbr(n: number, digits = 1): string {
  const a = Math.abs(n)
  if (a >= 1e9) return (n / 1e9).toFixed(digits).replace(/\.0$/, '') + 'B'
  if (a >= 1e6) return (n / 1e6).toFixed(digits).replace(/\.0$/, '') + 'M'
  if (a >= 1e3) return (n / 1e3).toFixed(a >= 1e4 ? 0 : digits).replace(/\.0$/, '') + 'k'
  return String(Math.round(n * 100) / 100)
}
const SYM: Record<string, string> = { USD: '$', EUR: '€', GBP: '£', JPY: '¥', RON: 'lei ', CAD: 'C$', AUD: 'A$', INR: '₹', BRL: 'R$', CHF: 'CHF ' }
export function money(n: number, cur = 'USD', short = true): string {
  const s = SYM[cur.toUpperCase()] ?? cur.toUpperCase() + ' '
  return (n < 0 ? '-' : '') + s + (short ? abbr(Math.abs(n)) : Math.abs(n).toLocaleString(undefined, { maximumFractionDigits: 2 }))
}
export function until(t: string | number | null | undefined): string {
  if (t == null) return ''
  const ms = (typeof t === 'number' ? (t < 1e12 ? t * 1000 : t) : Date.parse(t)) - Date.now()
  if (!isFinite(ms)) return ''
  if (ms <= 0) return 'now'
  const m = Math.round(ms / 60000)
  if (m < 60) return `${m} min`
  const h = Math.floor(m / 60)
  if (h < 48) return `${h} h ${m % 60} min`
  return `${Math.round(h / 24)} days`
}
export function ago(t: number | string): string {
  const ms = Date.now() - (typeof t === 'number' ? t : Date.parse(t))
  const m = Math.round(ms / 60000)
  if (m < 1) return 'just now'
  if (m < 60) return `${m} min ago`
  const h = Math.round(m / 60)
  if (h < 24) return `${h} h ago`
  return `${Math.round(h / 24)} d ago`
}
export const pctChange = (now: number, before: number) => (before ? ((now - before) / Math.abs(before)) * 100 : null)
export const isoDay = (d: Date) => `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
export const daysAgo = (n: number) => { const d = new Date(); d.setHours(0, 0, 0, 0); d.setDate(d.getDate() - n); return d }
/** Bucket (time ms, amount) pairs into `days` daily totals ending today. */
export function daily(points: [number, number][], days = 30): number[] {
  const start = +daysAgo(days - 1)
  const out = new Array(days).fill(0)
  for (const [t, v] of points) { const i = Math.floor((t - start) / 864e5); if (i >= 0 && i < days) out[i] += v }
  return out
}
const sum = (a: number[]) => a.reduce((x, y) => x + y, 0)
/** Finance views from 30 (or 60) daily totals: tabs today / week / 30 days with change vs previous period. */
export function revenueView(days60: number[], cur: string, tab: string | undefined, open?: string): View {
  const d = days60.length >= 60 ? days60 : [...new Array(60 - days60.length).fill(0), ...days60]
  const last = d.slice(-30), prev = d.slice(-60, -30)
  const today = d[d.length - 1], yesterday = d[d.length - 2]
  const week = sum(d.slice(-7)), pweek = sum(d.slice(-14, -7))
  const m = sum(last), pm = sum(prev)
  const t = tab ?? 'month'
  const value = t === 'today' ? today : t === 'week' ? week : m
  const delta = t === 'today' ? pctChange(today, yesterday) : t === 'week' ? pctChange(week, pweek) : pctChange(m, pm)
  return {
    value: money(value, cur), label: t === 'today' ? 'today' : t === 'week' ? '7 days' : '30 days', delta,
    series: t === 'week' || t === 'today' ? d.slice(-14) : last, seriesLabel: t === 'week' || t === 'today' ? 'Last 14 days' : 'Last 30 days',
    stats: [{ label: 'Today', value: money(today, cur, false) }, { label: '7 days', value: money(week, cur, false) }, { label: '30 days', value: money(m, cur, false) }, { label: 'Best day', value: money(Math.max(...last), cur, false) }],
    open,
  }
}
export const MONEY_TABS = [{ id: 'today', label: 'Today' }, { id: 'week', label: 'Week' }, { id: 'month', label: '30 days' }]
