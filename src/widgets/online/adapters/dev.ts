// Analytics and developer widgets (brief §4.6) + Google Analytics, Search Console, Gmail (one shared
// Google sign-in) + Meta Ads + Yahoo Mail (brief §7).
import { Activity, ChartColumn, ChartLine, Code, Database, GitBranch, GitPullRequest, Inbox, Link2, Mail, Mails, Megaphone, SearchCheck, Send, Target } from '@lucide/svelte'
import { invoke } from '@tauri-apps/api/core'
import { abbr, ago, api, daily, daysAgo, isoDay, money, MONEY_TABS, NotConnected, pctChange, type Adapter, type Row, type View } from '../../../lib/services.svelte'

const gh = (path: string) => api<any>('github', `https://api.github.com${path}`, { headers: { Accept: 'application/vnd.github+json', 'X-GitHub-Api-Version': '2022-11-28' } })
const ghql = (query: string) => api<any>('github', 'https://api.github.com/graphql', { method: 'POST', body: { query } }).then((r) => { if (r.errors) throw new Error(r.errors[0].message); return r.data })
const ghHelp = 'Create a fine-grained or classic token (classic: scopes notifications and read:user, plus repo for private repos).'
const ghLink = 'https://github.com/settings/tokens'
const htmlUrl = (api: string | null | undefined) => api ? api.replace('https://api.github.com/repos/', 'https://github.com/').replace('/pulls/', '/pull/').replace('/commits/', '/commit/') : 'https://github.com/notifications'

async function notifications() {
  const n = await gh('/notifications?per_page=50')
  return n as any[]
}

const github: Adapter = {
  title: 'GitHub', icon: GitBranch, secret: 'github', help: ghHelp, link: ghLink, keyLabel: 'Personal access token', every: 3 * 60_000,
  async fetch() {
    const n = await notifications()
    return {
      value: abbr(n.length), label: 'unread notifications', open: 'https://github.com/notifications',
      rows: n.slice(0, 12).map((x) => ({ title: x.subject?.title, sub: `${x.repository?.full_name} · ${x.reason.replace(/_/g, ' ')} · ${ago(x.updated_at)}`, href: htmlUrl(x.subject?.url) })),
      rowsTitle: 'Notifications',
    }
  },
}

const githubActivity: Adapter = {
  title: 'GitHub Activity', icon: GitBranch, secret: 'github', help: ghHelp, link: ghLink, keyLabel: 'Personal access token', every: 30 * 60_000,
  async fetch() {
    const d = await ghql('{ viewer { login contributionsCollection { contributionCalendar { totalContributions weeks { contributionDays { contributionCount date } } } } } }')
    const cal = d.viewer.contributionsCollection.contributionCalendar
    const days: number[] = cal.weeks.flatMap((w: any) => w.contributionDays.map((x: any) => x.contributionCount))
    let streak = 0
    for (let i = days.length - 1; i >= 0; i--) { if (days[i] > 0) streak++; else if (i === days.length - 1) continue; else break }
    let longest = 0, run = 0
    for (const c of days) { run = c > 0 ? run + 1 : 0; longest = Math.max(longest, run) }
    return {
      value: `${streak} d`, label: 'streak', grid: days, open: `https://github.com/${d.viewer.login}`,
      stats: [{ label: 'This year', value: abbr(cal.totalContributions) }, { label: 'Current streak', value: `${streak} days` }, { label: 'Longest streak', value: `${longest} days` }, { label: 'Today', value: String(days[days.length - 1] ?? 0) }],
    }
  },
}

