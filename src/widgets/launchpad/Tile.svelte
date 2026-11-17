<script lang="ts">
  import { LayoutGrid } from '@lucide/svelte'
  import type { TileProps } from '../types'
  let { }: TileProps = $props()
</script>

<LayoutGrid size="50%" strokeWidth={1.8} />
