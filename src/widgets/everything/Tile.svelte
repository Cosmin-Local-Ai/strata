<script lang="ts">
  // Line icons in the dock's style (like Search), not the apps' own logos.
  import { Command, FolderSearch } from '@lucide/svelte'
  import type { TileProps } from '../types'
  let { item }: TileProps = $props()
</script>

{#if item.widget === 'flow-launcher'}
  <Command size="46%" strokeWidth={1.9} />
{:else}
  <FolderSearch size="46%" strokeWidth={1.9} />
{/if}