const githubOverview: Adapter = {
  title: 'GitHub Overview', icon: GitPullRequest, secret: 'github', help: ghHelp, link: ghLink, keyLabel: 'Personal access token', every: 10 * 60_000,
  async fetch() {
    const [d, n] = await Promise.all([
      ghql('{ viewer { login pullRequests(states: OPEN, first: 10, orderBy: {field: UPDATED_AT, direction: DESC}) { totalCount nodes { title url repository { nameWithOwner } updatedAt } } repositories(ownerAffiliations: OWNER, first: 100) { nodes { stargazerCount } } followers { totalCount } } }'),
      notifications().catch(() => []),
    ])
    const v = d.viewer
    const stars = v.repositories.nodes.reduce((a: number, r: any) => a + r.stargazerCount, 0)
    return {
      value: `${n.length} · ${v.pullRequests.totalCount} PRs`, label: `★ ${abbr(stars)}`, open: `https://github.com/${v.login}`,
      stats: [{ label: 'Notifications', value: String(n.length) }, { label: 'Open PRs', value: String(v.pullRequests.totalCount) }, { label: 'Stars', value: abbr(stars) }, { label: 'Followers', value: abbr(v.followers.totalCount) }],
      rows: v.pullRequests.nodes.map((p: any) => ({ title: p.title, sub: `${p.repository.nameWithOwner} · ${ago(p.updatedAt)}`, href: p.url })), rowsTitle: 'Your open pull requests',
    }
  },
}

const linear: Adapter = {
  title: 'Linear', icon: Target, secret: 'linear', every: 5 * 60_000, link: 'https://linear.app/settings/account/security',
  help: 'Create a personal API key (Settings → Security & access → Personal API keys).', keyLabel: 'Personal API key',
  async fetch() {
    const r = await api<any>('linear', 'https://api.linear.app/graphql', { method: 'POST', body: { query: '{ viewer { assignedIssues(first: 25, filter: { state: { type: { nin: ["completed", "canceled"] } } }, orderBy: updatedAt) { nodes { identifier title url priority state { name type } } } } }' } })
    if (r.errors) throw new Error(r.errors[0].message)
    const issues = r.data.viewer.assignedIssues.nodes
    const started = issues.filter((i: any) => i.state.type === 'started').length
    return {
      value: String(issues.length), label: 'open issues', open: 'https://linear.app/',
      stats: [{ label: 'In progress', value: String(started) }, { label: 'Urgent', value: String(issues.filter((i: any) => i.priority === 1).length) }],
      rows: issues.slice(0, 12).map((i: any) => ({ title: `${i.identifier} ${i.title}`, sub: i.state.name, href: i.url })), rowsTitle: 'Assigned to you',
    }
  },
}

const supabase: Adapter = {
  title: 'Supabase', icon: Database, secret: 'supabase', every: 10 * 60_000, link: 'https://supabase.com/dashboard/account/tokens',
  help: 'Create an access token (Account → Access tokens).', keyLabel: 'Access token',
  async fetch() {
    const p = await api<any[]>('supabase', 'https://api.supabase.com/v1/projects')
    const healthy = p.filter((x) => x.status === 'ACTIVE_HEALTHY').length
    return {
      value: `${healthy}/${p.length}`, label: 'projects healthy', open: 'https://supabase.com/dashboard/projects',
      rows: p.map((x) => ({ title: x.name, sub: `${x.region} · ${x.status.replace(/_/g, ' ').toLowerCase()}`, href: `https://supabase.com/dashboard/project/${x.id}`, tone: x.status === 'ACTIVE_HEALTHY' ? 'good' as const : 'bad' as const, value: x.status === 'ACTIVE_HEALTHY' ? '●' : '●' })),
    }
  },
}

const resend: Adapter = {
  title: 'Resend', icon: Send, secret: 'resend', every: 10 * 60_000, link: 'https://resend.com/api-keys', help: 'Create an API key (full access is needed to list emails).',
  async fetch() {
    const r = await api<any>('resend', 'https://api.resend.com/emails?limit=100')
    const list: any[] = r.data ?? []
    const day = list.filter((e) => Date.now() - Date.parse(e.created_at) < 864e5)
    const bounced = list.filter((e) => /bounce|complain/.test(e.last_event ?? '')).length
    return {
      value: String(day.length), label: 'emails today', open: 'https://resend.com/emails',
      stats: [{ label: 'Last 100: delivered', value: String(list.filter((e) => e.last_event === 'delivered').length) }, { label: 'Bounced / complaints', value: String(bounced) }],
      rows: list.slice(0, 10).map((e) => ({ title: e.subject, sub: `${(e.to ?? []).join(', ')} · ${ago(e.created_at)}`, value: e.last_event, tone: /bounce|complain/.test(e.last_event ?? '') ? 'bad' as const : undefined })),
    }
  },
}

