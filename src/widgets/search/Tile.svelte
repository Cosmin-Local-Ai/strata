<script lang="ts">
  import { Globe } from '@lucide/svelte'
  import type { TileProps } from '../types'
  let { }: TileProps = $props()
</script>

<Globe size="46%" strokeWidth={1.9} />
