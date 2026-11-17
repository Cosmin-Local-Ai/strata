<script lang="ts">
  // Recently Opened · Downloads · Screenshots · Commands · Screenshot Capture: line icons in the
  // dock's style; Downloads and Screenshots show a count badge of items added today.
  import { Camera, Clock3, Download, Image, SquareTerminal, Wrench } from '@lucide/svelte'
  import { invoke } from '@tauri-apps/api/core'
  import type { TileProps } from '../types'
  let { item }: TileProps = $props()
  const id = $derived(item.widget)
  let today = $state(0)
  $effect(() => {
    if (id !== 'downloads' && id !== 'screenshots') return
    const load = () => invoke<{ modified: number }[]>('folder_list', { which: id, limit: 50 }).then((l) => {
      const d0 = new Date(); d0.setHours(0, 0, 0, 0)
      today = l.filter((e) => e.modified >= +d0).length
    }).catch(() => {})
    load()
    const t = setInterval(load, 60_000)
    return () => clearInterval(t)
  })
</script>

<span class="ic">
  {#if id === 'recently-opened'}<Clock3 size="46%" strokeWidth={1.9} />
  {:else if id === 'downloads'}<Download size="46%" strokeWidth={1.9} />
  {:else if id === 'screenshots'}<Image size="46%" strokeWidth={1.9} />
  {:else if id === 'commands'}<SquareTerminal size="46%" strokeWidth={1.9} />
  {:else if id === 'quick-utilities'}<Wrench size="46%" strokeWidth={1.9} />
  {:else}<Camera size="46%" strokeWidth={1.9} />{/if}
  {#if today}<b>{today}</b>{/if}
</span>

<style>
  .ic { position: relative; display: grid; place-items: center; width: 100%; height: 100%; }
  b { position: absolute; top: 10%; right: 10%; min-width: 15px; height: 15px; padding: 0 4px; border-radius: 8px; background: var(--accent); color: white; font: 700 9.5px/15px var(--font); text-align: center; }
</style>
