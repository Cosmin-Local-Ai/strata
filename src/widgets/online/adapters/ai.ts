// AI usage (brief §4.6 AI). Claude and ChatGPT/Codex read the sign-in that already exists on this PC
// (read-only) and talk only to that vendor. Readings are kept locally to chart use over the window.
import { Bot, BrainCircuit, Cpu, Gauge, Sparkles, Terminal, Wallet, Zap } from '@lucide/svelte'
import { invoke } from '@tauri-apps/api/core'
import { abbr, api, money, until, type Adapter, type Limit, type View } from '../../../lib/services.svelte'

// ---- local history of readings (for the "use over the window" chart and spend deltas)
function remember(key: string, value: number, keepMs: number): number[] {
  const k = `ai-hist:${key}`
  let h: [number, number][] = []
  try { h = JSON.parse(localStorage.getItem(k) ?? '[]') } catch { /* fresh */ }
  const now = Date.now()
  if (!h.length || now - h[h.length - 1][0] > 60_000) h.push([now, value])
  h = h.filter(([t]) => now - t < keepMs)
  localStorage.setItem(k, JSON.stringify(h))
  return h.map(([, v]) => v)
}
function history(key: string): [number, number][] {
  try { return JSON.parse(localStorage.getItem(`ai-hist:${key}`) ?? '[]') } catch { return [] }
}

const resetMs = (r: string | number | null | undefined) => r == null ? NaN : typeof r === 'number' ? (r < 1e12 ? r * 1000 : r) : Date.parse(r)
function view(limits: Limit[], histKey: string, extra: Partial<View> = {}): View {
  const main = limits[0]
  const series = main ? remember(histKey, main.pct, 8 * 864e5) : []
  // Window chart: 5-hour session (or weekly) from its start to its reset.
  const end = resetMs(main?.resets)
  const len = /week/i.test(main?.label ?? '') ? 7 * 864e5 : 5 * 3600_000
  const win = main && isFinite(end) ? { start: end - len, end, label: main.label, points: history(histKey).filter(([t]) => t >= end - len) as [number, number][] } : undefined
  return {
    value: main ? `${Math.round(main.pct)}%` : '–',
    label: main?.resets ? `resets in ${until(main.resets)}` : main?.label,
    ring: main ? main.pct / 100 : null,
    limits, series, seriesLabel: 'Use over the current window', window: win,
    ...extra,
  }
}

// ---- Claude (Claude Code sign-in)
async function claude() {
  const p = await api<any>('claude', 'https://api.anthropic.com/api/oauth/usage').catch((e) => {
    if (String(e.message).startsWith('expired:')) throw new Error(String(e.message).slice(9))
    throw e
  })
  const limits: Limit[] = []
  if (p.five_hour) limits.push({ label: 'Session (5 h)', pct: +p.five_hour.utilization, resets: p.five_hour.resets_at })
  if (p.seven_day) limits.push({ label: 'Weekly', pct: +p.seven_day.utilization, resets: p.seven_day.resets_at })
  for (const l of p.limits ?? []) {
    if (l?.kind === 'weekly_scoped' && l.scope?.model?.display_name && l.percent != null)
      limits.push({ label: `Weekly · ${l.scope.model.display_name}`, pct: +l.percent, resets: l.resets_at })
  }
  const extra = p.extra_usage ?? p.extra ?? null
  return { limits, extra }
}

