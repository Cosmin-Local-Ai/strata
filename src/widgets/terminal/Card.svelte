<script lang="ts">
  // Terminal: type a command, Enter opens a terminal (Windows Terminal when installed) already
  // running it; Ctrl+Enter runs it as administrator. Empty + Enter opens a plain prompt. Recent
  // commands are one click away; the shell and the start folder are remembered.
  import { Clock, FolderOpen, ShieldCheck, SquareTerminal, X } from '@lucide/svelte'
  import { invoke } from '@tauri-apps/api/core'
  import type { CardProps } from '../types'
  let { item, act }: CardProps = $props()

  let cmd = $state('')
  let err = $state('')
  let input = $state<HTMLInputElement>()
  let shell = $state<string>(item.options.shell ?? '')
  let cwd = $state<string>(item.options.cwd ?? '')
  let pwsh = $state(false), wt = $state(false)
  invoke<[boolean, boolean]>('terminal_info').then(([p, w]) => { pwsh = p; wt = w; if (!shell) shell = p ? 'pwsh' : 'powershell' })
  $effect(() => { input?.focus() })

  const KEY = 'terminal:history'
  let history = $state<string[]>((() => { try { return JSON.parse(localStorage.getItem(KEY) ?? '[]') } catch { return [] } })())
  function remember(c: string) {
    if (!c.trim()) return
    history = [c, ...history.filter((h) => h !== c)].slice(0, 12)
    localStorage.setItem(KEY, JSON.stringify(history))
  }
  function forget(c: string) { history = history.filter((h) => h !== c); localStorage.setItem(KEY, JSON.stringify(history)) }

  async function go(c: string, admin: boolean) {
    err = ''
    try {
      await invoke('terminal_run', { command: c, shell: shell || 'powershell', admin, cwd: cwd || null })
      remember(c); cmd = ''
    } catch (e) { err = String(e).includes('1223') || String(e).includes('code 5') ? 'Cancelled.' : String(e) }
  }
  function setShell(s: string) { shell = s; act('set-options', { shell: s }) }
  async function pickDir() {
    const d = await invoke<string | null>('pick_folder', { title: 'Start the terminal in…' })
    if (d) { cwd = d; act('set-options', { cwd: d }) }
  }
</script>

<div class="tc">
  <form class="bar" onsubmit={(e) => { e.preventDefault(); go(cmd, false) }}>
    <span class="pr">{shell === 'cmd' ? '>' : 'PS>'}</span>
    <input bind:this={input} bind:value={cmd} placeholder="Type a command, Enter to run" spellcheck="false" autocomplete="off"
           onkeydown={(e) => { if (e.key === 'Enter' && (e.ctrlKey || e.shiftKey)) { e.preventDefault(); go(cmd, true) } }} />
    <button type="submit" class="run" title="Run (Enter)"><SquareTerminal size={14} /></button>
    <button type="button" class="run adm" title="Run as administrator (Ctrl+Enter)" onclick={() => go(cmd, true)}><ShieldCheck size={14} /></button>
  </form>
  <div class="opts">
    <div class="seg">
      {#if pwsh}<button class:on={shell === 'pwsh'} onclick={() => setShell('pwsh')}>PowerShell 7</button>{/if}
      <button class:on={shell === 'powershell'} onclick={() => setShell('powershell')}>{pwsh ? 'Windows PS' : 'PowerShell'}</button>
      <button class:on={shell === 'cmd'} onclick={() => setShell('cmd')}>CMD</button>
    </div>
    <button class="dir" title={cwd || 'Your user folder'} onclick={pickDir}><FolderOpen size={12} /> {cwd ? cwd.split(/[\\/]/).filter(Boolean).pop() : '~'}</button>
    {#if cwd}<button class="x" aria-label="Start in your user folder" onclick={() => { cwd = ''; act('set-options', { cwd: '' }) }}><X size={11} /></button>{/if}
  </div>
  {#if history.length}
    <ul>
      {#each history as h (h)}
        <li>
          <button class="h" onclick={() => go(h, false)} title="Run again"><Clock size={12} /><code>{h}</code></button>
          <button class="x" aria-label="Forget" onclick={() => forget(h)}><X size={11} /></button>
        </li>
      {/each}
    </ul>
  {/if}
  {#if err}<p class="err">{err}</p>{/if}
  <p class="hint">{wt ? 'Opens a new Windows Terminal tab.' : 'Opens a console window.'} Ctrl+Enter: as administrator.</p>
</div>

<style>
  .tc { width: 400px; padding: 14px; display: flex; flex-direction: column; gap: 9px; }
  .bar { display: flex; align-items: center; gap: 6px; padding: 4px 4px 4px 10px; border-radius: 10px; background: #0c0c10; border: 1px solid var(--border); }
  .bar:focus-within { border-color: var(--accent); }
  .pr { font: 600 12px ui-monospace, Consolas, monospace; color: #4fc3f7; }
  .bar input { flex: 1; min-width: 0; background: none; border: 0; outline: none; color: #e8e8ea; font: 13px ui-monospace, Consolas, monospace; padding: 6px 0; }
  .run { width: 30px; height: 30px; border-radius: 8px; display: grid; place-items: center; background: var(--surface); color: var(--fg-2); transition: background var(--t-fast), color var(--t-fast); }
  .run:hover { background: var(--accent); color: white; }
  .run.adm:hover { background: #ff9f0a; }
  .opts { display: flex; align-items: center; gap: 6px; }
  .seg { display: flex; gap: 2px; padding: 2px; border-radius: 8px; background: var(--surface); }
  .seg button { padding: 4px 8px; border-radius: 6px; font-size: 11.5px; color: var(--fg-2); transition: background var(--t-fast); }
  .seg button:hover { background: var(--surface-hover); }
  .seg button.on { background: var(--surface-press); color: var(--fg); }
  .dir { display: flex; align-items: center; gap: 4px; padding: 4px 8px; border-radius: 7px; font-size: 11.5px; color: var(--fg-2); background: var(--surface); max-width: 150px; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; transition: background var(--t-fast); }
  .dir:hover { background: var(--surface-hover); color: var(--fg); }
  ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 1px; max-height: 260px; overflow-y: auto; }
  li { display: flex; align-items: center; border-radius: 7px; }
  li:hover { background: var(--surface-hover); }
  .h { flex: 1; min-width: 0; display: flex; align-items: center; gap: 7px; padding: 5px 7px; color: var(--fg-3); text-align: left; }
  .h code { font: 12px ui-monospace, Consolas, monospace; color: var(--fg); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .x { width: 22px; height: 22px; border-radius: 6px; display: grid; place-items: center; color: var(--fg-3); flex: none; }
  li .x { opacity: 0; margin-right: 3px; }
  li:hover .x { opacity: 1; }
  .x:hover { background: var(--surface-press); color: var(--fg); }
  .hint { margin: 0; font-size: 11px; color: var(--fg-3); }
  .err { margin: 0; color: var(--danger); font-size: 12px; }
</style>
