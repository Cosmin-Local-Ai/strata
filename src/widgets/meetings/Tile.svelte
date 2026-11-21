<script lang="ts">
  // Meeting Notes tile: the notebook icon; while a meeting's timer runs it shows the running time,
  // and a badge counts open action items.
  import { NotebookPen } from '@lucide/svelte'
  import { clock, dataFile } from '../../lib/stores.svelte'
  import type { TileProps } from '../types'
  import { elapsedOf, mmss, type MeetingsFile } from './meetings'
  let { }: TileProps = $props()
  const data = dataFile<MeetingsFile>('meetings', { list: [] })
  const now = clock(1000)
  const live = $derived(data.value.list.find((m) => m.timerStart))
  const open = $derived(data.value.list.reduce((n, m) => n + m.actions.filter((a) => !a.done).length, 0))
</script>

{#if live}
  <span class="live"><span class="rec"></span><span class="t">{mmss(elapsedOf(live, now.now.getTime()))}</span></span>
{:else}
  <span class="ic"><NotebookPen size="46%" strokeWidth={1.9} />{#if open}<span class="badge">{open > 9 ? '9+' : open}</span>{/if}</span>
{/if}

<style>
  .ic { position: relative; width: 100%; height: 100%; display: grid; place-items: center; }
  .badge { position: absolute; top: 3px; right: 3px; min-width: 15px; height: 15px; padding: 0 3px; border-radius: 8px; background: var(--accent); color: white; font: 700 9.5px/15px var(--font-display); text-align: center; }
  .live { display: flex; align-items: center; gap: 6px; padding: 0 10px; height: 100%; }
  .rec { width: 8px; height: 8px; border-radius: 4px; background: #ff453a; animation: p 1.4s ease-in-out infinite; }
  @keyframes p { 50% { opacity: 0.35; } }
  .t { font: 600 13px var(--font-display); font-variant-numeric: tabular-nums; }
</style>