const dub: Adapter = {
  title: 'Dub', icon: Link2, secret: 'dub', every: 15 * 60_000, link: 'https://app.dub.co/settings/tokens', help: 'Create an API key in your workspace settings.',
  async fetch() {
    const [ts, top] = await Promise.all([
      api<any[]>('dub', 'https://api.dub.co/analytics?event=clicks&interval=30d&groupBy=timeseries'),
      api<any[]>('dub', 'https://api.dub.co/analytics?event=clicks&interval=30d&groupBy=top_links').catch(() => []),
    ])
    const s = ts.map((x) => x.clicks ?? 0)
    const total = s.reduce((a, b) => a + b, 0)
    return { value: abbr(total), label: 'clicks (30 days)', series: s, seriesLabel: 'Clicks per day', delta: pctChange(s.slice(-7).reduce((a, b) => a + b, 0), s.slice(-14, -7).reduce((a, b) => a + b, 0)), open: 'https://app.dub.co/',
      rows: top.slice(0, 8).map((l: any) => ({ title: l.shortLink ?? l.url ?? l.link, value: abbr(l.clicks) })), rowsTitle: 'Top links' }
  },
}

const plausible: Adapter = {
  title: 'Plausible', icon: ChartLine, secret: 'plausible', every: 10 * 60_000, link: 'https://plausible.io/settings/api-keys',
  help: 'Create a Stats API key (Account settings → API keys).', fields: [{ key: 'site', label: 'Site domain', placeholder: 'example.com' }],
  async fetch(o) {
    if (!o.site) throw new NotConnected()
    const site = encodeURIComponent(o.site)
    const [agg, ts] = await Promise.all([
      api<any>('plausible', `https://plausible.io/api/v1/stats/aggregate?site_id=${site}&period=30d&metrics=visitors,pageviews,bounce_rate,visit_duration&compare=previous_period`),
      api<any>('plausible', `https://plausible.io/api/v1/stats/timeseries?site_id=${site}&period=30d`),
    ])
    const r = agg.results
    return {
      value: abbr(r.visitors.value), label: 'visitors (30 days)', delta: r.visitors.change ?? null, series: ts.results.map((x: any) => x.visitors), seriesLabel: 'Visitors per day',
      stats: [{ label: 'Pageviews', value: abbr(r.pageviews.value) }, { label: 'Bounce rate', value: `${r.bounce_rate.value}%` }, { label: 'Visit duration', value: `${Math.round(r.visit_duration.value)} s` }],
      open: `https://plausible.io/${o.site}`,
    }
  },
}

const datafast: Adapter = {
  title: 'DataFast Chart', icon: ChartColumn, secret: 'datafast', every: 10 * 60_000, link: 'https://datafa.st/dashboard',
  help: 'Create an API key in your DataFast website settings (API).',
  async fetch() {
    const end = new Date(), start = daysAgo(29)
    const ts = await api<any>('datafast', `https://datafa.st/api/v1/analytics/timeseries?startAt=${isoDay(start)}&endAt=${isoDay(end)}&interval=day&fields=visitors,revenue`)
    const rows: any[] = ts.data ?? ts.timeseries ?? []
    const s = rows.map((x) => x.visitors ?? 0)
    const rev = rows.reduce((a, x) => a + (x.revenue ?? 0), 0)
    return { value: abbr(s.reduce((a, b) => a + b, 0)), label: 'visitors (30 days)', series: s, seriesLabel: 'Visitors per day', delta: pctChange(s.slice(-7).reduce((a, b) => a + b, 0), s.slice(-14, -7).reduce((a, b) => a + b, 0)), stats: rev ? [{ label: 'Revenue (30 days)', value: money(rev, 'USD', false) }] : undefined, open: 'https://datafa.st/dashboard' }
  },
}

