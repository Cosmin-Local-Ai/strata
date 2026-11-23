<script lang="ts">
  // Cloud Storage card: one panel per account with space used, free, breakdown and the latest files.
  // Click an account (or Open) to open it on the web, where you add and manage files. Connect each
  // account right here (Google: the sign-in shared with Gmail / Analytics; Digi: email + app password).
  import { Cloud, ExternalLink, HardDrive, LoaderCircle, RefreshCw, Settings2 } from '@lucide/svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { ipc } from '../../lib/ipc'
  import { ago, setSecret } from '../../lib/services.svelte'
  import type { CardProps } from '../types'
  import { cloud, DIGI, pct, refresh, size, type Account } from './cloud.svelte'
  let { item, act, close }: CardProps = $props()
  let ids = $state<string[]>(item.options.accounts ?? ['drive', 'digi'])
  let digiEmail = $state<string>(item.options.digiEmail ?? '')
  const box = cloud(() => ids, () => digiEmail)
  const accts = $derived(box.accounts.filter((a) => ids.includes(a.id)))
  const color = (id: string) => (id === 'drive' ? '#1fa463' : '#1e88e5')

  let editing = $state(false)
  let email = $state(digiEmail), pass = $state('')
  let busy = $state(''), err = $state('')
  let clientSaved = $state(false)
  invoke<boolean>('google_client_saved').then((v) => (clientSaved = v))
  $effect(() => { invoke('card_sticky', { on: editing || accts.some((a) => a.state === 'connect') }); return () => { invoke('card_sticky', { on: false }) } })

  function openUrl(u: string) { ipc.launch(u); close() }
  async function googleSignIn() {
    busy = 'drive'; err = ''
    try { await invoke('google_connect', { id: '', secret: '' }); await refresh(ids, digiEmail) } catch (e) { err = String(e) } finally { busy = '' }
  }
  async function digiConnect() {
    if (!email.trim() || !pass.trim()) { err = 'Enter the email and the app password'; return }
    busy = 'digi'; err = ''
    try {
      await setSecret('digistorage', pass.trim()); pass = ''
      digiEmail = email.trim(); act('set-options', { digiEmail })
      await refresh(ids, digiEmail)
    } catch (e) { err = String(e) } finally { busy = '' }
  }
  function toggle(id: string) {
    ids = ids.includes(id) ? ids.filter((x) => x !== id) : [...ids, id].sort()
    act('set-options', { accounts: $state.snapshot(ids) })
  }
  const free = (a: Account) => Math.max(0, a.total - a.used)
</script>

