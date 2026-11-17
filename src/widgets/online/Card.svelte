<script lang="ts">
  // One card for every adapter widget: connect form (pasted key, Google or Yahoo sign-in, settings),
  // tabs, search, headline number with change, chart, limits with reset times, stats, lists.
  import { ExternalLink, RefreshCw, Search, Settings2 } from '@lucide/svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { ipc } from '../../lib/ipc'
  import { ago, deleteSecret, hasSecret, setSecret, until, useService } from '../../lib/services.svelte'
  import Spark from '../../lib/ui/Spark.svelte'
  import WindowChart from '../../lib/ui/WindowChart.svelte'
  import type { CardProps } from '../types'
  import { ADAPTERS } from './adapters'
  let { item, act }: CardProps = $props()
  const a = ADAPTERS[item.widget]
  const Icon = a.icon
  let opts = $state<Record<string, any>>({ ...item.options })
  // Settings fields are edited in a draft and applied on Save (typing never triggers requests).
  let draft = $state<Record<string, any>>({ ...item.options })
  let tab = $state<string | undefined>(item.widget.endsWith('-chart') ? 'week' : a.tileTab ?? a.tabs?.[0]?.id)
  let q = $state('')
  let qDebounced = $state('')
  $effect(() => { const v = q; const t = setTimeout(() => (qDebounced = v), 400); return () => clearTimeout(t) })
  const svc = useService(item.widget, a, () => opts, () => ({ tab, query: a.search ? qDebounced : undefined }))
  const v = $derived(svc.state.view)

  // ---- connect / settings
  let showSettings = $state(false)
  let key = $state('')
  let busy = $state(false)
  let err = $state('')
  let gid = $state(''), gsecret = $state('')
  let yemail = $state(''), ypass = $state('')
  let haveKey = $state(false)
  if (a.secret) hasSecret(a.secret).then((h) => (haveKey = h))
  const needsFields = $derived((a.fields ?? []).some((f) => !opts[f.key]))
  const showConnect = $derived(!svc.state.connected || needsFields || showSettings)
  // While the connect form is open the card stays open when you switch windows to copy a key.
  $effect(() => {
    const on = showConnect && !!(a.secret || a.signIn || a.fields)
    invoke('card_sticky', { on })
    return () => { invoke('card_sticky', { on: false }) }
  })
  const signal = () => localStorage.setItem(`svc-signal:${item.widget}`, String(Date.now()))

  async function connect() {
    busy = true; err = ''
    try {
      for (const f of a.fields ?? []) draft[f.key] = String(draft[f.key] ?? '').trim()
      opts = { ...opts, ...$state.snapshot(draft) }
      act('set-options', $state.snapshot(opts))
      if (a.secret && key.trim()) { await setSecret(a.secret, key.trim()); haveKey = true; key = '' }
      if (a.signIn === 'google' && (gid || gsecret)) await invoke('google_connect', { id: gid, secret: gsecret })
      if (a.signIn === 'yahoo' && (yemail || ypass)) await invoke('yahoo_login', { email: yemail, password: ypass })
      showSettings = false
      await svc.refresh()
      signal()
    } catch (e) { err = String(e) } finally { busy = false }
  }
  async function disconnect() {
    if (a.secret) await deleteSecret(a.secret)
    if (a.signIn === 'google') await invoke('google_disconnect')
    if (a.signIn === 'yahoo') await invoke('yahoo_disconnect')
    haveKey = false; showSettings = false
    await svc.refresh()
    signal()
  }
  const open = (url: string) => ipc.launch(url)
</script>

