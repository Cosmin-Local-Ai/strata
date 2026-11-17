<script lang="ts">
  import { Image } from '@lucide/svelte'
  import type { TileProps } from '../types'
  let { }: TileProps = $props()
</script>

<Image size="48%" strokeWidth={1.8} />
