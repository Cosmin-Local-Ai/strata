<script lang="ts">
  // Strata's own Start mark: three stacked layers (no Windows artwork).
  import type { TileProps } from '../types'
  let { }: TileProps = $props()
</script>

<svg viewBox="0 0 24 24" class="mark" aria-hidden="true">
  <rect x="4" y="4.5" width="16" height="4" rx="2" />
  <rect x="4" y="10" width="16" height="4" rx="2" opacity=".72" />
  <rect x="4" y="15.5" width="16" height="4" rx="2" opacity=".46" />
</svg>

<style>
  .mark { width: 58%; height: 58%; fill: var(--fg); }
</style>