<div class="cc">
  <div class="hd">
    <Cloud size={16} /><strong>Cloud storage</strong><span class="sp"></span>
    <button class="ic" title="Refresh" onclick={() => refresh(ids, digiEmail)}>{#if box.loading}<span class="spin"><LoaderCircle size={13} /></span>{:else}<RefreshCw size={13} />{/if}</button>
    <button class="ic" class:on={editing} title="Accounts" onclick={() => (editing = !editing)}><Settings2 size={13} /></button>
  </div>
  {#if editing}
    <div class="set">
      <label class="chk"><input type="checkbox" checked={ids.includes('drive')} onchange={() => toggle('drive')} /> Google Drive</label>
      <label class="chk"><input type="checkbox" checked={ids.includes('digi')} onchange={() => toggle('digi')} /> Digi Storage</label>
    </div>
  {/if}

  {#each accts as a (a.id)}
    <div class="acc" style="--c:{color(a.id)}">
      {#if a.state === 'ok'}
        <button class="main" onclick={() => openUrl(a.open)} title="Open {a.name} on the web">
          <span class="logo">{#if a.id === 'drive'}<HardDrive size={18} />{:else}<Cloud size={18} />{/if}</span>
          <span class="tx"><strong>{a.name}</strong><small>{a.email ?? ''}</small></span>
          <span class="pc">{Math.round(pct(a) * 100)}%</span>
          <ExternalLink size={13} />
        </button>
        <div class="bar"><i style="width:{pct(a) * 100}%"></i></div>
        <div class="nums"><span><b>{size(a.used)}</b> used</span><span><b>{size(free(a))}</b> free</span><span>of <b>{size(a.total)}</b></span></div>
        {#if a.breakdown}<div class="bd">{#each a.breakdown as b}<span>{b.label} <b>{size(b.bytes)}</b></span>{/each}</div>{/if}
        {#if a.recent.length}
          <ul>
            {#each a.recent as r}<li><button onclick={() => openUrl(r.href)}><span class="fn">{r.name}</span><small>{ago(r.when)}</small></button></li>{/each}
          </ul>
        {/if}
      {:else if a.id === 'drive'}
        <div class="main static"><span class="logo"><HardDrive size={18} /></span><span class="tx"><strong>Google Drive</strong><small>{a.error ?? 'Not connected'}</small></span></div>
        {#if clientSaved}
          <button class="btn primary" disabled={busy === 'drive'} onclick={googleSignIn}>{busy === 'drive' ? 'Waiting for the browser…' : 'Sign in to Google'}</button>
          <p class="hint">Read-only: file names and dates, never contents. Enable the Google Drive API in your Google Cloud project first.</p>
        {:else}
          <p class="hint">Set up Google once in the Google Drive (or Gmail) widget: it needs your own Google Cloud client. Then sign in here.</p>
        {/if}
      {:else}
        <div class="main static"><span class="logo"><Cloud size={18} /></span><span class="tx"><strong>Digi Storage</strong><small>{a.error ?? 'Not connected'}</small></span></div>
        <form class="login" onsubmit={(e) => { e.preventDefault(); digiConnect() }}>
          <label><span>Email</span><input bind:value={email} type="email" placeholder="you@example.com" spellcheck="false" autocomplete="off" /></label>
          <label><span>App password</span><input bind:value={pass} type="password" placeholder="From Digi Storage → Preferences" autocomplete="new-password" /></label>
          <div class="row">
            <button class="btn primary" type="submit" disabled={busy === 'digi'}>{busy === 'digi' ? 'Connecting…' : 'Connect'}</button>
            <button class="btn" type="button" onclick={() => ipc.launch(`${DIGI}/login?redirect=%2Fapp%2F`)}>Get an app password</button>
          </div>
        </form>
        <p class="hint">Digi Storage → your name → Preferences → Password → App passwords. It can be revoked any time.</p>
      {/if}
    </div>
  {:else}
    <p class="hint">Choose your accounts with the gear above.</p>
  {/each}
  {#if err}<p class="err">{err}</p>{/if}
</div>

<style>
  .cc { width: 400px; max-height: 640px; overflow-y: auto; padding: 14px; display: flex; flex-direction: column; gap: 10px; }
  .hd { display: flex; align-items: center; gap: 8px; }
  .hd strong { font-size: 14px; }
  .sp { flex: 1; }
  .ic { width: 28px; height: 28px; border-radius: 8px; display: grid; place-items: center; background: var(--surface); color: var(--fg-2); transition: background var(--t-fast); }
  .ic:hover { background: var(--surface-hover); color: var(--fg); }
  .ic.on { background: var(--accent); color: white; }
  .set { display: flex; gap: 14px; padding: 8px 10px; border-radius: 10px; background: var(--surface); }
  .chk { display: flex; align-items: center; gap: 6px; font-size: 12.5px; }
  .acc { display: flex; flex-direction: column; gap: 7px; padding: 10px; border-radius: 12px; background: var(--surface); }
  .main { display: flex; align-items: center; gap: 10px; text-align: left; border-radius: 9px; padding: 4px; margin: -4px; color: var(--fg-3); transition: background var(--t-fast); }
  button.main:hover { background: var(--surface-hover); }
  .logo { width: 34px; height: 34px; border-radius: 10px; display: grid; place-items: center; color: var(--c); background: color-mix(in srgb, var(--c) 16%, transparent); flex: none; }
  .tx { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .tx strong { font-size: 13.5px; color: var(--fg); }
  .tx small { font-size: 11px; color: var(--fg-3); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .pc { font: 700 18px var(--font-display); color: var(--fg); font-variant-numeric: tabular-nums; }
  .bar { height: 7px; border-radius: 4px; background: var(--surface-press); overflow: hidden; }
  .bar i { display: block; height: 100%; border-radius: 4px; background: var(--c); }
  .nums, .bd { display: flex; justify-content: space-between; font-size: 11.5px; color: var(--fg-3); }
  .nums b, .bd b { color: var(--fg); font-weight: 600; }
  .bd { justify-content: flex-start; gap: 12px; font-size: 11px; }
  ul { list-style: none; margin: 2px 0 0; padding: 0; display: flex; flex-direction: column; }
  li button { width: 100%; display: flex; align-items: center; gap: 8px; padding: 4px 6px; border-radius: 7px; text-align: left; transition: background var(--t-fast); }
  li button:hover { background: var(--surface-hover); }
  .fn { flex: 1; min-width: 0; font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  li small { font-size: 10.5px; color: var(--fg-3); }
  .login { display: flex; flex-direction: column; gap: 8px; margin-top: 2px; }
  .login label { display: flex; flex-direction: column; gap: 4px; }
  .login label span { font-size: 11px; font-weight: 600; color: var(--fg-2); }
  .login input { width: 100%; box-sizing: border-box; height: 32px; padding: 0 10px; border-radius: 8px; border: 1px solid var(--border); background: rgba(0, 0, 0, 0.22);
                 outline: none; font-size: 12.5px; color: var(--fg); transition: border-color var(--t-fast), background var(--t-fast); }
  .login input::placeholder { color: var(--fg-3); }
  .login input:hover { border-color: color-mix(in srgb, var(--fg) 25%, transparent); }
  .login input:focus { border-color: var(--accent); background: rgba(0, 0, 0, 0.3); }
  /* Windows' autofill paints filled boxes light blue with dark text: keep them like the others. */
  .login input:-webkit-autofill { -webkit-text-fill-color: var(--fg); box-shadow: 0 0 0 40px #1d1d22 inset; caret-color: var(--fg); }
  .login .row { margin-top: 2px; }
  .row { display: flex; gap: 6px; }
  .btn { padding: 6px 12px; border-radius: 8px; background: var(--surface-press); font-size: 12.5px; transition: background var(--t-fast), filter var(--t-fast); align-self: flex-start; }
  .btn:hover:not(:disabled) { background: var(--surface-hover); }
  .btn.primary { background: var(--accent); color: white; }
  .btn.primary:hover:not(:disabled) { filter: brightness(1.12); background: var(--accent); }
  .btn:disabled { opacity: 0.5; }
  .hint { margin: 0; font-size: 11px; color: var(--fg-3); line-height: 1.45; }
  .err { margin: 0; color: var(--danger); font-size: 12px; }
  .spin { display: grid; animation: r 0.9s linear infinite; }
  @keyframes r { to { transform: rotate(360deg); } }
</style>
