<script lang="ts">
  // Vault: passwords and API keys, one click to copy. Secrets live in Windows Credential Manager and
  // only come to this card when you press the eye; copies skip clipboard history and clear after 30 s.
  import { Copy, Eye, EyeOff, KeyRound, Lock, Pencil, Plus, RefreshCw, Search, ShieldCheck, Trash2, User, ExternalLink } from '@lucide/svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { ipc } from '../../lib/ipc'
  import type { CardProps } from '../types'
  let { item }: CardProps = $props()

  type Entry = { id: string; kind: 'password' | 'api'; title: string; username: string; url: string; updated: number }
  let unlocked = $state(false)
  let lockMsg = $state('')
  let entries = $state<Entry[]>([])
  let q = $state('')
  let tab = $state<'all' | 'password' | 'api'>('all')
  let toast = $state('')
  let tt: ReturnType<typeof setTimeout> | undefined
  const say = (m: string) => { toast = m; clearTimeout(tt); tt = setTimeout(() => (toast = ''), 2600) }
  let input = $state<HTMLInputElement>()

  async function refresh() { entries = await invoke<Entry[]>('vault_list'); unlocked = await invoke<boolean>('vault_state') }
  async function unlock() {
    lockMsg = ''
    try {
      const mode = await invoke<string>('vault_unlock', { minutes: item.options.minutes ?? 5 })
      unlocked = true
      if (mode === 'no-hello') say('Windows Hello isn\'t set up, so the vault opened without it')
      queueMicrotask(() => input?.focus())
    } catch (e) { lockMsg = String(e).includes('Not verified') ? 'Not verified. Try again.' : String(e) }
  }
  refresh().then(() => { if (!unlocked) unlock() })

  const shown = $derived(entries
    .filter((e) => tab === 'all' || e.kind === tab)
    .filter((e) => !q || `${e.title} ${e.username} ${e.url}`.toLowerCase().includes(q.trim().toLowerCase()))
    .sort((a, b) => a.title.localeCompare(b.title)))
  const hue = (s: string) => [...s].reduce((h, c) => (h * 31 + c.charCodeAt(0)) % 360, 7)

  async function copy(e: Entry, field: 'secret' | 'username') {
    try { await invoke('vault_copy', { id: e.id, field }); say(`${field === 'secret' ? (e.kind === 'api' ? 'Key' : 'Password') : 'Username'} copied · clears in 30 s`) }
    catch (x) { if (String(x).includes('locked')) { unlocked = false; unlock() } else say(String(x)) }
  }

  // ---- editor
  type Draft = Entry & { secret: string; notes: string; changedSecret: boolean }
  let edit = $state<Draft | null>(null)
  let show = $state(false)
  let genLen = $state(20), genSym = $state(true)
  $effect(() => { invoke('card_sticky', { on: !!edit }); return () => { invoke('card_sticky', { on: false }) } })
  function blank(kind: 'password' | 'api'): Draft { return { id: '', kind, title: '', username: '', url: '', updated: 0, secret: '', notes: '', changedSecret: true } }
  async function open(e: Entry) {
    try { const s = await invoke<{ secret: string; notes: string }>('vault_reveal', { id: e.id }); edit = { ...e, secret: s.secret, notes: s.notes, changedSecret: false }; show = false }
    catch (x) { say(String(x)) }
  }
  async function gen() { if (edit) { edit.secret = await invoke<string>('vault_generate', { length: genLen, symbols: genSym }); edit.changedSecret = true; show = true } }
  async function save() {
    if (!edit || !edit.title.trim()) return say('Give it a name')
    const { secret, notes, changedSecret, ...entry } = $state.snapshot(edit)
    try { await invoke('vault_save', { entry: { ...entry, title: entry.title.trim() }, secret: changedSecret ? secret : null, notes }); edit = null; await refresh(); say('Saved') }
    catch (x) { say(String(x)) }
  }
  async function del() {
    if (!edit?.id) return
    try { await invoke('vault_delete', { id: edit.id }); edit = null; await refresh(); say('Deleted') } catch (x) { say(String(x)) }
  }
  const strength = $derived.by(() => {
    const s = edit?.secret ?? ''
    if (!s) return 0
    const sets = [/[a-z]/, /[A-Z]/, /\d/, /[^A-Za-z0-9]/].filter((r) => r.test(s)).length
    const bits = s.length * Math.log2([0, 26, 52, 62, 90][sets] || 26)
    return bits >= 100 ? 4 : bits >= 70 ? 3 : bits >= 45 ? 2 : 1
  })
