<script lang="ts">
  // Vault: logins, API keys, secure notes, cards, Wi-Fi, IDs and small files.
  // Secrets live in Windows Credential Manager (large items and files: DPAPI-encrypted files) and only
  // come to this card when you open an item; copies skip clipboard history and clear after 30 s.
  // Tools: import from Google / Apple / Vivaldi (CSV), export, health report with an opt-in breach
  // check, and an optional master password.
  import { ArrowLeft, Copy, Download, ExternalLink, Eye, EyeOff, FileUp, HeartPulse, KeyRound, Lock, Paperclip, Pencil, Plus, RefreshCw, Search, Settings2, ShieldAlert, ShieldCheck, Trash2, Upload, User } from '@lucide/svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { ipc } from '../../lib/ipc'
  import type { CardProps } from '../types'
  import { KINDS, kindOf, type Field, type Kind } from './kinds'
  let { item, act }: CardProps = $props()

  type Entry = { id: string; kind: Kind; title: string; username: string; url: string; updated: number; fileName: string }
  type Health = { id: string; weak: boolean; reused: number; old: boolean; breached: number | null }
  let unlocked = $state(false)
  let lockMsg = $state('')
  let lockInfo = $state<{ master: boolean; helloToo: boolean }>({ master: false, helloToo: false })
  let master = $state('')
  let entries = $state<Entry[]>([])
  let q = $state('')
  let tab = $state<'all' | Kind>('all')
  let view = $state<'list' | 'edit' | 'tools' | 'health'>('list')
  let toast = $state('')
  let tt: ReturnType<typeof setTimeout> | undefined
  const say = (m: string) => { toast = m; clearTimeout(tt); tt = setTimeout(() => (toast = ''), 3200) }
  let input = $state<HTMLInputElement>()
  const err = (x: unknown) => { const s = String(x); if (s.includes('locked')) { unlocked = false } else say(s) }

  async function refresh() {
    entries = await invoke<Entry[]>('vault_list')
    unlocked = await invoke<boolean>('vault_state')
    lockInfo = await invoke('vault_lock_info')
  }
  async function unlock() {
    lockMsg = ''
    if (lockInfo.master && !master) { lockMsg = ''; return }
    try {
      const mode = await invoke<string>('vault_unlock', { minutes: item.options.minutes ?? 5, password: lockInfo.master ? master : null })
      unlocked = true; master = ''
      if (mode === 'no-hello') say('Windows Hello isn\'t set up, so the vault opened without it')
      queueMicrotask(() => input?.focus())
    } catch (e) {
      const s = String(e)
      lockMsg = s.includes('Not verified') ? 'Not verified. Try again.' : s.includes('master-required') ? 'Enter your master password.' : s
    }
  }
  refresh().then(() => { if (!unlocked && !lockInfo.master) unlock() })
  let forgot = $state(false)
  async function resetMaster() {
    lockMsg = ''
    try {
      await invoke('vault_reset_master', { minutes: item.options.minutes ?? 5 })
      await refresh()
      forgot = false; view = 'tools'
      say('Master password removed. Set a new one below, or keep using Windows Hello.')
    } catch (e) {
      const s = String(e)
      lockMsg = s.includes('no-hello') ? "Windows Hello isn't set up on this PC, so there is no other way to prove it's you. Set up a PIN in Settings → Accounts → Sign-in options, then try again."
        : s.includes('Not verified') ? 'Not verified. Try again.' : s
    }
  }

  const shown = $derived(entries
    .filter((e) => tab === 'all' || e.kind === tab)
    .filter((e) => !q || `${e.title} ${e.username} ${e.url} ${e.fileName}`.toLowerCase().includes(q.trim().toLowerCase()))
    .sort((a, b) => a.title.localeCompare(b.title)))
  const counts = $derived(Object.fromEntries(KINDS.map((k) => [k.id, entries.filter((e) => e.kind === k.id).length])))
  const hue = (s: string) => [...s].reduce((h, c) => (h * 31 + c.charCodeAt(0)) % 360, 7)

  async function copy(e: Entry, field: string, label: string) {
    try { await invoke('vault_copy', { id: e.id, field }); say(`${label} copied · clears in 30 s`) } catch (x) { err(x) }
  }

  // ---- editor
  type Draft = Entry & { secret: string; notes: string; fields: Record<string, string>; changedSecret: boolean }
  let edit = $state<Draft | null>(null)
  let show = $state<Record<string, boolean>>({})
  let genLen = $state(20), genSym = $state(true)
  // Generator options stay hidden until the refresh button is used.
  let genOpen = $state(false)
  $effect(() => { invoke('card_sticky', { on: view !== 'list' }); return () => { invoke('card_sticky', { on: false }) } })
  function blank(kind: Kind): Draft { return { id: '', kind, title: '', username: '', url: '', updated: 0, fileName: '', secret: '', notes: '', fields: {}, changedSecret: true } }
  function startNew(kind: Kind) { edit = blank(kind); show = {}; genOpen = false; view = 'edit' }
  async function open(e: Entry) {
    try {
      const s = await invoke<{ secret: string; notes: string; fields: Record<string, string> }>('vault_reveal', { id: e.id })
      edit = { ...e, secret: s.secret, notes: s.notes, fields: { ...(s.fields ?? {}) }, changedSecret: false }; show = {}; genOpen = false; view = 'edit'
    } catch (x) { err(x) }
  }
  /** Fields an item still has from an older version of its type (e.g. an ID's country or notes). */
  const legacyFields = (d: Draft, known: { key: string }[]) => [
    ...Object.keys(d.fields).filter((key) => d.fields[key] && !known.some((f) => f.key === key)).map((key) => ({ key, label: key[0].toUpperCase() + key.slice(1) })),
    ...(d.notes && !known.some((f) => f.key === 'notes') ? [{ key: 'notes', label: 'Notes', multiline: true }] : []),
  ] as Field[]
  const val = (d: Draft, key: string) => key === 'username' ? d.username : key === 'url' ? d.url : key === 'secret' ? d.secret : key === 'notes' ? d.notes : d.fields[key] ?? ''
  function setVal(d: Draft, key: string, v: string) {
    if (key === 'username') d.username = v
    else if (key === 'url') d.url = v
    else if (key === 'secret') { d.secret = v; d.changedSecret = true }
    else if (key === 'notes') d.notes = v
    else d.fields[key] = v
  }
  async function gen(kind: 'random' | 'words') {
    if (!edit) return
    edit.secret = kind === 'words' ? await invoke<string>('vault_passphrase', { words: 6 }) : await invoke<string>('vault_generate', { length: genLen, symbols: genSym })
    edit.changedSecret = true; show = { ...show, secret: true }
  }
  async function save(): Promise<Entry | null> {
    if (!edit) return null
    if (!edit.title.trim()) { say('Give it a name'); return null }
    const { secret, notes, fields, changedSecret, ...entry } = $state.snapshot(edit)
    // Cards show their last four digits in the list; nothing else of the number leaves the secret.
    if (entry.kind === 'card') { const d = secret.replace(/\D/g, ''); entry.username = d.length >= 4 ? `•••• ${d.slice(-4)}` : '' }
    try {
      const saved = await invoke<Entry>('vault_save', { entry: { ...entry, title: entry.title.trim() }, secret: changedSecret ? secret : null, notes, fields })
      await refresh(); return saved
    } catch (x) { err(x); return null }
  }
  async function saveAndClose() { if (await save()) { edit = null; view = 'list'; say('Saved') } }
  async function del() {
    if (!edit?.id) return
    try { await invoke('vault_delete', { id: edit.id }); edit = null; view = 'list'; await refresh(); say('Deleted') } catch (x) { err(x) }
  }
  async function attach() {
    if (!edit) return
    const path = await invoke<string | null>('pick_file', { save: false, title: 'Add a file to the vault', filters: [['All files', '*.*']], defaultName: null })
    if (!path) return
    const saved = edit.id ? edit : await save()
    if (!saved) return
    try { const e = await invoke<Entry>('vault_attach', { id: saved.id, path }); edit = { ...edit!, id: e.id, fileName: e.fileName }; await refresh(); say('File encrypted into the vault. You can delete the original.') } catch (x) { err(x) }
  }
  async function saveFile(e: { id: string; fileName: string }) {
    const path = await invoke<string | null>('pick_file', { save: true, title: 'Save the file', filters: [['All files', '*.*']], defaultName: e.fileName })
    if (!path) return
    try { await invoke('vault_file_save', { id: e.id, path }); say('Saved') } catch (x) { err(x) }
  }
  async function removeFile() {
    if (!edit?.id) return
    try { await invoke('vault_file_remove', { id: edit.id }); edit.fileName = ''; await refresh() } catch (x) { err(x) }
  }
  const strength = $derived.by(() => {
    const s = edit?.secret ?? ''
    if (!s) return 0
    const sets = [/[a-z]/, /[A-Z]/, /\d/, /[^A-Za-z0-9]/].filter((r) => r.test(s)).length
    const bits = s.length * Math.log2([0, 26, 52, 62, 90][sets] || 26)
    return bits >= 100 ? 4 : bits >= 70 ? 3 : bits >= 45 ? 2 : 1
  })

  // ---- tools: import / export / master password
  let delAfter = $state(true)
  let importing = $state(false)
  async function importCsv() {
    const path = await invoke<string | null>('pick_file', { save: false, title: 'Choose the passwords CSV you exported', filters: [['Password export (CSV)', '*.csv'], ['All files', '*.*']], defaultName: null })
    if (!path) return
    importing = true
    try {
      const r = await invoke<{ imported: number; duplicates: number; skipped: number; source: string }>('vault_import', { path, deleteAfter: delAfter })
      await refresh()
      say(`${r.source}: ${r.imported} imported${r.duplicates ? `, ${r.duplicates} already here` : ''}${r.skipped ? `, ${r.skipped} empty skipped` : ''}${delAfter ? ' · CSV deleted' : ''}`)
    } catch (x) { err(x) } finally { importing = false }
  }
  async function exportTo(format: 'csv' | 'json') {
    const path = await invoke<string | null>('pick_file', { save: true, title: 'Export the vault (unencrypted!)', filters: format === 'csv' ? [['CSV', '*.csv']] : [['JSON', '*.json']], defaultName: format === 'csv' ? 'strata-passwords.csv' : 'strata-vault.json' })
    if (!path) return
    try { const n = await invoke<number>('vault_export', { path, format }); say(`Exported ${format === 'csv' ? 'logins and keys' : `${n} items`}. Delete the file when you're done.`) } catch (x) { err(x) }
  }
  let newMaster = $state(''), newMaster2 = $state(''), helloToo = $state(false)
  async function setMaster(off = false) {
    if (!off && newMaster !== newMaster2) return say('The two passwords don\'t match')
    try {
      await invoke('vault_set_master', { password: off ? null : newMaster, helloToo })
      newMaster = newMaster2 = ''; await refresh()
      say(off ? 'Master password turned off: Windows Hello unlocks the vault' : 'Master password set')
    } catch (x) { err(x) }
  }

  // ---- health
  let health = $state<Health[] | null>(null)
  let checking = $state(false)
  async function runHealth(breaches: boolean) {
    checking = true
    try { health = await invoke<Health[]>('vault_health', { breaches }) } catch (x) { err(x) } finally { checking = false }
  }
  const byId = $derived(Object.fromEntries(entries.map((e) => [e.id, e])))
  const issues = $derived((health ?? []).filter((h) => h.weak || h.reused || h.old || (h.breached ?? 0) > 0))
  const score = $derived(health && health.length ? Math.round(100 * (1 - issues.length / health.length)) : null)
  const onCopyMain = (e: Entry) => kindOf(e.kind).copyMain
