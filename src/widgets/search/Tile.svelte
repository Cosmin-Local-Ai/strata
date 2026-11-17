<script lang="ts">
  import { Search } from '@lucide/svelte'
  import type { TileProps } from '../types'
  let { }: TileProps = $props()
</script>

<Search size="46%" strokeWidth={1.9} />
