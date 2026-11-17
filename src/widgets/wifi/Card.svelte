<script lang="ts">
  // Network card: what you're connected to, whether Windows sees real internet, and (on PCs with
  // Wi-Fi) nearby networks you can join from here. Windows settings is a small link, not the default.
  import { Cable, Check, Globe, Lock, RefreshCw, Settings2, Wifi, WifiOff } from '@lucide/svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { ipc } from '../../lib/ipc'
  import type { CardProps } from '../types'
  let { }: CardProps = $props()

  type Info = { kind: string; adapter: string; ipv4: string; linkMbps: number; internet: boolean; wifi: { adapter: boolean; connected: boolean; ssid: string; signal: number } }
  type Net = { ssid: string; signal: number; secured: boolean; connected: boolean; saved: boolean }
  let info = $state<Info | null>(null)
  let nets = $state<Net[]>([])
  let scanning = $state(false)
  let msg = $state('')

  async function load() {
    info = await invoke<Info>('net_info')
    if (info.wifi.adapter) scan()
  }
  async function scan() {
    scanning = true
    nets = await invoke<Net[]>('wifi_networks')
    scanning = false
  }
  async function join(n: Net) {
    msg = ''
    if (!n.saved) { ipc.launch('ms-availablenetworks:'); return } // Windows asks for the password
    try { await invoke('wifi_connect', { ssid: n.ssid }); msg = `Connecting to ${n.ssid}…`; setTimeout(load, 3000) } catch (e) { msg = String(e) }
  }
  load()
  const speed = (m: number) => (m >= 1000 ? `${(m / 1000).toFixed(m % 1000 ? 1 : 0)} Gbps` : `${m} Mbps`)
</script>

<div class="nc">
  {#if info}
    <div class="conn">
      <span class="ic" class:ok={info.internet}>
        {#if info.kind === 'ethernet'}<Cable size={20} />{:else if info.kind === 'wifi'}<Wifi size={20} />{:else}<WifiOff size={20} />{/if}
      </span>
      <div>
        <div class="title">{info.kind === 'ethernet' ? 'Ethernet' : info.kind === 'wifi' ? info.wifi.ssid || 'Wi-Fi' : 'Not connected'}</div>
        <div class="sub">
          {#if info.kind !== 'none'}<Globe size={11} /> {info.internet ? 'Internet' : 'No internet access'}{/if}
        </div>
      </div>
    </div>
    {#if info.kind !== 'none'}
      <dl>
        <dt>Adapter</dt><dd>{info.adapter}</dd>
        {#if info.ipv4}<dt>IPv4</dt><dd>{info.ipv4}</dd>{/if}
        {#if info.linkMbps}<dt>Link speed</dt><dd>{speed(info.linkMbps)}</dd>{/if}
        {#if info.kind === 'wifi'}<dt>Signal</dt><dd>{info.wifi.signal}%</dd>{/if}
      </dl>
    {/if}
    {#if info.wifi.adapter}
      <div class="head">Wi-Fi networks <button class="icon" onclick={scan} disabled={scanning} aria-label="Scan"><RefreshCw size={13} class={scanning ? 'spin' : ''} /></button></div>
      <ul>
        {#each nets as n (n.ssid)}
          <li><button onclick={() => join(n)} disabled={n.connected}>
            <Wifi size={15} style="opacity:{0.35 + n.signal / 150}" />
            <span class="ssid">{n.ssid}</span>
            {#if n.secured}<Lock size={11} />{/if}
            {#if n.connected}<Check size={14} />{:else if n.saved}<span class="tag">Saved</span>{/if}
          </button></li>
        {:else}<li class="muted">{scanning ? 'Scanning…' : 'No networks found'}</li>{/each}
      </ul>
    {:else}
      <p class="muted">This PC has no Wi-Fi adapter.</p>
    {/if}
    {#if msg}<p class="msg">{msg}</p>{/if}
  {:else}
    <p class="muted">Reading network…</p>
  {/if}
  <button class="link" onclick={() => ipc.launch('ms-settings:network')}><Settings2 size={13} /> Network settings</button>
</div>

<style>
  .nc { width: 300px; padding: 14px; display: flex; flex-direction: column; gap: 10px; }
  .conn { display: flex; align-items: center; gap: 12px; }
  .ic { width: 40px; height: 40px; border-radius: 20px; display: grid; place-items: center; background: var(--surface-press); }
  .ic.ok { background: var(--accent); color: white; }
  .title { font: 600 15px var(--font-display); }
  .sub { display: flex; align-items: center; gap: 4px; font-size: 12px; color: var(--fg-2); }
  dl { display: grid; grid-template-columns: auto 1fr; gap: 4px 12px; margin: 0; padding: 10px 12px; border-radius: 10px; background: var(--surface); font-size: 12px; }
  dt { color: var(--fg-3); } dd { margin: 0; text-align: right; font-variant-numeric: tabular-nums; }
  .head { display: flex; align-items: center; justify-content: space-between; font-size: 11px; font-weight: 600; color: var(--fg-3); text-transform: uppercase; letter-spacing: .04em; }
  .icon { padding: 3px; border-radius: 6px; color: var(--fg-2); }
  .icon:hover { background: var(--surface-hover); }
  :global(.spin) { animation: spin 0.9s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
  ul { list-style: none; margin: 0; padding: 0; max-height: 220px; overflow-y: auto; }
  li button { width: 100%; display: flex; align-items: center; gap: 8px; padding: 7px 8px; border-radius: 8px; font-size: 12.5px; text-align: left; }
  li button:hover:not(:disabled) { background: var(--surface-hover); }
  .ssid { flex: 1; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .tag { font-size: 10.5px; color: var(--fg-3); }
  .muted { color: var(--fg-2); margin: 0; font-size: 12px; }
  .msg { margin: 0; font-size: 12px; color: var(--fg-2); }
  .link { display: flex; align-items: center; gap: 6px; color: var(--fg-3); font-size: 12px; align-self: flex-start; }
  .link:hover { color: var(--fg); }
</style>
