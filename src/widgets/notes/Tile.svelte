<script lang="ts">
  import { StickyNote } from '@lucide/svelte'
  import { dataFile } from '../../lib/stores.svelte'
  import type { TileProps } from '../types'
  import type { NotesFile } from './notes'
  let { }: TileProps = $props()
  const data = dataFile<NotesFile>('notes', { notes: [] })
  const count = $derived(data.value.notes.length)
</script>

<span class="wrap">
  <StickyNote size="50%" strokeWidth={1.8} />
  {#if count}<span class="badge">{count}</span>{/if}
</span>

<style>
  .wrap { position: relative; display: grid; place-items: center; width: 100%; height: 100%; }
  .badge { position: absolute; top: 8%; right: 8%; min-width: 16px; height: 16px; padding: 0 4px; border-radius: 8px;
           background: var(--accent); color: white; font: 600 10px/16px var(--font); text-align: center; }
</style>
