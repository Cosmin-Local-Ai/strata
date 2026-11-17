// Social (brief §4.6) and Markets stocks.
import { AtSign, Camera, ChartCandlestick, CirclePlay, List, Music } from '@lucide/svelte'
import { abbr, ago, api, hasSecret, NotConnected, pctChange, type Adapter } from '../../../lib/services.svelte'

const x: Adapter = {
  title: 'X Profile', icon: AtSign, secret: 'x', every: 60 * 60_000, link: 'https://developer.x.com/en/portal/dashboard',
  help: 'Create a project and app in the X developer portal and copy its Bearer token.', keyLabel: 'Bearer token', fields: [{ key: 'username', label: 'Username', placeholder: 'without @' }],
  async fetch(o) {
    if (!o.username) throw new NotConnected()
    const r = await api<any>('x', `https://api.x.com/2/users/by/username/${encodeURIComponent(String(o.username).replace(/^@/, ''))}?user.fields=public_metrics,profile_image_url,name`)
    const m = r.data.public_metrics
    const prev = +(localStorage.getItem(`x:${o.username}`) ?? 0)
    localStorage.setItem(`x:${o.username}`, String(m.followers_count))
    return { value: abbr(m.followers_count), label: 'followers', delta: prev ? pctChange(m.followers_count, prev) : null, open: `https://x.com/${o.username}`,
      stats: [{ label: 'Following', value: abbr(m.following_count) }, { label: 'Posts', value: abbr(m.tweet_count) }, { label: 'Listed', value: abbr(m.listed_count) }, { label: 'Likes', value: abbr(m.like_count ?? 0) }], note: r.data.name }
  },
}

const youtube: Adapter = {
  title: 'YouTube', icon: CirclePlay, secret: 'youtube', every: 30 * 60_000, link: 'https://console.cloud.google.com/apis/library/youtube.googleapis.com',
  help: 'Enable "YouTube Data API v3" in a Google Cloud project and create an API key (Credentials → Create credentials → API key).', fields: [{ key: 'channel', label: 'Channel handle or ID', placeholder: '@yourchannel or UC…' }],
  async fetch(o) {
    if (!o.channel) throw new NotConnected()
    const ch = String(o.channel).trim()
    const by = ch.startsWith('UC') ? `id=${ch}` : `forHandle=${encodeURIComponent(ch.startsWith('@') ? ch : '@' + ch)}`
    const r = await api<any>('youtube', `https://www.googleapis.com/youtube/v3/channels?part=statistics,snippet,contentDetails&${by}`)
    const c = r.items?.[0]
    if (!c) throw new Error('Channel not found')
    const up = c.contentDetails.relatedPlaylists.uploads
    const v = await api<any>('youtube', `https://www.googleapis.com/youtube/v3/playlistItems?part=snippet&maxResults=6&playlistId=${up}`).catch(() => ({ items: [] }))
    const s = c.statistics
    return { value: abbr(+s.subscriberCount), label: 'subscribers', open: `https://studio.youtube.com/channel/${c.id}`, note: c.snippet.title,
      stats: [{ label: 'Views', value: abbr(+s.viewCount) }, { label: 'Videos', value: abbr(+s.videoCount) }],
      rows: v.items.map((i: any) => ({ title: i.snippet.title, sub: ago(i.snippet.publishedAt), icon: i.snippet.thumbnails?.default?.url, href: `https://youtu.be/${i.snippet.resourceId.videoId}` })), rowsTitle: 'Latest videos' }
  },
}

const instagram: Adapter = {
  title: 'Instagram', icon: Camera, secret: 'instagram', every: 60 * 60_000, link: 'https://developers.facebook.com/docs/instagram-platform/instagram-api-with-instagram-login',
  help: 'Use the official Instagram API with Instagram Login: create a Meta app, add Instagram, and generate an access token for your professional account.', keyLabel: 'Access token',
  async fetch() {
    const me = await api<any>('instagram', 'https://graph.instagram.com/me?fields=username,followers_count,follows_count,media_count')
    const media = await api<any>('instagram', 'https://graph.instagram.com/me/media?fields=caption,like_count,comments_count,permalink,timestamp,thumbnail_url,media_url,media_type&limit=6').catch(() => ({ data: [] }))
    return { value: abbr(me.followers_count ?? 0), label: 'followers', open: `https://instagram.com/${me.username}`, note: `@${me.username}`,
      stats: [{ label: 'Following', value: abbr(me.follows_count ?? 0) }, { label: 'Posts', value: abbr(me.media_count ?? 0) }],
      rows: media.data.map((m: any) => ({ title: (m.caption ?? '').split('\n')[0] || 'Post', sub: `♥ ${abbr(m.like_count ?? 0)} · 💬 ${abbr(m.comments_count ?? 0)} · ${ago(m.timestamp)}`, icon: m.media_type === 'VIDEO' ? m.thumbnail_url : m.media_url, href: m.permalink })), rowsTitle: 'Recent posts' }
  },
}