const leetcode: Adapter = {
  title: 'LeetCode Stats', icon: Code, every: 60 * 60_000, fields: [{ key: 'username', label: 'LeetCode username', placeholder: 'your_username' }],
  async fetch(o) {
    if (!o.username) throw new NotConnected()
    const year = new Date().getFullYear()
    const r = await api<any>('public', 'https://leetcode.com/graphql', { method: 'POST', headers: { Referer: 'https://leetcode.com' }, body: {
      query: 'query($u: String!, $y: Int) { allQuestionsCount { difficulty count } matchedUser(username: $u) { profile { ranking } submitStatsGlobal { acSubmissionNum { difficulty count } } userCalendar(year: $y) { streak totalActiveDays } } }',
      variables: { u: o.username, y: year } } })
    const u = r.data?.matchedUser
    if (!u) throw new Error('LeetCode user not found')
    const ac = Object.fromEntries(u.submitStatsGlobal.acSubmissionNum.map((x: any) => [x.difficulty, x.count]))
    const all = Object.fromEntries(r.data.allQuestionsCount.map((x: any) => [x.difficulty, x.count]))
    return {
      value: String(ac.All ?? 0), label: 'solved', ring: (ac.All ?? 0) / Math.max(1, all.All ?? 1), open: `https://leetcode.com/u/${o.username}/`,
      stats: [{ label: 'Easy', value: `${ac.Easy}/${all.Easy}` }, { label: 'Medium', value: `${ac.Medium}/${all.Medium}` }, { label: 'Hard', value: `${ac.Hard}/${all.Hard}` }, { label: 'Ranking', value: abbr(u.profile.ranking) },
        { label: 'Streak', value: `${u.userCalendar?.streak ?? 0} days` }, { label: 'Active days', value: String(u.userCalendar?.totalActiveDays ?? 0) }],
    }
  },
}

// ---------------------------------------------------------------- Google (Analytics, Search Console, Gmail)
const googleHelp = 'Uses your own Google Cloud app (read-only):\n1. console.cloud.google.com → create a project.\n2. Enable "Google Analytics Data API", "Google Analytics Admin API", "Google Search Console API", "Gmail API" and "Google Drive API".\n3. OAuth consent screen: External, add yourself as a test user.\n4. Credentials → Create OAuth client ID → Desktop app.\n5. Paste the client ID and secret here and sign in. One sign-in serves all Google widgets.'
const googleLink = 'https://console.cloud.google.com/apis/credentials'
const g = (url: string, body?: unknown) => api<any>('google', url, body ? { method: 'POST', body } : {})

const searchConsole: Adapter = {
  title: 'Search Console', icon: SearchCheck, signIn: 'google', help: googleHelp, link: googleLink, every: 60 * 60_000,
  fields: [{ key: 'site', label: 'Property', placeholder: 'sc-domain:example.com or https://example.com/' }],
  tabs: [{ id: 'query', label: 'Queries' }, { id: 'page', label: 'Pages' }, { id: 'country', label: 'Countries' }, { id: 'device', label: 'Devices' }],
  async fetch(o, c) {
    if (!await invoke<boolean>('google_status')) throw new NotConnected()
    if (!o.site) {
      const s = await g('https://www.googleapis.com/webmasters/v3/sites')
      return { value: '–', label: 'pick a property', rows: (s.siteEntry ?? []).map((x: any) => ({ title: x.siteUrl, sub: x.permissionLevel })), rowsTitle: 'Your properties: copy one into settings (gear)' }
    }
    const base = `https://www.googleapis.com/webmasters/v3/sites/${encodeURIComponent(o.site)}/searchAnalytics/query`
    // Search Console data lags ~2 days.
    const end = daysAgo(2), start = daysAgo(29), pend = daysAgo(30), pstart = daysAgo(57)
    const [cur, prev, top] = await Promise.all([
      g(base, { startDate: isoDay(start), endDate: isoDay(end), dimensions: ['date'] }),
      g(base, { startDate: isoDay(pstart), endDate: isoDay(pend) }),
      g(base, { startDate: isoDay(start), endDate: isoDay(end), dimensions: [c.tab ?? 'query'], rowLimit: 10 }),
    ])
    const rows: any[] = cur.rows ?? []
    const clicks = rows.reduce((a, r) => a + r.clicks, 0), impr = rows.reduce((a, r) => a + r.impressions, 0)
    const pc = prev.rows?.[0]?.clicks ?? 0
    return {
      value: abbr(clicks), label: 'clicks (28 days)', delta: pctChange(clicks, pc), series: rows.map((r) => r.clicks), seriesLabel: 'Clicks per day',
      stats: [{ label: 'Impressions', value: abbr(impr) }, { label: 'CTR', value: `${impr ? ((clicks / impr) * 100).toFixed(1) : 0}%` }, { label: 'Avg. position', value: rows.length ? (rows.reduce((a, r) => a + r.position, 0) / rows.length).toFixed(1) : '–' }],
      rows: (top.rows ?? []).map((r: any) => ({ title: r.keys[0], value: `${abbr(r.clicks)} · ${abbr(r.impressions)}` })), rowsTitle: 'Top (clicks · impressions)',
      open: `https://search.google.com/search-console?resource_id=${encodeURIComponent(o.site)}`,
    }
  },
}

