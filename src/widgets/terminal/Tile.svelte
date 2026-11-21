<script lang="ts">
  import { SquareTerminal } from '@lucide/svelte'
  import type { TileProps } from '../types'
  let { }: TileProps = $props()
</script>

<SquareTerminal size="46%" strokeWidth={1.9} />