const claudeUsage: Adapter = {
  title: 'Claude Usage', icon: Sparkles, every: 5 * 60_000,
  async fetch() {
    const { limits, extra } = await claude()
    const s = limits.find((l) => l.label.startsWith('Session')), w = limits.find((l) => l.label === 'Weekly')
    const models = limits.filter((l) => l.label.startsWith('Weekly · ')).sort((a, b) => b.pct - a.pct)
    const v = view(limits, 'claude', { open: 'https://claude.ai/settings/usage', note: 'Read from your Claude Code sign-in on this PC.' })
    // Tile: both windows at once, e.g. "67% · 80%" with "5 h 2h 10min · week 44 h".
    v.value = `${Math.round(s?.pct ?? 0)}% · ${Math.round(w?.pct ?? 0)}%`
    v.label = `5 h ${until(s?.resets) || '–'} · week ${until(w?.resets) || '–'}`
    v.ring = Math.max(s?.pct ?? 0, w?.pct ?? 0) / 100
    v.stats = [{ label: 'Most used model', value: models[0]?.label.slice(9) ?? '–' }, { label: 'Extra usage', value: extra ? (extra.is_enabled ? 'On' : 'Off') : '–' }]
    return v
  },
}
const claudePlan: Adapter = {
  title: 'Claude Usage (weekly)', icon: Sparkles, every: 15 * 60_000,
  async fetch() {
    const { limits, extra } = await claude()
    const weekly = limits.find((l) => l.label === 'Weekly')
    const models = limits.filter((l) => l.label.startsWith('Weekly · ')).sort((a, b) => b.pct - a.pct)
    const v = view(weekly ? [weekly, ...limits.filter((l) => l !== weekly)] : limits, 'claude-week')
    v.series = remember('claude-week', weekly?.pct ?? 0, 7 * 864e5)
    v.seriesLabel = 'Weekly use this week'
    v.stats = [
      { label: 'Session', value: `${Math.round(limits[0]?.pct ?? 0)}%` },
      { label: 'Weekly', value: `${Math.round(weekly?.pct ?? 0)}%` },
      { label: 'Most used model', value: models[0]?.label.slice(9) ?? '–' },
      { label: 'Extra usage', value: extra ? (extra.is_enabled ? 'On' : 'Off') : '–' },
    ]
    v.open = 'https://claude.ai/settings/usage'
    return v
  },
}

// ---- ChatGPT / Codex
const chatgptUsage: Adapter = {
  title: 'ChatGPT Usage', icon: Bot, every: 5 * 60_000,
  async fetch() {
    const p = await api<any>('codex', 'https://chatgpt.com/backend-api/wham/usage')
    const rl = p.rate_limit ?? p.rate_limits ?? {}
    const win = (w: any, label: string): Limit | null => w ? { label, pct: +(w.used_percent ?? 0), resets: w.reset_at ?? (w.reset_after_seconds ? Date.now() + w.reset_after_seconds * 1000 : null) } : null
    const limits = [win(rl.primary_window ?? rl.primary, 'Codex (5 h)'), win(rl.secondary_window ?? rl.secondary, 'Codex weekly')].filter(Boolean) as Limit[]
    return view(limits, 'codex', { stats: p.plan_type ? [{ label: 'Plan', value: String(p.plan_type) }] : undefined, open: 'https://chatgpt.com/codex/settings/usage', note: 'Read from your Codex sign-in on this PC.' })
  },
}

// ---- Not detected on this PC (honest states rather than invented numbers)
const notHere = (title: string, icon: any, msg: string): Adapter => ({ title, icon, every: 30 * 60_000, async fetch() { throw new Error(msg) } })
const geminiUsage = notHere('Gemini Usage', BrainCircuit, 'Antigravity isn\'t installed or signed in on this PC. Install it and sign in, then refresh.')
const grokUsage = notHere('Grok Usage', Zap, 'No Grok sign-in was found on this PC. SuperGrok credits are read from the Grok app\'s sign-in.')

// ---- Cursor: preview with sample data
const cursorUsage: Adapter = {
  title: 'Cursor Usage', icon: Terminal, every: 60 * 60_000, preview: true,
  async fetch() {
    return view([{ label: 'Included requests', pct: 42, resets: Date.now() + 12 * 864e5, detail: '210 of 500' }, { label: 'Usage-based', pct: 8, detail: '$4.10 of $50' }], 'cursor', { note: 'Preview with sample data.' })
  },
}

// ---- DeepSeek balance (pasted key); spend derived from balance changes we've seen
const deepseek: Adapter = {
  title: 'DeepSeek Balance', icon: Wallet, every: 15 * 60_000, secret: 'deepseek', link: 'https://platform.deepseek.com/api_keys',
  help: 'Create an API key on platform.deepseek.com → API keys, then paste it here.',
  async fetch() {
    const p = await api<any>('deepseek', 'https://api.deepseek.com/user/balance')
    const b = p.balance_infos?.[0] ?? {}
    const bal = +(b.total_balance ?? 0), cur = b.currency ?? 'USD'
    const h = history('deepseek')
    remember('deepseek', bal, 8 * 864e5)
    const spentSince = (ms: number) => { const old = h.find(([t]) => Date.now() - t < ms); return old ? Math.max(0, old[1] - bal) : 0 }
    const today0 = new Date(); today0.setHours(0, 0, 0, 0)
    return {
      value: money(bal, cur, false), label: 'balance',
      series: history('deepseek').map(([, v]) => v), seriesLabel: 'Balance this week',
      stats: [{ label: 'Spent today', value: money(spentSince(Date.now() - +today0), cur, false) }, { label: 'Spent this week', value: money(spentSince(7 * 864e5), cur, false) },
        { label: 'Granted', value: money(+(b.granted_balance ?? 0), cur, false) }, { label: 'Topped up', value: money(+(b.topped_up_balance ?? 0), cur, false) }],
      note: p.is_available === false ? 'Balance too low to make requests.' : undefined, open: 'https://platform.deepseek.com/usage',
    }
  },
}