const ga: Adapter = {
  title: 'Google Analytics', icon: Activity, signIn: 'google', help: googleHelp, link: googleLink, every: 10 * 60_000,
  fields: [{ key: 'property', label: 'GA4 property ID (numbers)', placeholder: '123456789' }],
  tabs: [{ id: 'overview', label: 'Overview' }, { id: 'pages', label: 'Pages' }, { id: 'sources', label: 'Sources' }, { id: 'realtime', label: 'Realtime' }],
  async fetch(o, c) {
    if (!await invoke<boolean>('google_status')) throw new NotConnected()
    if (!o.property) {
      const s = await g('https://analyticsadmin.googleapis.com/v1beta/accountSummaries?pageSize=200')
      const rows: Row[] = (s.accountSummaries ?? []).flatMap((a: any) => (a.propertySummaries ?? []).map((p: any) => ({ title: p.displayName, sub: `${a.displayName} · ID ${p.property.split('/')[1]}` })))
      return { value: '–', label: 'pick a property', rows, rowsTitle: 'Your properties: copy an ID into settings (gear)' }
    }
    const P = `https://analyticsdata.googleapis.com/v1beta/properties/${String(o.property).replace(/\D/g, '')}`
    const rt = await g(`${P}:runRealtimeReport`, { metrics: [{ name: 'activeUsers' }] }).catch(() => null)
    const live = +(rt?.rows?.[0]?.metricValues?.[0]?.value ?? 0)
    if (c.tab === 'realtime') {
      const r = await g(`${P}:runRealtimeReport`, { dimensions: [{ name: 'unifiedScreenName' }], metrics: [{ name: 'activeUsers' }], limit: 10 })
      return { value: String(live), label: 'active now', rows: (r.rows ?? []).map((x: any) => ({ title: x.dimensionValues[0].value, value: x.metricValues[0].value })), rowsTitle: 'Pages being viewed', open: 'https://analytics.google.com/' }
    }
    const [tot, ts] = await Promise.all([
      g(`${P}:runReport`, { dateRanges: [{ startDate: '27daysAgo', endDate: 'today' }, { startDate: '55daysAgo', endDate: '28daysAgo' }], metrics: [{ name: 'activeUsers' }, { name: 'sessions' }, { name: 'screenPageViews' }, { name: 'averageSessionDuration' }] }),
      g(`${P}:runReport`, { dateRanges: [{ startDate: '27daysAgo', endDate: 'today' }], dimensions: [{ name: 'date' }], metrics: [{ name: 'activeUsers' }], orderBys: [{ dimension: { dimensionName: 'date' } }] }),
    ])
    const val = (range: number, i: number) => +(tot.rows?.find((r: any) => r.dimensionValues?.[0]?.value === `date_range_${range}`)?.metricValues?.[i]?.value ?? tot.rows?.[range]?.metricValues?.[i]?.value ?? 0)
    const users = val(0, 0), pusers = val(1, 0)
    const v: View = {
      value: abbr(users), label: 'users (28 days)', delta: pctChange(users, pusers), series: (ts.rows ?? []).map((r: any) => +r.metricValues[0].value), seriesLabel: 'Users per day',
      stats: [{ label: 'Active now', value: String(live) }, { label: 'Sessions', value: abbr(val(0, 1)) }, { label: 'Views', value: abbr(val(0, 2)) }, { label: 'Avg. session', value: `${Math.round(val(0, 3))} s` }],
      open: 'https://analytics.google.com/',
    }
    if (c.tab === 'pages' || c.tab === 'sources') {
      const dim = c.tab === 'pages' ? 'pagePath' : 'sessionSourceMedium'
      const r = await g(`${P}:runReport`, { dateRanges: [{ startDate: '27daysAgo', endDate: 'today' }], dimensions: [{ name: dim }], metrics: [{ name: c.tab === 'pages' ? 'screenPageViews' : 'sessions' }], limit: 10, orderBys: [{ metric: { metricName: c.tab === 'pages' ? 'screenPageViews' : 'sessions' }, desc: true }] })
      v.rows = (r.rows ?? []).map((x: any) => ({ title: x.dimensionValues[0].value, value: abbr(+x.metricValues[0].value) }))
      v.rowsTitle = c.tab === 'pages' ? 'Top pages (views)' : 'Top sources (sessions)'
    }
    return v
  },
}

