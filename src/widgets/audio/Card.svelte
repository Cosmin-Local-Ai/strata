<script lang="ts">
  // Audio Output. Every output device has its own volume and mute (with virtual mixers such as
  // SteelSeries Sonar, apps play into a virtual device, so the Windows default isn't what you hear);
  // a dot shows which devices are playing right now. Below: per-app mixer across all devices.
  import { Settings2, Volume1, Volume2, VolumeX } from '@lucide/svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { iconUrl, ipc } from '../../lib/ipc'
  import type { CardProps } from '../types'
  let { }: CardProps = $props()

  type Dev = { id: string; name: string; default: boolean; volume: number; muted: boolean; active: boolean }
  type Session = { pid: number; name: string; exe: string; volume: number; muted: boolean }
  let devices = $state<Dev[]>([])
  let sessions = $state<Session[]>([])
  let err = $state('')
  let dragging = $state(false)
  async function load() {
    if (dragging) return
    ;[devices, sessions] = await Promise.all([invoke<Dev[]>('audio_devices'), invoke<Session[]>('audio_sessions')])
  }
  load()
  $effect(() => { const t = setInterval(load, 1500); return () => clearInterval(t) })
  // Playing devices first, then the default, then the rest.
  const sorted = $derived([...devices].sort((a, b) => Number(b.active) - Number(a.active) || Number(b.default) - Number(a.default)))

  function setDev(d: Dev, patch: { volume?: number; muted?: boolean }) {
    Object.assign(d, patch); devices = [...devices]
    invoke('audio_device_set', { id: d.id, ...patch }).catch((x) => (err = String(x)))
  }
  async function makeDefault(d: Dev) {
    err = ''
    try { await invoke('audio_set_default', { id: d.id }); load() } catch (x) { err = `Couldn't switch: ${x}` }
  }
  function app(s: Session, patch: { volume?: number; muted?: boolean }) {
    Object.assign(s, patch); sessions = [...sessions]
    invoke('audio_session_set', { pid: s.pid, ...patch }).catch((x) => (err = String(x)))
  }
</script>

<div class="ac">
  <div class="head">Outputs</div>
  {#each sorted as d (d.id)}
    <div class="dev" class:def={d.default}>
      <div class="dtop">
        <span class="dot" class:on={d.active} title={d.active ? 'Playing now' : 'Silent'}></span>
        <span class="nm" title={d.name}>{d.name}</span>
        {#if d.default}<span class="tag">Default</span>{:else}<button class="mk" onclick={() => makeDefault(d)}>Make default</button>{/if}
      </div>
      <div class="drow">
        <button class="mute" class:on={d.muted} onclick={() => setDev(d, { muted: !d.muted })} aria-label={d.muted ? 'Unmute' : 'Mute'}>
          {#if d.muted}<VolumeX size={14} />{:else if d.volume < 50}<Volume1 size={14} />{:else}<Volume2 size={14} />{/if}
        </button>
        <input type="range" min="0" max="100" value={d.volume}
               onpointerdown={() => (dragging = true)} onpointerup={() => (dragging = false)}
               oninput={(e) => setDev(d, { volume: +e.currentTarget.value })} />
        <span class="pct">{d.volume}%</span>
      </div>
    </div>
  {/each}

  {#if sessions.length}
    <div class="head">Apps</div>
    {#each sessions as s (s.pid)}
      <div class="drow app">
        <button class="mute" class:on={s.muted} onclick={() => app(s, { muted: !s.muted })} title={s.muted ? 'Unmute' : 'Mute'}>
          {#if s.exe}<img src={iconUrl(s.exe, 32)} alt="" />{:else}<Volume2 size={14} />{/if}
        </button>
        <span class="an" title={s.name}>{s.name}</span>
        <input type="range" min="0" max="100" value={s.volume}
               onpointerdown={() => (dragging = true)} onpointerup={() => (dragging = false)}
               oninput={(e) => app(s, { volume: +e.currentTarget.value })} />
      </div>
    {/each}
  {/if}
  {#if err}<p class="err">{err}</p>{/if}
  <button class="link" onclick={() => ipc.launch('ms-settings:sound')}><Settings2 size={13} /> Sound settings</button>
</div>

<style>
  .ac { width: 340px; padding: 14px; display: flex; flex-direction: column; gap: 8px; max-height: 620px; overflow-y: auto; }
  .head { font-size: 11px; font-weight: 600; color: var(--fg-3); text-transform: uppercase; letter-spacing: .04em; margin-top: 4px; }
  .dev { padding: 8px 10px; border-radius: 10px; background: var(--surface); display: flex; flex-direction: column; gap: 6px; }
  .dev.def { outline: 1px solid color-mix(in srgb, var(--accent) 60%, transparent); }
  .dtop { display: flex; align-items: center; gap: 7px; font-size: 12.5px; }
  .dot { width: 7px; height: 7px; border-radius: 4px; background: var(--surface-press); flex: none; }
  .dot.on { background: var(--good); box-shadow: 0 0 6px var(--good); }
  .nm { flex: 1; min-width: 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .tag { font-size: 10.5px; color: var(--accent); font-weight: 600; }
  .mk { font-size: 11px; color: var(--fg-2); padding: 2px 8px; border-radius: 6px; background: var(--surface-press); }
  .mk:hover { color: var(--fg); background: var(--accent); }
  .drow { display: grid; grid-template-columns: 28px 1fr 38px; align-items: center; gap: 8px; }
  .drow.app { grid-template-columns: 28px 90px 1fr; padding: 2px 2px; }
  .mute { width: 28px; height: 28px; border-radius: 8px; display: grid; place-items: center; background: var(--surface-press); }
  .mute.on { background: var(--danger); color: white; }
  .mute img { width: 17px; height: 17px; }
  input[type='range'] { width: 100%; accent-color: var(--accent); }
  .pct { text-align: right; font-size: 12px; color: var(--fg-2); font-variant-numeric: tabular-nums; }
  .an { font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .link { display: flex; align-items: center; gap: 6px; color: var(--fg-3); font-size: 12px; align-self: flex-start; margin-top: 4px; }
  .link:hover { color: var(--fg); }
  .err { color: var(--danger); font-size: 12px; margin: 0; }
</style>
