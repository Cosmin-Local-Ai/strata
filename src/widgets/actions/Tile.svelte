<script lang="ts">
  // One-click system tiles: Show Desktop, Notifications, Keyboard, Theme toggle, Microphone, Recycle Bin,
  // Language, Battery, Day/Month/Year progress (chosen by widget id).
  import { Battery, BatteryCharging, Bell, Keyboard, Mic, MicOff, Monitor, Moon, Sun, Trash2, MessageSquareText } from '@lucide/svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { clock } from '../../lib/stores.svelte'
  import type { TileProps } from '../types'
  import Ring from '../system/Ring.svelte'
  import { act } from './actions.svelte'
  let { item }: TileProps = $props()
  const id = $derived(item.widget)
  $effect(() => act.watch(id))
  const c = clock(60_000)
  const progress = $derived.by(() => {
    const n = c.now
    if (id === 'day-progress') return (n.getHours() * 60 + n.getMinutes()) / 1440
    if (id === 'month-progress') return (n.getDate() - 1 + n.getHours() / 24) / new Date(n.getFullYear(), n.getMonth() + 1, 0).getDate()
    const start = new Date(n.getFullYear(), 0, 1).getTime()
    return (n.getTime() - start) / (new Date(n.getFullYear() + 1, 0, 1).getTime() - start)
  })
</script>

{#if id === 'show-desktop'}<Monitor size="46%" strokeWidth={1.8} />
{:else if id === 'notifications'}<MessageSquareText size="46%" strokeWidth={1.8} />
{:else if id === 'keyboard'}<Keyboard size="48%" strokeWidth={1.8} />
{:else if id === 'theme-toggle'}{#if act.light}<Sun size="46%" strokeWidth={1.8} />{:else}<Moon size="44%" strokeWidth={1.8} />{/if}
{:else if id === 'microphone'}<span class:off={act.mic === true}>{#if act.mic === true}<MicOff size={20} strokeWidth={1.8} />{:else}<Mic size={20} strokeWidth={1.8} />{/if}</span>
{:else if id === 'recycle-bin'}
  <span class="rb"><Trash2 size={20} strokeWidth={1.8} />{#if act.bin.items > 0}<span class="badge">{act.bin.items > 999 ? '999+' : act.bin.items}</span>{/if}</span>
{:else if id === 'language'}
  <span class="lang">{(act.layouts.find((l) => l.current)?.locale ?? '··').split('-')[0].toUpperCase()}</span>
{:else if id === 'battery'}
  <span class="bat">
    {#if !act.battery?.present}<Battery size={18} /><small>AC</small>
    {:else}{#if act.battery.charging}<BatteryCharging size={18} />{:else}<Battery size={18} />{/if}<small>{act.battery.percent}%</small>{/if}
  </span>
{:else if id === 'year-progress'}
  <span class="dots" title={`${Math.round(progress * 100)}% of the year`}>
    {#each Array(52) as _, i}<i class:on={i / 52 < progress}></i>{/each}
  </span>
{:else}
  <span class="prog"><Ring value={progress} size={30} label={`${Math.round(progress * 100)}`} /><small>{id === 'day-progress' ? 'Day' : 'Month'}</small></span>
{/if}

<style>
  .off { color: var(--danger); }
  .rb { position: relative; display: grid; place-items: center; }
  .badge { position: absolute; top: -7px; right: -11px; min-width: 15px; height: 15px; padding: 0 3px; border-radius: 8px; background: var(--accent); color: white; font: 600 9.5px/15px var(--font); text-align: center; }
  .lang { font: 700 12.5px var(--font); letter-spacing: .03em; }
  .bat, .prog { display: flex; align-items: center; gap: 6px; padding: 0 10px; height: 100%; }
  small { font-size: 11.5px; color: var(--fg-2); font-variant-numeric: tabular-nums; }
  .dots { display: grid; grid-template-columns: repeat(13, 4px); gap: 2px; padding: 0 10px; }
  .dots i { width: 4px; height: 4px; border-radius: 1px; background: var(--surface-press); }
  .dots i.on { background: var(--accent); }
</style>