const MAIL_TABS = [{ id: 'inbox', label: 'Inbox' }, { id: 'unread', label: 'Unread' }, { id: 'starred', label: 'Starred' }]
const gmail: Adapter = {
  title: 'Gmail', icon: Mail, signIn: 'google', help: googleHelp, link: googleLink, every: 2 * 60_000, tabs: MAIL_TABS, search: true,
  async fetch(_o, c) {
    if (!await invoke<boolean>('google_status')) throw new NotConnected()
    const G = 'https://gmail.googleapis.com/gmail/v1/users/me'
    const inbox = await g(`${G}/labels/INBOX`)
    const q = { inbox: 'in:inbox', unread: 'in:inbox is:unread', starred: 'is:starred' }[c.tab ?? 'inbox'] + (c.query ? ` ${c.query}` : '')
    const list = await g(`${G}/messages?maxResults=15&q=${encodeURIComponent(q)}`)
    const msgs = await Promise.all((list.messages ?? []).map((m: any) => g(`${G}/messages/${m.id}?format=metadata&metadataHeaders=From&metadataHeaders=Subject&metadataHeaders=Date`)))
    const h = (m: any, n: string) => m.payload?.headers?.find((x: any) => x.name === n)?.value ?? ''
    return {
      value: abbr(inbox.messagesUnread ?? 0), label: 'unread', open: 'https://mail.google.com/',
      rows: msgs.map((m: any) => ({ title: h(m, 'Subject') || '(no subject)', sub: `${h(m, 'From').replace(/<.*>/, '').trim()} · ${ago(+m.internalDate)}`, href: `https://mail.google.com/mail/u/0/#all/${m.id}`, dim: !m.labelIds?.includes('UNREAD') })),
    }
  },
}