</script>

<div class="vc">
  {#if !unlocked}
    <div class="locked">
      <span class="lk"><Lock size={26} /></span>
      <strong>Vault is locked</strong>
      <p>Your items stay encrypted on this PC by Windows (Credential Manager and DPAPI).</p>
      {#if lockInfo.master}
        <form class="mp" onsubmit={(e) => { e.preventDefault(); unlock() }}>
          <input type="password" bind:value={master} placeholder="Master password" autocomplete="off" />
          <button class="primary" type="submit"><ShieldCheck size={15} /> Unlock{lockInfo.helloToo ? ' + Hello' : ''}</button>
        </form>
      {:else}
        <button class="primary" onclick={unlock}><ShieldCheck size={15} /> Unlock with Windows Hello</button>
      {/if}
      {#if lockMsg}<p class="err">{lockMsg}</p>{/if}
      {#if lockInfo.master}
        {#if !forgot}<button class="link" onclick={() => (forgot = true)}>Forgot master password?</button>
        {:else}
          <p class="forgot">Your items are encrypted with your Windows account, so they are safe without it: confirm it's you with Windows Hello (face, fingerprint or PIN) and the master password is removed. You can set a new one right after.</p>
          <div class="fa"><button onclick={() => (forgot = false)}>Cancel</button><button class="primary" onclick={resetMaster}><ShieldCheck size={15} /> Reset with Windows Hello</button></div>
        {/if}
      {/if}
    </div>
  {:else if view === 'edit' && edit}
    {@const k = kindOf(edit.kind)}
    <div class="hd"><button class="ic" aria-label="Back" onclick={() => { edit = null; view = 'list' }}><ArrowLeft size={15} /></button><strong>{edit.id ? 'Edit' : 'New'} {k.name.toLowerCase()}</strong></div>
    {#if !edit.id}
      <div class="kinds">
        {#each KINDS as kd (kd.id)}<button class:on={edit.kind === kd.id} onclick={() => (edit!.kind = kd.id)}><kd.icon size={13} /> {kd.name}</button>{/each}
      </div>
    {/if}
    <label>Name<input bind:value={edit.title} placeholder={edit.kind === 'card' ? 'e.g. Revolut Visa' : edit.kind === 'wifi' ? 'e.g. Home' : 'e.g. GitHub'} /></label>
    {#each [...k.fields, ...legacyFields(edit, k.fields)] as f (f.key)}
      <label><span class="fl">{f.label}{#if edit.id && val(edit, f.key)}<button class="cp" title="Copy {f.label.toLowerCase()}" onclick={(ev) => { ev.preventDefault(); copy(edit!, f.key, f.label) }}><Copy size={12} /></button>{/if}</span>
        {#if f.multiline}
          <textarea class:mono={f.mono} rows={f.key === 'notes' && edit.kind !== 'note' ? 2 : 5} value={val(edit, f.key)} oninput={(e) => setVal(edit!, f.key, e.currentTarget.value)} spellcheck="false"></textarea>
        {:else if f.secret}
          <span class="sec">
            <input class:mono={f.mono} type={show[f.key] ? 'text' : 'password'} value={val(edit, f.key)} placeholder={f.placeholder ?? ''} oninput={(e) => setVal(edit!, f.key, e.currentTarget.value)} spellcheck="false" autocomplete="off" />
            <button aria-label={show[f.key] ? 'Hide' : 'Show'} onclick={() => (show = { ...show, [f.key]: !show[f.key] })}>{#if show[f.key]}<EyeOff size={14} />{:else}<Eye size={14} />{/if}</button>
            {#if f.key === 'secret' && (edit.kind === 'password' || edit.kind === 'wifi')}<button aria-label="Generate" title="Generate a strong password" class:on={genOpen} onclick={() => { genOpen = true; gen('random') }}><RefreshCw size={14} /></button>{/if}
          </span>
        {:else}
          <input value={val(edit, f.key)} placeholder={f.placeholder ?? ''} oninput={(e) => setVal(edit!, f.key, e.currentTarget.value)} spellcheck="false" />
        {/if}
      </label>
      {#if f.key === 'secret' && (edit.kind === 'password' || edit.kind === 'wifi') && genOpen}
        <div class="meter"><i class="s{strength}" style="width:{strength * 25}%"></i></div>
        <div class="genrow"><span>Length {genLen}</span><input type="range" min="12" max="48" bind:value={genLen} /><label class="chk"><input type="checkbox" bind:checked={genSym} /> Symbols</label><button class="link" onclick={() => gen('words')}>Passphrase</button></div>
      {/if}
    {/each}
    {#if edit.kind === 'file' || edit.fileName}
    <div class="file">
      {#if edit.fileName}
        <Paperclip size={14} /><span class="fn">{edit.fileName}</span>
        <button class="link" onclick={() => saveFile(edit!)}>Save a copy</button>
        <button class="link danger" onclick={removeFile}>Remove</button>
      {:else}
        <button class="link" onclick={attach}><Paperclip size={13} /> Choose the file (up to 10 MB, encrypted)</button>
      {/if}
    </div>
    {/if}
    <div class="btns">
      {#if edit.id}<button class="danger" onclick={del}><Trash2 size={13} /> Delete</button>{/if}
      <span class="sp"></span>
      <button onclick={() => { edit = null; view = 'list' }}>Cancel</button>
      <button class="primary" onclick={saveAndClose}>Save</button>
    </div>
  {:else if view === 'tools'}
    <div class="hd"><button class="ic" aria-label="Back" onclick={() => (view = 'list')}><ArrowLeft size={15} /></button><strong>Import, export & security</strong></div>
    <section>
      <h4><FileUp size={14} /> Import passwords</h4>
      <p>Export a CSV from where your passwords are now, then choose it here. Logins already in the vault are skipped.</p>
      <ul class="how">
        <li><b>Google</b> (Android, Chrome): <a href="#g" onclick={(e) => { e.preventDefault(); ipc.launch('https://passwords.google.com/options') }}>passwords.google.com</a> → Settings → Export passwords.</li>
        <li><b>Apple</b> (iPhone, iCloud Keychain): on a Mac, Passwords app → File → Export All Passwords; on Windows, the iCloud Passwords app → ⋯ → Export Passwords. iPhone alone can't export to a file.</li>
        <li><b>Vivaldi</b>: open <code>vivaldi://password-manager/settings</code> → Export passwords.</li>
      </ul>
      <label class="chk"><input type="checkbox" bind:checked={delAfter} /> Delete the CSV after importing (it holds your passwords in plain text)</label>
      <button class="primary" disabled={importing} onclick={importCsv}><Upload size={14} /> {importing ? 'Importing…' : 'Choose CSV…'}</button>
    </section>
    <section>
      <h4><Download size={14} /> Export</h4>
      <p>Exports are <b>not encrypted</b>. The CSV (logins and API keys) imports into Google, Apple, Vivaldi, Bitwarden and 1Password; JSON keeps every item and field.</p>
      <div class="row"><button onclick={() => exportTo('csv')}>Export CSV…</button><button onclick={() => exportTo('json')}>Export JSON…</button></div>
    </section>
    <section>
      <h4><KeyRound size={14} /> Master password</h4>
      {#if lockInfo.master}
        <p>Set. The vault asks for it when it opens{lockInfo.helloToo ? ', followed by Windows Hello' : ''}. Change it or turn it off below (only while unlocked).</p>
      {:else}
        <p>Optional. Without one, Windows Hello unlocks the vault. Strata keeps only a slow hash of it; it can't be recovered, but turning it off later doesn't lose any items.</p>
      {/if}
      <input type="password" bind:value={newMaster} placeholder={lockInfo.master ? 'New master password' : 'Master password (8+ characters)'} autocomplete="new-password" />
      <input type="password" bind:value={newMaster2} placeholder="Repeat it" autocomplete="new-password" />
      <label class="chk"><input type="checkbox" bind:checked={helloToo} /> Also ask for Windows Hello</label>
      <label class="row-l">Ask again after
        <select value={String(item.options.minutes ?? 5)} onchange={(e) => act('set-options', { minutes: +e.currentTarget.value })}>
          {#each [1, 5, 15, 30, 60] as m}<option value={String(m)}>{m} min</option>{/each}
        </select>
      </label>
      <div class="row">
        <button class="primary" disabled={newMaster.length < 8} onclick={() => setMaster(false)}>{lockInfo.master ? 'Change' : 'Set'} master password</button>
        {#if lockInfo.master}<button class="danger" onclick={() => setMaster(true)}>Turn off</button>{/if}
      </div>
    </section>
  {:else if view === 'health'}
    <div class="hd"><button class="ic" aria-label="Back" onclick={() => (view = 'list')}><ArrowLeft size={15} /></button><strong>Vault health</strong></div>
    {#if health === null}
      <p class="muted">Checks your logins for weak, reused and old passwords. The breach check asks Have I Been Pwned using k-anonymity: only the first 5 characters of each password's SHA-1 hash leave this PC, never the password.</p>
      <div class="row"><button onclick={() => runHealth(false)} disabled={checking}>Check on this PC</button><button class="primary" onclick={() => runHealth(true)} disabled={checking}><ShieldAlert size={14} /> {checking ? 'Checking…' : 'Check, including breaches'}</button></div>
    {:else}
      <div class="score"><span class="big" class:good={(score ?? 0) >= 80} class:bad={(score ?? 100) < 50}>{score ?? '–'}</span><span>/ 100<br /><small>{health.length} logins, {issues.length} need attention</small></span></div>
      <div class="chips">
        <span>Weak {health.filter((h) => h.weak).length}</span>
        <span>Reused {health.filter((h) => h.reused).length}</span>
        <span>Older than a year {health.filter((h) => h.old).length}</span>
        <span>{health.some((h) => h.breached !== null) ? `Breached ${health.filter((h) => (h.breached ?? 0) > 0).length}` : 'Breaches not checked'}</span>
      </div>
      <ul>
        {#each issues as h (h.id)}
          {@const e = byId[h.id]}
          {#if e}
            <li>
              <span class="av" style="--h:{hue(e.title)}">{e.title.slice(0, 1).toUpperCase()}</span>
              <span class="tx"><strong>{e.title}</strong><small class="warn">{[(h.breached ?? 0) > 0 ? `in ${h.breached!.toLocaleString()} breaches` : '', h.reused ? `used ${h.reused + 1}×` : '', h.weak ? 'weak' : '', h.old ? 'old' : ''].filter(Boolean).join(' · ')}</small></span>
              <button class="act" title="Change it" onclick={() => open(e)}><Pencil size={13} /></button>
            </li>
          {/if}
        {:else}
          <li class="empty">Nothing to fix. 👍</li>
        {/each}
      </ul>
      {#if !health.some((h) => h.breached !== null)}<button class="link" onclick={() => runHealth(true)} disabled={checking}>{checking ? 'Checking…' : 'Also check for breaches'}</button>{/if}
    {/if}
  {:else}
    <div class="top">
      <label class="search"><Search size={14} /><input bind:this={input} bind:value={q} placeholder="Search {entries.length} items" spellcheck="false" /></label>
      <button class="ic" aria-label="Add" title="Add" onclick={() => startNew(tab === 'all' ? 'password' : tab)}><Plus size={16} /></button>
      <button class="ic" aria-label="Health" title="Vault health" onclick={() => (view = 'health')}><HeartPulse size={15} /></button>
      <button class="ic" aria-label="Tools" title="Import, export & master password" onclick={() => (view = 'tools')}><Settings2 size={15} /></button>
      <button class="ic" aria-label="Lock" title="Lock now" onclick={async () => { await invoke('vault_lock'); unlocked = false }}><Lock size={15} /></button>
    </div>
    <div class="tabs">
      <button class:on={tab === 'all'} onclick={() => (tab = 'all')}>All</button>
      {#each KINDS.filter((k) => counts[k.id] > 0 || tab === k.id) as kd (kd.id)}<button class:on={tab === kd.id} onclick={() => (tab = kd.id)}>{kd.plural} <small>{counts[kd.id]}</small></button>{/each}
    </div>
    <ul>
      {#each shown as e (e.id)}
        {@const kd = kindOf(e.kind)}
        <li>
          <span class="av" style="--h:{hue(e.title)}">{#if e.kind === 'password'}{e.title.slice(0, 1).toUpperCase()}{:else}<kd.icon size={15} />{/if}</span>
          <span class="tx"><strong>{e.title}</strong><small>{e.username || e.fileName || kd.name}</small></span>
          {#if e.username && (e.kind === 'password' || e.kind === 'wifi')}<button class="act" title="Copy {e.kind === 'wifi' ? 'network name' : 'username'}" onclick={() => copy(e, 'username', e.kind === 'wifi' ? 'Network name' : 'Username')}><User size={14} /></button>{/if}
          {#if onCopyMain(e)}<button class="act main" title="Copy {onCopyMain(e)?.toLowerCase()}" onclick={() => copy(e, 'secret', onCopyMain(e)!)}><Copy size={14} /></button>{/if}
          {#if e.fileName}<button class="act" title="Save {e.fileName}" onclick={() => saveFile(e)}><Download size={14} /></button>{/if}
          {#if e.url}<button class="act" title="Open website" onclick={() => ipc.launch(e.url)}><ExternalLink size={14} /></button>{/if}
          <button class="act" title="Open" onclick={() => open(e)}><Pencil size={13} /></button>
        </li>
      {:else}
        <li class="empty">{q ? 'No matches.' : entries.length ? 'Nothing of this type yet.' : 'Nothing saved yet. Press + to add an item, or ⚙ to import from Google, Apple or Vivaldi.'}</li>
      {/each}
    </ul>
  {/if}
  {#if toast}<div class="toast">{toast}</div>{/if}
</div>

<style>
  .vc { position: relative; width: 400px; padding: 14px; display: flex; flex-direction: column; gap: 9px; max-height: 640px; overflow-y: auto; }
  .locked { display: flex; flex-direction: column; align-items: center; gap: 8px; padding: 14px 6px; text-align: center; }
  .lk { width: 54px; height: 54px; border-radius: 27px; display: grid; place-items: center; background: color-mix(in srgb, var(--accent) 25%, transparent); color: var(--accent); }
  .locked p { margin: 0; color: var(--fg-3); font-size: 12px; line-height: 1.45; }
  .mp { display: flex; flex-direction: column; gap: 8px; width: 100%; align-items: center; }
  .mp input { width: 80%; }
  .primary { display: flex; align-items: center; justify-content: center; gap: 7px; padding: 8px 14px; border-radius: 10px; background: var(--accent); color: white; font-size: 13px; }
  .primary:disabled { opacity: 0.5; }
  .locked .link { font-size: 12px; color: var(--fg-3); padding: 2px 6px; border-radius: 6px; }
  .locked .link:hover { color: var(--fg); background: var(--surface-hover); }
  .locked .fa { display: flex; gap: 8px; }
  .locked .fa > button:not(.primary) { padding: 8px 14px; border-radius: 10px; background: var(--surface-press); font-size: 13px; }
  .locked .fa > button:not(.primary):hover { background: var(--surface-hover); }
  .hd { display: flex; align-items: center; gap: 8px; }
  .hd strong { font-size: 13.5px; }
  .top { display: flex; gap: 6px; }
  .search { flex: 1; display: flex; align-items: center; gap: 7px; padding: 7px 10px; border-radius: 10px; background: var(--surface); color: var(--fg-3); flex-direction: row; }
  .search input { flex: 1; background: none; border: 0; outline: none; color: var(--fg); font-size: 13px; padding: 0; }
  .ic { width: 34px; height: 34px; flex: none; border-radius: 10px; display: grid; place-items: center; background: var(--surface); }
  .ic:hover { background: var(--surface-hover); }
  .tabs, .kinds { display: flex; flex-wrap: wrap; gap: 4px; }
  .tabs button, .kinds button { padding: 4px 9px; border-radius: 999px; font-size: 11.5px; color: var(--fg-2); background: var(--surface); display: flex; align-items: center; gap: 5px; }
  .tabs button small { color: var(--fg-3); }
  .tabs button.on, .kinds button.on { background: var(--accent); color: white; }
  .tabs button.on small { color: rgba(255,255,255,.75); }
  ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 2px; }
  li { display: flex; align-items: center; gap: 9px; padding: 6px 6px; border-radius: 10px; }
  li:hover { background: var(--surface-hover); }
  li.empty { color: var(--fg-3); font-size: 12px; }
  .av { width: 30px; height: 30px; border-radius: 9px; display: grid; place-items: center; flex: none; font: 700 13px var(--font-display); color: white; background: hsl(var(--h) 55% 42%); }
  .tx { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .tx strong { font-size: 13px; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .tx small { font-size: 11px; color: var(--fg-3); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .tx small.warn { color: #ff9f0a; }
  .act { width: 28px; height: 28px; border-radius: 8px; display: grid; place-items: center; color: var(--fg-2); flex: none; }
  .act:hover { background: var(--surface-press); color: var(--fg); }
  .act.main { color: var(--accent); }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 11.5px; color: var(--fg-2); }
  input, textarea { padding: 7px 9px; border-radius: 8px; border: 1px solid var(--border); background: var(--surface); outline: none; font-size: 12.5px; color: var(--fg); resize: none; font-family: inherit; }
  .mono { font-family: ui-monospace, Consolas, monospace; }
  .fl { display: flex; align-items: center; gap: 6px; min-height: 18px; }
  .cp { display: grid; place-items: center; width: 20px; height: 18px; border-radius: 5px; color: var(--fg-3); opacity: 0.7; }
  .cp:hover { color: var(--fg); background: var(--surface-hover); opacity: 1; }
  .sec { display: flex; gap: 4px; }
  .sec input { flex: 1; min-width: 0; }
  .sec button { width: 32px; flex: none; border-radius: 8px; display: grid; place-items: center; background: var(--surface); }
  .sec button:hover { background: var(--surface-hover); }
  .sec button.on { color: var(--accent); }
  .meter { height: 4px; border-radius: 2px; background: var(--surface-press); overflow: hidden; }
  .meter i { display: block; height: 100%; transition: width 240ms var(--ease-out); }
  .meter .s1 { background: var(--danger); } .meter .s2 { background: #ff9f0a; } .meter .s3 { background: #ffd60a; } .meter .s4 { background: var(--good); }
  .genrow { display: flex; align-items: center; gap: 8px; font-size: 11.5px; color: var(--fg-2); }
  .genrow input[type='range'] { flex: 1; accent-color: var(--accent); padding: 0; border: 0; }
  .chk { flex-direction: row; align-items: center; gap: 6px; }
  .chk input { padding: 0; }
  .row-l { flex-direction: row; align-items: center; gap: 8px; }
  .row-l select { padding: 4px 8px; border-radius: 7px; border: 1px solid var(--border); background: var(--surface); color: var(--fg); font-size: 12px; }
  .file { display: flex; align-items: center; gap: 8px; font-size: 12px; color: var(--fg-2); }
  .fn { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--fg); }
  .link { color: var(--accent); font-size: 12px; display: inline-flex; align-items: center; gap: 5px; padding: 2px 0; }
  .link.danger { color: var(--danger); }
  .btns, .row { display: flex; gap: 6px; align-items: center; }
  .btns .sp { flex: 1; }
  .btns button, .row button { padding: 6px 13px; border-radius: 8px; background: var(--surface-press); font-size: 12.5px; display: flex; align-items: center; gap: 5px; }
  .btns .primary, .row .primary { background: var(--accent); color: white; }
  .btns .danger:hover, .row .danger:hover { background: var(--danger); color: white; }
  section { display: flex; flex-direction: column; gap: 7px; padding: 10px; border-radius: 12px; background: var(--surface); }
  section h4 { margin: 0; font-size: 12.5px; display: flex; align-items: center; gap: 6px; }
  section p, .muted { margin: 0; font-size: 11.5px; color: var(--fg-3); line-height: 1.45; }
  .how { gap: 4px; }
  .how li { display: block; padding: 0; font-size: 11.5px; color: var(--fg-2); line-height: 1.45; }
  .how li:hover { background: none; }
  .how a { color: var(--accent); }
  code { font-size: 11px; background: var(--surface-press); padding: 1px 4px; border-radius: 4px; }
  .score { display: flex; align-items: center; gap: 10px; font-size: 12px; color: var(--fg-2); }
  .score .big { font: 700 34px var(--font-display); color: #ff9f0a; }
  .score .big.good { color: var(--good); } .score .big.bad { color: var(--danger); }
  .chips { display: flex; flex-wrap: wrap; gap: 4px; }
  .chips span { padding: 3px 8px; border-radius: 999px; background: var(--surface); font-size: 11px; color: var(--fg-2); }
  .err { color: var(--danger) !important; }
  .toast { position: sticky; bottom: 0; align-self: center; padding: 6px 12px; border-radius: 999px; background: rgba(20, 20, 24, 0.92); color: white; font-size: 12px; animation: up 180ms var(--ease-out); text-align: center; }
  @keyframes up { from { opacity: 0; transform: translateY(6px); } }
</style>
