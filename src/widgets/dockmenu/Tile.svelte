<script lang="ts">
  import { Ellipsis } from '@lucide/svelte'
  import type { TileProps } from '../types'
  let { }: TileProps = $props()
</script>

<Ellipsis size="46%" strokeWidth={1.9} />
