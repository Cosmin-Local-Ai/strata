<script lang="ts">
  import { Search } from '@lucide/svelte'
  import type { TileProps } from '../types'
  let { }: TileProps = $props()
</script>

<Search size="44%" strokeWidth={2} />
