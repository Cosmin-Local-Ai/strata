<script lang="ts">
  // CPU ring · RAM ring · Top CPU · Top RAM · Network · Storage (one component, chosen by widget id).
  import { ArrowDown, ArrowUp, HardDrive } from '@lucide/svelte'
  import type { TileProps } from '../types'
  import Ring from './Ring.svelte'
  import { gb, rate, spark, stats, useStats } from './stats.svelte'
  let { item }: TileProps = $props()
  const id = $derived(item.widget)
  $effect(() => useStats({ procs: id === 'top-cpu' || id === 'top-ram', drives: id === 'storage' || id === 'drives' }))
  const s = $derived(stats.value)
  const ram = $derived(s ? s.ramUsed / Math.max(1, s.ramTotal) : 0)
  const sys = $derived(s?.drives.find((d) => d.mount === item.options.drive) ?? s?.drives.find((d) => d.mount.toUpperCase().startsWith('C')) ?? s?.drives[0])
</script>

{#if id === 'cpu'}
  <span class="col"><Ring value={(s?.cpu ?? 0) / 100} size={30} label={`${Math.round(s?.cpu ?? 0)}`} /><small>CPU</small></span>
{:else if id === 'ram'}
  <span class="col"><Ring value={ram} size={30} label={`${Math.round(ram * 100)}`}  /><small>RAM</small></span>
{:else if id === 'top-cpu' || id === 'top-ram'}
  {@const p = (id === 'top-cpu' ? s?.topCpu : s?.topRam)?.[0]}
  <span class="top">
    <small>{id === 'top-cpu' ? 'Top CPU' : 'Top RAM'}</small>
    <strong>{p?.name ?? '…'}</strong>
    <span class="v">{p ? (id === 'top-cpu' ? `${p.value.toFixed(1)}%` : gb(p.value)) : ''}</span>
  </span>
{:else if id === 'network'}
  <span class="net">
    <svg viewBox="0 0 60 24" width="60" height="24" class="spark"><path d={spark(stats.down, 60, 22)} /><path class="up" d={spark(stats.up, 60, 22, Math.max(...stats.down, 1))} /></svg>
    <span class="rates"><span><ArrowDown size={10} />{rate(s?.netDown ?? 0)}</span><span class="muted"><ArrowUp size={10} />{rate(s?.netUp ?? 0)}</span></span>
  </span>
{:else if id === 'drives'}
  <span class="col">
    {#each s?.drives ?? [] as d (d.mount)}<span class="dr" title="{d.mount} {gb(d.free)} free of {gb(d.total)}"><Ring value={1 - d.free / Math.max(1, d.total)} size={26} color={1 - d.free / Math.max(1, d.total) > 0.9 ? 'var(--danger)' : 'var(--accent)'} label={d.mount.slice(0, 1)} /></span>{:else}<HardDrive size={18} />{/each}
  </span>
{:else if id === 'storage'}
  <span class="col">
    {#if sys}<Ring value={1 - sys.free / Math.max(1, sys.total)} size={30}  label={sys.mount.slice(0, 1)} />{:else}<HardDrive size={18} />{/if}
    <small>{sys ? `${gb(sys.free)} free` : 'Storage'}</small>
  </span>
{/if}

<style>
  .col { display: flex; align-items: center; gap: 7px; padding: 0 10px; height: 100%; }
  .col small { min-width: 30px; font-variant-numeric: tabular-nums; }
  small { font-size: 11px; color: var(--fg-2); white-space: nowrap; }
  .top { display: flex; flex-direction: column; justify-content: center; padding: 0 12px; height: 100%; line-height: 1.15; width: 104px; } /* fixed: name changes never move neighbours */
  .top strong { font-size: 12.5px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .v { font-size: 11px; color: var(--fg-2); font-variant-numeric: tabular-nums; }
  .net { display: flex; align-items: center; gap: 8px; padding: 0 10px; height: 100%; }
  .spark path { fill: none; stroke: var(--accent); stroke-width: 1.6; stroke-linejoin: round; }
  .spark path.up { stroke: #64d2ff; opacity: 0.8; }
  .rates { display: flex; flex-direction: column; width: 66px; /* fixed: changing speeds never move neighbours */ font-size: 11px; font-variant-numeric: tabular-nums; line-height: 1.25; }
  .rates span { display: flex; align-items: center; gap: 3px; white-space: nowrap; }
  .muted { color: var(--fg-2); }
</style>
