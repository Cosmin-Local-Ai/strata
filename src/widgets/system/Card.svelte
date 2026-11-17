<script lang="ts">
  // Details for the system tiles: top processes (CPU/RAM), live network chart, drives.
  import { ArrowDown, ArrowUp, ExternalLink, HardDrive, Usb } from '@lucide/svelte'
  import { invoke } from '@tauri-apps/api/core'
  import type { CardProps } from '../types'
  import Ring from './Ring.svelte'
  import Spark from '../../lib/ui/Spark.svelte'
  import { gb, rate, spark, stats, useStats } from './stats.svelte'
  let { item, act }: CardProps = $props()
  let pickedDrive = $state<string | undefined>(item.options.drive)
  const id = item.widget
  $effect(() => useStats({ procs: id !== 'network' && id !== 'storage', drives: id === 'storage' }))
  const s = $derived(stats.value)
  const isRam = id === 'ram' || id === 'top-ram'
  const list = $derived(isRam ? s?.topRam ?? [] : s?.topCpu ?? [])
  const max = $derived(Math.max(...list.map((p) => p.value), 1))
</script>

<div class="sc">
  {#if id === 'network'}
    <div class="hdr"><span class="d"><ArrowDown size={14} /> {rate(s?.netDown ?? 0)}</span><span class="u"><ArrowUp size={14} /> {rate(s?.netUp ?? 0)}</span></div>
    <svg viewBox="0 0 300 110" class="chart">
      <path class="area" d={spark(stats.down, 300, 106) + (stats.down.length > 1 ? ' L300,110 L0,110 Z' : '')} />
      <path class="line" d={spark(stats.down, 300, 106)} />
      <path class="line up" d={spark(stats.up, 300, 106, Math.max(...stats.down, 1))} />
    </svg>
    <p class="muted">Last 2 minutes · all adapters except virtual ones</p>
  {:else if id === 'storage' || id === 'drives'}
    {#if id === 'storage'}<p class="muted">Click a drive to show it on the tile.</p>{/if}
    {#each s?.drives ?? [] as d (d.mount)}
      {@const used = 1 - d.free / Math.max(1, d.total)}
      <button class="drive" class:picked={id === 'storage' && (pickedDrive ?? s?.drives.find((x) => x.mount.toUpperCase().startsWith('C'))?.mount) === d.mount} onclick={() => { if (id === 'storage') { pickedDrive = d.mount; act('set-options', { drive: d.mount }) } }}>
        {#if d.removable}<Usb size={16} />{:else}<HardDrive size={16} />{/if}
        <div class="dinfo">
          <div class="dl"><strong>{d.mount.replace(/\\$/, '')} {d.name}</strong><span class="muted">{gb(d.free)} free of {gb(d.total)}</span></div>
          <div class="bar"><span style="width:{used * 100}%" class:full={used > 0.9}></span></div>
        </div>
      </button>
    {/each}
    <button class="link" onclick={() => invoke('launch', { target: 'ms-settings:storagesense' })}><ExternalLink size={12} /> Storage settings</button>
  {:else}
    <div class="hdr">
      {#if isRam}
        <Ring value={s ? s.ramUsed / s.ramTotal : 0} size={44} stroke={4}  />
        <div><strong>{s ? gb(s.ramUsed) : '…'}</strong> <span class="muted">of {s ? gb(s.ramTotal) : ''} in use</span></div>
      {:else}
        <Ring value={(s?.cpu ?? 0) / 100} size={44} stroke={4} />
        <div><strong>{Math.round(s?.cpu ?? 0)}%</strong> <span class="muted">CPU · {s?.cores ?? ''} logical processors</span></div>
      {/if}
    </div>
    {#if !isRam}<Spark values={stats.cpuHist} width={300} height={64} min={0} max={Math.max(20, Math.min(100, Math.max(...stats.cpuHist, 0) * 1.3))} /><p class="muted">Last 2 minutes</p>{/if}
    <ul>
      {#each list as p (p.name)}
        <li><span class="pn">{p.name}</span><span class="pb"><span style="width:{(p.value / max) * 100}%"></span></span><span class="pv">{isRam ? gb(p.value) : `${p.value.toFixed(1)}%`}</span></li>
      {:else}<li class="muted">Measuring…</li>{/each}
    </ul>
    <button class="link" onclick={() => invoke('sys_action', { action: 'task-manager' })}><ExternalLink size={12} /> Task Manager</button>
  {/if}
</div>

<style>
  .sc { width: 320px; padding: 14px; display: flex; flex-direction: column; gap: 10px; }
  .hdr { display: flex; align-items: center; gap: 12px; }
  .hdr strong { font: 600 18px var(--font-display); }
  .hdr .d, .hdr .u { display: flex; align-items: center; gap: 4px; font-weight: 600; font-variant-numeric: tabular-nums; }
  .hdr .u { color: #64d2ff; }
  .chart { width: 100%; height: 110px; }
  .chart.small { height: 50px; }
  .chart .line { fill: none; stroke: var(--accent); stroke-width: 1.8; stroke-linejoin: round; }
  .chart .line.up { stroke: #64d2ff; }
  .chart .area { fill: var(--accent); opacity: 0.15; }
  ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 5px; }
  li { display: grid; grid-template-columns: 1fr 80px 60px; align-items: center; gap: 8px; font-size: 12.5px; }
  .pn { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .pb { height: 5px; border-radius: 3px; background: var(--surface-press); overflow: hidden; }
  .pb span { display: block; height: 100%; background: var(--accent); }
  .pv { text-align: right; color: var(--fg-2); font-variant-numeric: tabular-nums; }
  .drive { display: flex; align-items: center; gap: 10px; }
  .dinfo { flex: 1; display: flex; flex-direction: column; gap: 5px; }
  .dl { display: flex; justify-content: space-between; gap: 8px; font-size: 12.5px; }
  .bar { height: 6px; border-radius: 3px; background: var(--surface-press); overflow: hidden; }
  .bar span { display: block; height: 100%; background: var(--accent); }
  .bar span.full { background: var(--danger); }
  .muted { color: var(--fg-2); font-size: 11.5px; margin: 0; }
  .link { display: flex; align-items: center; gap: 6px; color: var(--fg-3); font-size: 12px; align-self: flex-start; }
  .link:hover { color: var(--fg); }
  button.drive { width: 100%; text-align: left; border-radius: 10px; padding: 6px; }
  button.drive:hover { background: var(--surface-hover); }
  button.drive.picked { background: color-mix(in srgb, var(--accent) 20%, transparent); }
</style>