const tiktok: Adapter = {
  title: 'TikTok', icon: Music, secret: 'tiktok', every: 60 * 60_000, link: 'https://developers.tiktok.com/',
  help: 'Create a TikTok developer app with Login Kit and the user.info.stats scope, authorise your account and paste the user access token.', keyLabel: 'User access token',
  async fetch() {
    const r = await api<any>('tiktok', 'https://open.tiktokapis.com/v2/user/info/?fields=display_name,follower_count,following_count,likes_count,video_count')
    if (r.error?.code && r.error.code !== 'ok') throw new Error(r.error.message)
    const u = r.data.user
    return { value: abbr(u.follower_count), label: 'followers', note: u.display_name, open: 'https://www.tiktok.com/',
      stats: [{ label: 'Likes', value: abbr(u.likes_count) }, { label: 'Videos', value: abbr(u.video_count) }, { label: 'Following', value: abbr(u.following_count) }] }
  },
}

// ---------------------------------------------------------------- stocks
// Intraday chart and fallback quotes come from Yahoo Finance's public chart endpoint (no key);
// with a Finnhub key the watchlist uses Finnhub quotes (decisions D-021).
async function yahooChart(sym: string, range = '1d', interval = '5m') {
  const r = await api<any>('public', `https://query1.finance.yahoo.com/v8/finance/chart/${encodeURIComponent(sym)}?range=${range}&interval=${interval}`)
  const c = r.chart?.result?.[0]
  if (!c) throw new Error(r.chart?.error?.description ?? `No data for ${sym}`)
  const closes: number[] = (c.indicators.quote[0].close ?? []).filter((x: number | null) => x != null)
  const price = c.meta.regularMarketPrice ?? closes[closes.length - 1]
  const prev = c.meta.chartPreviousClose ?? c.meta.previousClose
  return { price, prev, closes, currency: c.meta.currency ?? 'USD', name: c.meta.shortName ?? sym }
}
const fmtPrice = (n: number) => n >= 1000 ? n.toLocaleString(undefined, { maximumFractionDigits: 0 }) : n.toFixed(2)

const stockChart: Adapter = {
  title: 'Stock Chart', icon: ChartCandlestick, every: 60_000, fields: [{ key: 'symbol', label: 'Ticker', placeholder: 'AAPL' }], tileTab: 'week',
  async fetch(o) {
    const sym = String(o.symbol || 'AAPL').toUpperCase()
    const q = await yahooChart(sym)
    return { value: `${sym} ${fmtPrice(q.price)}`, label: q.currency, delta: pctChange(q.price, q.prev), series: q.closes, seriesFromZero: false, seriesLabel: 'Today, 5-minute steps', open: `https://finance.yahoo.com/quote/${sym}`,
      stats: [{ label: 'Previous close', value: fmtPrice(q.prev) }, { label: 'High', value: fmtPrice(Math.max(...q.closes)) }, { label: 'Low', value: fmtPrice(Math.min(...q.closes)) }], note: q.name }
  },
}

const watchlist: Adapter = {
  title: 'Stock Watchlist', icon: List, every: 60_000, secret: 'finnhub', keyLabel: 'Finnhub key (optional)', link: 'https://finnhub.io/dashboard', help: 'Works without a key (Yahoo Finance quotes). Optionally paste a free Finnhub API key to use Finnhub quotes.', fields: [{ key: 'symbols', label: 'Three tickers', placeholder: 'AAPL, MSFT, NVDA' }],
  async fetch(o) {
    const syms = String(o.symbols || 'AAPL, MSFT, NVDA').split(/[\s,]+/).filter(Boolean).slice(0, 3).map((s) => s.toUpperCase())
    const finn = await hasSecret('finnhub').catch(() => false)
    const quotes = await Promise.all(syms.map(async (s) => {
      if (finn) { const q = await api<any>('finnhub', `https://finnhub.io/api/v1/quote?symbol=${s}`); return { s, price: q.c, prev: q.pc } }
      const q = await yahooChart(s, '1d', '1d'); return { s, price: q.price, prev: q.prev }
    }))
    return {
      value: quotes.map((q) => `${q.s} ${fmtPrice(q.price)}`).join('  '), label: quotes.map((q) => `${(pctChange(q.price, q.prev) ?? 0) >= 0 ? '+' : ''}${(pctChange(q.price, q.prev) ?? 0).toFixed(1)}%`).join('  '),
      rows: quotes.map((q) => { const d = pctChange(q.price, q.prev) ?? 0; return { title: q.s, value: `${fmtPrice(q.price)}  ${d >= 0 ? '+' : ''}${d.toFixed(2)}%`, tone: d >= 0 ? 'good' as const : 'bad' as const, href: `https://finance.yahoo.com/quote/${q.s}` } }),
      note: finn ? 'Quotes from Finnhub.' : 'Quotes from Yahoo Finance. Add a Finnhub key in settings (gear) for Finnhub quotes.',
    }
  },
}

export const SOCIAL: Record<string, Adapter> = { 'x-profile': x, youtube, instagram, tiktok }
export const MARKETS: Record<string, Adapter> = { 'stock-chart': stockChart, 'stock-watchlist': watchlist }
