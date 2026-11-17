<script lang="ts">
  // Placeholder for a widget that is on the dock by default but not built yet. Honest, never fake data.
  import { Clock3 } from '@lucide/svelte'
  import type { TileProps } from '../types'
  import { plannedIcons } from './icons'
  let { item }: TileProps = $props()
  const Icon = $derived(plannedIcons[item.widget] ?? Clock3)
</script>

<span class="planned"><Icon size="46%" strokeWidth={1.8} /></span>

<style>
  .planned { display: grid; place-items: center; width: 100%; height: 100%; opacity: 0.35; }
  .planned :global(svg) { width: 46%; height: 46%; }
</style>