</script>

<div class="vc">
  {#if !unlocked}
    <div class="locked">
      <span class="lk"><Lock size={26} /></span>
      <strong>Vault is locked</strong>
      <p>Your passwords and API keys stay in Windows Credential Manager on this PC.</p>
      {#if lockMsg}<p class="err">{lockMsg}</p>{/if}
      <button class="primary" onclick={unlock}><ShieldCheck size={15} /> Unlock with Windows Hello</button>
    </div>
  {:else if edit}
    <div class="seg">
      <button class:on={edit.kind === 'password'} onclick={() => (edit!.kind = 'password')}>Password</button>
      <button class:on={edit.kind === 'api'} onclick={() => (edit!.kind = 'api')}>API key</button>
    </div>
    <label>Name<input bind:value={edit.title} placeholder={edit.kind === 'api' ? 'e.g. OpenAI production' : 'e.g. GitHub'} /></label>
    <label>{edit.kind === 'api' ? 'Service or account (optional)' : 'Username or email'}<input bind:value={edit.username} spellcheck="false" /></label>
    <label>{edit.kind === 'api' ? 'Key' : 'Password'}
      <span class="sec">
        <input type={show ? 'text' : 'password'} value={edit.secret} oninput={(e) => { edit!.secret = e.currentTarget.value; edit!.changedSecret = true }} spellcheck="false" />
        <button aria-label={show ? 'Hide' : 'Show'} onclick={() => (show = !show)}>{#if show}<EyeOff size={14} />{:else}<Eye size={14} />{/if}</button>
        {#if edit.kind === 'password'}<button aria-label="Generate" title="Generate a strong password" onclick={gen}><RefreshCw size={14} /></button>{/if}
      </span>
    </label>
    {#if edit.kind === 'password'}
      <div class="meter"><i class="s{strength}" style="width:{strength * 25}%"></i></div>
      <div class="genrow"><span>Length {genLen}</span><input type="range" min="12" max="48" bind:value={genLen} /><label class="chk"><input type="checkbox" bind:checked={genSym} /> Symbols</label></div>
    {/if}
    <label>Website (optional)<input bind:value={edit.url} placeholder="https://" spellcheck="false" /></label>
    <label>Notes (optional)<textarea bind:value={edit.notes} rows="2"></textarea></label>
    <div class="btns">
      {#if edit.id}<button class="danger" onclick={del}><Trash2 size={13} /> Delete</button>{/if}
      <span class="sp"></span>
      <button onclick={() => (edit = null)}>Cancel</button>
      <button class="primary" onclick={save}>Save</button>
    </div>
  {:else}
    <div class="top">
      <label class="search"><Search size={14} /><input bind:this={input} bind:value={q} placeholder="Search {entries.length} items" spellcheck="false" /></label>
      <button class="ic" aria-label="Add" title="Add" onclick={() => (edit = blank(tab === 'api' ? 'api' : 'password'))}><Plus size={16} /></button>
      <button class="ic" aria-label="Lock" title="Lock now" onclick={async () => { await invoke('vault_lock'); unlocked = false }}><Lock size={15} /></button>
    </div>
    <div class="seg">
      <button class:on={tab === 'all'} onclick={() => (tab = 'all')}>All</button>
      <button class:on={tab === 'password'} onclick={() => (tab = 'password')}>Passwords</button>
      <button class:on={tab === 'api'} onclick={() => (tab = 'api')}>API keys</button>
    </div>
    <ul>
      {#each shown as e (e.id)}
        <li>
          <span class="av" style="--h:{hue(e.title)}">{#if e.kind === 'api'}<KeyRound size={15} />{:else}{e.title.slice(0, 1).toUpperCase()}{/if}</span>
          <span class="tx"><strong>{e.title}</strong><small>{e.username || (e.kind === 'api' ? 'API key' : '')}</small></span>
          {#if e.username}<button class="act" title="Copy username" onclick={() => copy(e, 'username')}><User size={14} /></button>{/if}
          <button class="act main" title={e.kind === 'api' ? 'Copy key' : 'Copy password'} onclick={() => copy(e, 'secret')}><Copy size={14} /></button>
          {#if e.url}<button class="act" title="Open website" onclick={() => ipc.launch(e.url)}><ExternalLink size={14} /></button>{/if}
          <button class="act" title="Edit" onclick={() => open(e)}><Pencil size={13} /></button>
        </li>
      {:else}
        <li class="empty">{q ? 'No matches.' : 'Nothing saved yet. Press + to add a password or an API key.'}</li>
      {/each}
    </ul>
  {/if}
  {#if toast}<div class="toast">{toast}</div>{/if}
</div>

<style>
  .vc { position: relative; width: 380px; padding: 14px; display: flex; flex-direction: column; gap: 9px; max-height: 620px; overflow-y: auto; }
  .locked { display: flex; flex-direction: column; align-items: center; gap: 8px; padding: 14px 6px; text-align: center; }
  .lk { width: 54px; height: 54px; border-radius: 27px; display: grid; place-items: center; background: color-mix(in srgb, var(--accent) 25%, transparent); color: var(--accent); }
  .locked p { margin: 0; color: var(--fg-3); font-size: 12px; line-height: 1.45; }
  .primary { display: flex; align-items: center; gap: 7px; padding: 8px 14px; border-radius: 10px; background: var(--accent); color: white; font-size: 13px; }
  .top { display: flex; gap: 6px; }
  .search { flex: 1; display: flex; align-items: center; gap: 7px; padding: 7px 10px; border-radius: 10px; background: var(--surface); color: var(--fg-3); }
  .search input { flex: 1; background: none; border: 0; outline: none; color: var(--fg); font-size: 13px; }
  .ic { width: 34px; border-radius: 10px; display: grid; place-items: center; background: var(--surface); }
  .ic:hover { background: var(--surface-hover); }
  .seg { display: flex; gap: 4px; padding: 3px; border-radius: 9px; background: var(--surface); }
  .seg button { flex: 1; padding: 5px 0; border-radius: 7px; font-size: 12px; color: var(--fg-2); }
  .seg button.on { background: var(--surface-press); color: var(--fg); }
  ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 2px; }
  li { display: flex; align-items: center; gap: 9px; padding: 6px 6px; border-radius: 10px; }
  li:hover { background: var(--surface-hover); }
  li.empty { color: var(--fg-3); font-size: 12px; }
  .av { width: 30px; height: 30px; border-radius: 9px; display: grid; place-items: center; flex: none; font: 700 13px var(--font-display); color: white; background: hsl(var(--h) 55% 42%); }
  .tx { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .tx strong { font-size: 13px; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .tx small { font-size: 11px; color: var(--fg-3); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .act { width: 28px; height: 28px; border-radius: 8px; display: grid; place-items: center; color: var(--fg-2); flex: none; }
  .act:hover { background: var(--surface-press); color: var(--fg); }
  .act.main { color: var(--accent); }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 11.5px; color: var(--fg-2); }
  label input, textarea { padding: 7px 9px; border-radius: 8px; border: 1px solid var(--border); background: var(--surface); outline: none; font-size: 12.5px; color: var(--fg); resize: none; font-family: inherit; }
  .sec { display: flex; gap: 4px; }
  .sec input { flex: 1; font-family: ui-monospace, Consolas, monospace; }
  .sec button { width: 32px; border-radius: 8px; display: grid; place-items: center; background: var(--surface); }
  .sec button:hover { background: var(--surface-hover); }
  .meter { height: 4px; border-radius: 2px; background: var(--surface-press); overflow: hidden; }
  .meter i { display: block; height: 100%; transition: width 240ms var(--ease-out); }
  .meter .s1 { background: var(--danger); } .meter .s2 { background: #ff9f0a; } .meter .s3 { background: #ffd60a; } .meter .s4 { background: var(--good); }
  .genrow { display: flex; align-items: center; gap: 8px; font-size: 11.5px; color: var(--fg-2); }
  .genrow input[type='range'] { flex: 1; accent-color: var(--accent); }
  .chk { flex-direction: row; align-items: center; gap: 5px; }
  .btns { display: flex; gap: 6px; align-items: center; }
  .btns .sp { flex: 1; }
  .btns button { padding: 6px 13px; border-radius: 8px; background: var(--surface-press); font-size: 12.5px; display: flex; align-items: center; gap: 5px; }
  .btns .primary { background: var(--accent); color: white; }
  .btns .danger:hover { background: var(--danger); color: white; }
  .err { color: var(--danger) !important; }
  .toast { position: sticky; bottom: 0; align-self: center; padding: 6px 12px; border-radius: 999px; background: rgba(20, 20, 24, 0.92); color: white; font-size: 12px; animation: up 180ms var(--ease-out); }
  @keyframes up { from { opacity: 0; transform: translateY(6px); } }
</style>