<div class="oc">
  <header>
    <span class="ic"><Icon size={16} /></span>
    <span class="tt">{a.title}{#if a.preview}<em>Preview</em>{/if}</span>
    {#if v?.updated}<span class="up" title="Last updated">{ago(v.updated)}</span>{/if}
    <button aria-label="Refresh" class:spin={svc.state.loading} onclick={() => svc.refresh()}><RefreshCw size={13} /></button>
    {#if a.secret || a.signIn || a.fields}<button aria-label="Settings" class:on={showSettings} onclick={() => (showSettings = !showSettings)}><Settings2 size={13} /></button>{/if}
    {#if v?.open}<button aria-label="Open" onclick={() => open(v.open!)}><ExternalLink size={13} /></button>{/if}
  </header>

  {#if showConnect && (a.secret || a.signIn || a.fields)}
    <div class="connect">
      {#if a.help}<p class="help">{a.help}</p>{/if}
      {#if a.link}<button class="link" onclick={() => open(a.link!)}>Open the page where you create it <ExternalLink size={11} /></button>{/if}
      {#each a.fields ?? [] as f}
        <label>{f.label}<input bind:value={draft[f.key]} placeholder={f.placeholder} spellcheck="false" /></label>
      {/each}
      {#if a.secret}
        <label>{a.keyLabel ?? 'API key'} {#if haveKey}<span class="ok">saved</span>{/if}
          <input type="password" bind:value={key} placeholder={haveKey ? 'Paste a new key to replace it' : 'Paste here'} spellcheck="false" />
        </label>
      {/if}
      {#if a.signIn === 'google'}
        <label>OAuth client ID<input bind:value={gid} placeholder="…apps.googleusercontent.com" spellcheck="false" /></label>
        <label>OAuth client secret<input type="password" bind:value={gsecret} placeholder="GOCSPX-…" spellcheck="false" /></label>
      {/if}
      {#if a.signIn === 'yahoo'}
        <label>Yahoo address<input bind:value={yemail} placeholder="you@yahoo.com" spellcheck="false" /></label>
        <label>App password<input type="password" bind:value={ypass} placeholder="16 letters from Yahoo Account security" spellcheck="false" /></label>
      {/if}
      <p class="fine">Stored in Windows Credential Manager on this PC; only sent to {a.title}'s own servers.</p>
      {#if err}<p class="err">{err}</p>{/if}
      <div class="btns">
        {#if svc.state.connected && (haveKey || a.signIn)}<button class="danger" onclick={disconnect}>Disconnect</button>{/if}
        <button class="primary" disabled={busy} onclick={connect}>{busy ? (a.signIn === 'google' ? 'Finish sign-in in your browser…' : 'Connecting…') : a.signIn === 'google' ? 'Sign in with Google' : 'Save'}</button>
      </div>
    </div>
  {:else}
    {#if a.tabs}
      <div class="tabs">{#each a.tabs as t}<button class:on={tab === t.id} onclick={() => (tab = t.id)}>{t.label}</button>{/each}</div>
    {/if}
    {#if a.search}
      <label class="search"><Search size={13} /><input bind:value={q} placeholder="Search" spellcheck="false" /></label>
    {/if}
    {#if v?.warning}<p class="warn">{v.warning}</p>{/if}
    {#if svc.state.error}<p class="err">{svc.state.error}</p>{/if}
    {#if v}
      <div class="head">
        <span class="big">{v.value}</span>
        {#if v.delta != null}<span class="d" class:neg={v.delta < 0}>{v.delta >= 0 ? '+' : ''}{v.delta.toFixed(1)}%</span>{/if}
        {#if v.label}<span class="lab">{v.label}</span>{/if}
      </div>
      {#if v.limits?.length}
        <div class="limits">
          {#each v.limits as l}
            <div class="lim">
              <div class="lt"><span>{l.label}</span><span class="pc">{Math.round(l.pct)}%</span></div>
              <div class="bar"><i style="width:{Math.min(100, l.pct)}%" class:hot={l.pct > 85} class:warmish={l.pct > 60 && l.pct <= 85}></i></div>
              <div class="ls">{#if l.resets}Resets in {until(l.resets)}{/if}{#if l.detail}<span>{l.detail}</span>{/if}</div>
            </div>
          {/each}
        </div>
      {/if}
      {#if v.window}
        <WindowChart {...v.window} />
      {:else if v.series && v.series.length > 1}
        <div class="chart"><Spark values={v.series} width={312} height={70} min={v.seriesFromZero === false ? undefined : 0} />{#if v.seriesLabel}<small>{v.seriesLabel}</small>{/if}</div>
      {/if}
      {#if v.grid}
        {@const m = Math.max(1, ...v.grid)}
        <div class="cgrid">{#each v.grid as c}<i style="opacity:{c ? 0.3 + 0.7 * (c / m) : 0.1}"></i>{/each}</div>
      {/if}
      {#if v.stats?.length}
        <div class="stats">{#each v.stats as s}<div><small>{s.label}</small><strong>{s.value}</strong></div>{/each}</div>
      {/if}
      {#if v.rows?.length}
        {#if v.rowsTitle}<div class="rt">{v.rowsTitle}</div>{/if}
        <ul class="rows">
          {#each v.rows as r}
            <li><button class:dim={r.dim} disabled={!r.href} onclick={() => r.href && open(r.href)}>
              {#if r.icon}<img src={r.icon} alt="" />{/if}
              <span class="rx"><span class="rtt">{r.title}</span>{#if r.sub}<span class="rs">{r.sub}</span>{/if}</span>
              {#if r.value}<span class="rv" class:good={r.tone === 'good'} class:bad={r.tone === 'bad'}>{r.value}</span>{/if}
            </button></li>
          {/each}
        </ul>
      {/if}
      {#if v.note}<p class="note">{v.note}</p>{/if}
    {:else if !svc.state.error}
      <p class="note">Loading…</p>
    {/if}
  {/if}
</div>

<style>
  .oc { width: 344px; padding: 14px; display: flex; flex-direction: column; gap: 10px; max-height: 640px; overflow-y: auto; }
  header { display: flex; align-items: center; gap: 8px; }
  .ic { width: 26px; height: 26px; border-radius: 8px; display: grid; place-items: center; background: var(--surface-press); }
  .tt { flex: 1; font: 600 14px var(--font-display); display: flex; align-items: center; gap: 6px; }
  .tt em { font-style: normal; font-size: 10px; padding: 1px 6px; border-radius: 6px; background: #ff9f0a33; color: #ffb340; }
  .up { font-size: 11px; color: var(--fg-3); }
  header button { width: 26px; height: 26px; border-radius: 8px; display: grid; place-items: center; color: var(--fg-2); }
  header button:hover, header button.on { background: var(--surface-hover); color: var(--fg); }
  header button.spin :global(svg) { animation: spin 0.9s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
  .connect { display: flex; flex-direction: column; gap: 8px; }
  .help { margin: 0; font-size: 12.5px; color: var(--fg-2); line-height: 1.45; white-space: pre-line; }
  .link { align-self: flex-start; display: flex; align-items: center; gap: 5px; font-size: 12px; color: var(--accent); }
  .connect label { display: flex; flex-direction: column; gap: 4px; font-size: 11.5px; color: var(--fg-2); }
  .connect input { padding: 7px 9px; border-radius: 8px; border: 1px solid var(--border); background: var(--surface); outline: none; font-size: 12.5px; }
  .ok { color: var(--good); font-weight: 600; }
  .fine { margin: 0; font-size: 11px; color: var(--fg-3); }
  .btns { display: flex; justify-content: flex-end; gap: 6px; }
  .btns button { padding: 6px 14px; border-radius: 8px; background: var(--surface-press); font-size: 12.5px; }
  .btns .primary { background: var(--accent); color: white; }
  .btns .danger:hover { background: var(--danger); color: white; }
  .tabs { display: flex; gap: 4px; padding: 3px; border-radius: 9px; background: var(--surface); }
  .tabs button { flex: 1; padding: 5px 0; border-radius: 7px; font-size: 12px; color: var(--fg-2); }
  .tabs button.on { background: var(--surface-press); color: var(--fg); }
  .search { display: flex; align-items: center; gap: 6px; padding: 6px 9px; border-radius: 8px; background: var(--surface); color: var(--fg-3); }
  .search input { flex: 1; background: none; border: 0; outline: none; font-size: 12.5px; color: var(--fg); }
  .head { display: flex; align-items: baseline; gap: 8px; flex-wrap: wrap; }
  .big { font: 600 30px var(--font-display); font-variant-numeric: tabular-nums; letter-spacing: -0.01em; }
  .d { font-size: 13px; font-weight: 600; color: var(--good); }
  .d.neg { color: var(--danger); }
  .lab { font-size: 12px; color: var(--fg-3); }
  .limits { display: flex; flex-direction: column; gap: 10px; }
  .lt { display: flex; justify-content: space-between; font-size: 12.5px; }
  .pc { font-variant-numeric: tabular-nums; font-weight: 600; }
  .bar { height: 7px; border-radius: 4px; background: var(--surface-press); overflow: hidden; margin-top: 4px; }
  .bar i { display: block; height: 100%; border-radius: 4px; background: var(--accent); transition: width 500ms var(--ease-out); }
  .bar i.warmish { background: #ff9f0a; } .bar i.hot { background: var(--danger); }
  .ls { display: flex; justify-content: space-between; font-size: 11px; color: var(--fg-3); margin-top: 3px; }
  .chart { display: flex; flex-direction: column; gap: 3px; }
  .chart small { font-size: 10.5px; color: var(--fg-3); }
  .cgrid { display: grid; grid-template-rows: repeat(7, 5px); grid-auto-flow: column; grid-auto-columns: 5px; gap: 1px; overflow: hidden; }
  .cgrid i { background: var(--good); border-radius: 1px; }
  .stats { display: grid; grid-template-columns: 1fr 1fr; gap: 6px; }
  .stats div { padding: 7px 9px; border-radius: 9px; background: var(--surface); display: flex; flex-direction: column; min-width: 0; }
  .stats small { font-size: 10.5px; color: var(--fg-3); }
  .stats strong { font: 600 14px var(--font-display); font-variant-numeric: tabular-nums; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .rt { font-size: 11px; font-weight: 600; color: var(--fg-3); text-transform: uppercase; letter-spacing: .04em; }
  .rows { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 2px; }
  .rows button { width: 100%; display: flex; align-items: center; gap: 9px; padding: 6px 7px; border-radius: 8px; text-align: left; }
  .rows button:not(:disabled):hover { background: var(--surface-hover); }
  .rows button:disabled { cursor: default; color: inherit; }
  .rows button.dim { opacity: 0.6; }
  .rows img { width: 22px; height: 22px; border-radius: 6px; object-fit: cover; flex: none; }
  .rx { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .rtt { font-size: 12.5px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .rs { font-size: 11px; color: var(--fg-3); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .rv { font-size: 12px; font-variant-numeric: tabular-nums; color: var(--fg-2); flex: none; }
  .rv.good { color: var(--good); } .rv.bad { color: var(--danger); }
  .note { margin: 0; font-size: 11.5px; color: var(--fg-3); line-height: 1.4; }
  .warn { margin: 0; padding: 7px 9px; border-radius: 8px; background: #ff9f0a22; color: #ffb340; font-size: 12px; }
  .err { margin: 0; color: var(--danger); font-size: 12px; overflow-wrap: anywhere; }
</style>
