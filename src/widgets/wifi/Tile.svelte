<script lang="ts">
  // Wi-Fi: SSID strength. Polls every 15 s while visible.
  import { EthernetPort, Wifi, WifiHigh, WifiLow, WifiOff, WifiZero } from '@lucide/svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { poll } from '../../lib/poll'
  import type { TileProps } from '../types'
  let { }: TileProps = $props()
  type W = { adapter: boolean; connected: boolean; ssid: string; signal: number }
  let w = $state<W | null>(null)
  $effect(() => poll('wifi', 15_000, () => invoke<W>('wifi_state'), (v) => (w = v)))
  // Wired and online: show Ethernet.
  const wired = $derived(!w?.connected && navigator.onLine)
  const Icon = $derived(wired ? EthernetPort : !w?.connected ? WifiOff : w.signal > 75 ? Wifi : w.signal > 50 ? WifiHigh : w.signal > 25 ? WifiLow : WifiZero)
</script>

<span class="wf" class:off={!w?.connected} class:wired title={w?.connected ? `${w.ssid} · ${w.signal}%` : wired ? 'Ethernet' : w?.adapter === false ? 'No Wi-Fi adapter' : 'Not connected'}>
  <Icon size={18} strokeWidth={1.9} />
  {#if w?.connected}<span class="v">{w.signal}%</span>{/if}
</span>

<style>
  .wf { display: flex; align-items: center; gap: 6px; padding: 0 12px; height: 100%; }
  .v { font-size: 12.5px; font-weight: 600; font-variant-numeric: tabular-nums; }
  .off:not(.wired) { opacity: 0.5; }
</style>
