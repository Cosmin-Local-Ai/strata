// Finance (brief §4.6): read-only keys; daily revenue for 60 days → Today / Week / 30-day views with
// change vs the previous period; "-chart" variants show the weekly number and a filled 14-day line.
import { CircleDollarSign, Coins, CreditCard, Landmark, Receipt, ShoppingBag, ShoppingCart, Wallet } from '@lucide/svelte'
import { api, daily, daysAgo, isoDay, money, MONEY_TABS, NotConnected, revenueView, type Adapter, type View } from '../../../lib/services.svelte'

type Series = { days: number[]; currency: string; open?: string }
const since = () => daysAgo(59)
const pages = async <T = any>(first: () => Promise<{ items: T[]; next: (() => Promise<any>) | null }>, max = 20) => {
  const out: T[] = []
  let r = await first()
  for (let i = 0; i < max; i++) {
    out.push(...r.items)
    if (!r.next) break
    r = await r.next()
  }
  return out
}

async function stripe(): Promise<Series> {
  const gte = Math.floor(+since() / 1000)
  const get = (after?: string) => api<any>('stripe', `https://api.stripe.com/v1/balance_transactions?limit=100&created[gte]=${gte}${after ? `&starting_after=${after}` : ''}`)
    .then((b) => ({ items: b.data as any[], next: b.has_more ? () => get(b.data[b.data.length - 1].id) : null }))
  const tx = (await pages(() => get())).filter((t) => t.type === 'charge' || t.type === 'payment')
  const currency = (tx[0]?.currency ?? 'usd').toUpperCase()
  return { days: daily(tx.map((t) => [t.created * 1000, t.amount / 100]), 60), currency, open: 'https://dashboard.stripe.com/' }
}
async function polar(): Promise<Series> {
  const b = await api<any>('polar', `https://api.polar.sh/v1/metrics/?start_date=${isoDay(since())}&end_date=${isoDay(new Date())}&interval=day`)
  return { days: daily((b.periods ?? []).map((p: any) => [Date.parse(p.timestamp), (p.revenue ?? 0) / 100]), 60), currency: 'USD', open: 'https://polar.sh/dashboard' }
}
async function shopify(o: Record<string, any>): Promise<Series> {
  const shop = String(o.shop ?? '').replace(/^https?:\/\//, '').replace(/\.myshopify\.com.*$/, '').trim()
  if (!shop) throw new NotConnected()
  const b = await api<any>('shopify', `https://${shop}.myshopify.com/admin/api/2024-10/orders.json?status=any&limit=250&created_at_min=${since().toISOString()}&fields=created_at,total_price,currency,cancelled_at`)
  const orders = (b.orders ?? []).filter((x: any) => !x.cancelled_at)
  return { days: daily(orders.map((x: any) => [Date.parse(x.created_at), +x.total_price]), 60), currency: orders[0]?.currency ?? 'USD', open: `https://${shop}.myshopify.com/admin` }
}
async function lemon(): Promise<Series> {
  const cutoff = +since()
  const get = (n: number) => api<any>('lemonsqueezy', `https://api.lemonsqueezy.com/v1/orders?page[size]=100&page[number]=${n}&sort=-createdAt`, { headers: { Accept: 'application/vnd.api+json' } })
    .then((b) => { const items = b.data ?? []; const old = items.some((x: any) => Date.parse(x.attributes.created_at) < cutoff); return { items, next: !old && b.meta?.page?.lastPage > n ? () => get(n + 1) : null } })
  const orders = (await pages(() => get(1))).filter((x: any) => x.attributes.status === 'paid')
  return { days: daily(orders.map((x: any) => [Date.parse(x.attributes.created_at), x.attributes.total / 100]), 60), currency: orders[0]?.attributes.currency ?? 'USD', open: 'https://app.lemonsqueezy.com/dashboard' }
}
async function paddle(): Promise<Series> {
  const get = (url: string) => api<any>('paddle', url).then((b) => ({ items: b.data ?? [], next: b.meta?.pagination?.has_more ? () => get(b.meta.pagination.next) : null }))
  const tx = await pages(() => get(`https://api.paddle.com/transactions?status=completed&created_at[GTE]=${since().toISOString()}&per_page=200&order_by=created_at[DESC]`))
  return { days: daily(tx.map((t: any) => [Date.parse(t.created_at), +(t.details?.totals?.grand_total ?? 0) / 100]), 60), currency: tx[0]?.currency_code ?? 'USD', open: 'https://vendors.paddle.com/' }
}
async function gumroad(): Promise<Series> {
  const get = (url: string) => api<any>('gumroad', url).then((b) => ({ items: b.sales ?? [], next: b.next_page_url ? () => get(`https://api.gumroad.com${b.next_page_url}`) : null }))
  const sales = await pages(() => get(`https://api.gumroad.com/v2/sales?after=${isoDay(since())}`))
  return { days: daily(sales.map((s: any) => [Date.parse(s.created_at), (s.price ?? 0) / 100]), 60), currency: (sales[0]?.currency ?? 'usd').toUpperCase(), open: 'https://app.gumroad.com/dashboard' }
}
async function dodo(): Promise<Series> {
  const get = (n: number) => api<any>('dodo', `https://live.dodopayments.com/payments?created_at_gte=${since().toISOString()}&page_size=100&page_number=${n}`)
    .then((b) => ({ items: b.items ?? [], next: (b.items ?? []).length === 100 ? () => get(n + 1) : null }))
  const p = (await pages(() => get(0))).filter((x: any) => x.status === 'succeeded')
  return { days: daily(p.map((x: any) => [Date.parse(x.created_at), (x.total_amount ?? 0) / 100]), 60), currency: p[0]?.currency ?? 'USD', open: 'https://app.dodopayments.com/' }
}

const STORES: Record<string, { title: string; icon: any; fn: (o: Record<string, any>) => Promise<Series>; help: string; link: string; fields?: Adapter['fields']; keyLabel?: string }> = {
  stripe: { title: 'Stripe', icon: CreditCard, fn: stripe, help: 'Create a restricted key with read access to Balance (Developers → API keys → Create restricted key).', link: 'https://dashboard.stripe.com/apikeys', keyLabel: 'Restricted key (rk_live_…)' },
  polar: { title: 'Polar', icon: Coins, fn: polar, help: 'Create an organization access token with the metrics:read scope.', link: 'https://polar.sh/dashboard' },
  shopify: { title: 'Shopify', icon: ShoppingBag, fn: shopify, help: 'Create a custom app in your store (Settings → Apps → Develop apps) with read_orders, install it and paste the Admin API access token.', link: 'https://admin.shopify.com/', fields: [{ key: 'shop', label: 'Store name', placeholder: 'mystore (from mystore.myshopify.com)' }], keyLabel: 'Admin API access token' },
  lemonsqueezy: { title: 'Lemon Squeezy', icon: Receipt, fn: lemon, help: 'Create an API key under Settings → API.', link: 'https://app.lemonsqueezy.com/settings/api' },
  paddle: { title: 'Paddle', icon: Landmark, fn: paddle, help: 'Create an API key with transaction read permission (Developer tools → Authentication).', link: 'https://vendors.paddle.com/authentication-v2' },
  gumroad: { title: 'Gumroad', icon: ShoppingCart, fn: gumroad, help: 'Create an application (Settings → Advanced → Applications) and generate an access token.', link: 'https://app.gumroad.com/settings/advanced', keyLabel: 'Access token' },
  dodo: { title: 'Dodo Payments', icon: Wallet, fn: dodo, help: 'Create an API key in the Dodo Payments dashboard (Developer → API keys).', link: 'https://app.dodopayments.com/' },
}

export const FINANCE: Record<string, Adapter> = {}
for (const [id, s] of Object.entries(STORES)) {
  const a: Adapter = {
    title: s.title, icon: s.icon, secret: id, help: s.help, link: s.link, fields: s.fields, keyLabel: s.keyLabel,
    every: 15 * 60_000, tabs: MONEY_TABS, tileTab: 'month',
    async fetch(o, c) { const r = await s.fn(o); return revenueView(r.days, r.currency, c.tab, r.open) },
  }
  const wid = id === 'lemonsqueezy' ? 'lemon-squeezy' : id
  FINANCE[wid] = a
  FINANCE[`${wid}-chart`] = { ...a, title: `${s.title} Chart` }
}

// ---- Total Revenue: every connected store, converted to one currency; monthly goal and pace
FINANCE['total-revenue'] = {
  title: 'Total Revenue', icon: CircleDollarSign, every: 15 * 60_000, tabs: MONEY_TABS, tileTab: 'month',
  fields: [{ key: 'currency', label: 'Show in currency', placeholder: 'USD' }, { key: 'goal', label: 'Monthly goal (optional)', placeholder: '5000' }],
  async fetch(o, c) {
    const cur = String(o.currency || 'USD').toUpperCase()
    const res = await Promise.allSettled(Object.entries(STORES).map(async ([id, s]) => ({ id, r: await s.fn(o) })))
    const ok = res.filter((r): r is PromiseFulfilledResult<{ id: string; r: Series }> => r.status === 'fulfilled').map((r) => r.value)
    if (!ok.length) throw new Error('Connect at least one store widget (Stripe, Polar, Shopify, Lemon Squeezy, Paddle, Gumroad or Dodo) first.')
    const rates: Record<string, number> = (await api<any>('public', `https://api.frankfurter.dev/v1/latest?base=${cur}`).catch(() => ({ rates: {} }))).rates ?? {}
    const days = new Array(60).fill(0)
    for (const { r } of ok) {
      const k = r.currency.toUpperCase() === cur ? 1 : rates[r.currency.toUpperCase()] ? 1 / rates[r.currency.toUpperCase()] : 1
      r.days.forEach((v, i) => (days[i + (60 - r.days.length)] += v * k))
    }
    const v: View = revenueView(days, cur, c.tab)
    const now = new Date()
    const mStart = new Date(now.getFullYear(), now.getMonth(), 1)
    const inMonth = Math.floor((+now - +mStart) / 864e5) + 1
    const monthSoFar = days.slice(-inMonth).reduce((a, b) => a + b, 0)
    const dim = new Date(now.getFullYear(), now.getMonth() + 1, 0).getDate()
    const pace = (monthSoFar / inMonth) * dim
    const goal = +o.goal || 0
    if (goal) v.ring = Math.min(1, monthSoFar / goal)
    v.stats = [...(v.stats ?? []), { label: 'This month', value: money(monthSoFar, cur, false) }, { label: goal ? `Pace (goal ${money(goal, cur)})` : 'Month pace', value: money(pace, cur, false) }]
    v.rows = ok.map(({ id, r }) => ({ title: STORES[id].title, value: money(r.days.slice(-30).reduce((a, b) => a + b, 0), r.currency, false), sub: r.currency }))
    v.rowsTitle = 'Stores (30 days)'
    const missing = res.length - ok.length
    if (missing) v.note = `${missing} store widget(s) not connected.`
    return v
  },
}