// ---- GLM (Z.ai / Zhipu coding plan)
const glm: Adapter = {
  title: 'GLM Usage', icon: Gauge, every: 10 * 60_000, secret: 'glm', link: 'https://z.ai/manage-apikey/apikey-list',
  help: 'Paste the API key of your Z.ai (Zhipu) GLM Coding Plan.',
  async fetch() {
    const p = await api<any>('glm', 'https://api.z.ai/api/monitor/usage/quota/limit')
    const ls = p.data?.limits ?? p.limits ?? []
    const limits: Limit[] = ls.map((l: any) => ({ label: l.type === 'TIME_LIMIT' ? 'Tool calls (month)' : 'Prompts (5 h)', pct: +(l.percentage ?? 0), resets: l.nextResetTime ?? null, detail: l.currentValue != null && l.usage != null ? `${l.currentValue} of ${l.usage}` : undefined }))
    return view(limits, 'glm', { open: 'https://z.ai/manage-apikey/subscription' })
  },
}

// ---- OpenCode (local history)
type OC = { sessionTokens: number; sessionTitle: string; weekTokens: number; todayTokens: number; topModel: string; lastActive: number }
const opencodeUsage: Adapter = {
  title: 'OpenCode Usage', icon: Cpu, every: 60_000,
  async fetch() {
    const o = await invoke<OC>('opencode_usage')
    return { value: abbr(o.sessionTokens), label: 'tokens this session', stats: [{ label: 'Session', value: o.sessionTitle || '–' }, { label: 'Today', value: abbr(o.todayTokens) }], note: 'From OpenCode\'s local history.' }
  },
}
const opencodePlan: Adapter = {
  title: 'OpenCode Plan Usage', icon: Cpu, every: 5 * 60_000,
  async fetch() {
    const o = await invoke<OC>('opencode_usage')
    return { value: abbr(o.weekTokens), label: 'tokens this week', stats: [{ label: 'Session', value: abbr(o.sessionTokens) }, { label: 'Today', value: abbr(o.todayTokens) }, { label: 'Top model', value: o.topModel || '–' }] }
  },
}

// ---- AI Limits: two chosen plans on one tile
const PICK: Record<string, Adapter> = { claude: claudeUsage, chatgpt: chatgptUsage, glm, cursor: cursorUsage }
const aiLimits: Adapter = {
  title: 'AI Limits', icon: Sparkles, every: 5 * 60_000,
  fields: [{ key: 'a', label: 'First plan (claude, chatgpt, glm or cursor)', placeholder: 'claude' }, { key: 'b', label: 'Second plan', placeholder: 'chatgpt' }],
  async fetch(o) {
    const ids = [o.a || 'claude', o.b || 'chatgpt']
    const res = await Promise.allSettled(ids.map((id) => (PICK[id] ?? claudeUsage).fetch({}, {})))
    const limits: Limit[] = []
    res.forEach((r, i) => { if (r.status === 'fulfilled' && r.value.limits?.[0]) limits.push({ ...r.value.limits[0], label: `${(PICK[ids[i]] ?? claudeUsage).title.replace(' Usage', '')} · ${r.value.limits[0].label}` }) })
    const failed = res.map((r, i) => r.status === 'rejected' ? `${ids[i]}: ${(r.reason as Error).message}` : '').filter(Boolean)
    if (!limits.length) throw new Error(failed.join('\n') || 'No plan connected')
    const top = Math.max(...limits.map((l) => l.pct))
    return { value: limits.map((l) => `${Math.round(l.pct)}%`).join(' · '), label: limits.map((l) => l.label.split(' · ')[0]).join(' · '), ring: top / 100, limits, note: failed.join('\n') || undefined }
  },
}

export const AI: Record<string, Adapter> = {
  'ai-limits': aiLimits, 'claude-usage': claudeUsage, 'claude-plan': claudePlan, 'chatgpt-usage': chatgptUsage,
  'gemini-usage': geminiUsage, 'grok-usage': grokUsage, 'deepseek-balance': deepseek, 'glm-usage': glm,
  'opencode-usage': opencodeUsage, 'opencode-plan': opencodePlan, 'cursor-usage': cursorUsage,
}