const yahoo: Adapter = {
  title: 'Yahoo Mail', icon: Mails, signIn: 'yahoo', every: 2 * 60_000, tabs: [{ id: 'inbox', label: 'Inbox' }, { id: 'unread', label: 'Unread' }, { id: 'flagged', label: 'Flagged' }], search: true,
  link: 'https://login.yahoo.com/account/security/app-passwords',
  help: 'Yahoo → Account info → Account security → Generate app password (name it "Strata"). Paste the 16-letter password here. Strata only reads: nothing is ever marked as read.',
  async fetch(_o, c) {
    type S = { email: string; unread: number; total: number; messages: { uid: number; from: string; subject: string; date: string; seen: boolean; flagged: boolean }[] }
    let s: S
    try { s = await invoke<S>('yahoo_mail', { view: c.tab ?? 'inbox', query: c.query || null }) } catch (e) { if (String(e).includes('not-connected')) throw new NotConnected(); throw new Error(String(e)) }
    return {
      value: abbr(s.unread), label: 'unread', open: 'https://mail.yahoo.com/',
      rows: s.messages.map((m) => ({ title: m.subject || '(no subject)', sub: `${m.from} · ${m.date ? ago(Date.parse(m.date)) : ''}`, href: `https://mail.yahoo.com/d/search/keyword=${encodeURIComponent(m.subject)}`, dim: m.seen })),
      note: s.email,
    }
  },
}

const meta: Adapter = {
  title: 'Meta Ads', icon: Megaphone, secret: 'meta', every: 15 * 60_000, tabs: MONEY_TABS, tileTab: 'today', link: 'https://business.facebook.com/settings/system-users',
  help: 'Business settings → System users → add one with access to your ad account → Generate token with ads_read. Paste the token and your ad account ID.',
  keyLabel: 'Access token (ads_read)', fields: [{ key: 'account', label: 'Ad account ID', placeholder: '1234567890 (without act_)' }],
  async fetch(o, c) {
    if (!o.account) throw new NotConnected()
    const A = `https://graph.facebook.com/v21.0/act_${String(o.account).replace(/^act_/, '')}`
    const preset = { today: 'today', week: 'last_7d', month: 'last_30d' }[c.tab ?? 'today'] ?? 'today'
    const [tot, ts, camp, acct] = await Promise.all([
      api<any>('meta', `${A}/insights?fields=spend,impressions,clicks,ctr,cpc,cpm,actions&date_preset=${preset}`),
      api<any>('meta', `${A}/insights?fields=spend&date_preset=last_30d&time_increment=1&limit=40`),
      api<any>('meta', `${A}/insights?level=campaign&fields=campaign_name,spend,clicks&date_preset=${preset}&limit=10&sort=spend_descending`),
      api<any>('meta', `${A}?fields=currency,name`),
    ])
    const t = tot.data?.[0] ?? {}
    const cur = acct.currency ?? 'USD'
    const series = daily((ts.data ?? []).map((d: any) => [Date.parse(d.date_start), +d.spend]), 30)
    const results = (t.actions ?? []).filter((a: any) => /purchase|lead|complete_registration/.test(a.action_type)).reduce((x: number, a: any) => x + +a.value, 0)
    return {
      value: money(+(t.spend ?? 0), cur), label: `spend ${preset === 'today' ? 'today' : preset === 'last_7d' ? '7 days' : '30 days'}`,
      series, seriesLabel: 'Spend per day (30 days)', open: `https://adsmanager.facebook.com/adsmanager/manage/campaigns?act=${String(o.account).replace(/^act_/, '')}`,
      stats: [{ label: 'Impressions', value: abbr(+(t.impressions ?? 0)) }, { label: 'Clicks', value: abbr(+(t.clicks ?? 0)) }, { label: 'CTR', value: `${(+(t.ctr ?? 0)).toFixed(2)}%` }, { label: 'CPC', value: money(+(t.cpc ?? 0), cur, false) }, { label: 'CPM', value: money(+(t.cpm ?? 0), cur, false) }, { label: 'Results', value: abbr(results) }],
      rows: (camp.data ?? []).map((x: any) => ({ title: x.campaign_name, value: money(+x.spend, cur, false), sub: `${abbr(+x.clicks)} clicks` })), rowsTitle: 'Campaigns by spend',
      note: acct.name,
    }
  },
}

export const DEV: Record<string, Adapter> = {
  github, 'github-activity': githubActivity, 'github-overview': githubOverview, linear, supabase, resend, dub, plausible,
  'datafast-chart': datafast, leetcode, 'search-console': searchConsole, 'google-analytics': ga, gmail, 'yahoo-mail': yahoo, 'meta-ads': meta,
}
export { Inbox }
