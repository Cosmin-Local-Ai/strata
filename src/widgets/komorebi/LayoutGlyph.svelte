<script lang="ts">
  // Tiny diagram of a tiling layout (our own drawings).
  let { layout, size = 18 }: { layout: string; size?: number } = $props()
  const R: Record<string, number[][]> = {
    BSP: [[0, 0, 11, 24], [13, 0, 11, 11], [13, 13, 5, 11], [20, 13, 4, 11]],
    Columns: [[0, 0, 7, 24], [8.5, 0, 7, 24], [17, 0, 7, 24]],
    Rows: [[0, 0, 24, 7], [0, 8.5, 24, 7], [0, 17, 24, 7]],
    Grid: [[0, 0, 11, 11], [13, 0, 11, 11], [0, 13, 11, 11], [13, 13, 11, 11]],
    VerticalStack: [[0, 0, 14, 24], [16, 0, 8, 11], [16, 13, 8, 11]],
    HorizontalStack: [[0, 0, 24, 13], [0, 15, 11, 9], [13, 15, 11, 9]],
    UltrawideVerticalStack: [[0, 0, 5, 24], [7, 0, 10, 24], [19, 0, 5, 11], [19, 13, 5, 11]],
    RightMainVerticalStack: [[0, 0, 8, 11], [0, 13, 8, 11], [10, 0, 14, 24]],
    Scrolling: [[-3, 0, 8, 24], [7, 0, 10, 24], [19, 0, 8, 24]],
    Monocle: [[0, 0, 24, 24]],
    Floating: [[1, 2, 13, 11], [8, 9, 14, 12]],
    Paused: [[3, 2, 6, 20], [15, 2, 6, 20]],
  }
  const rects = $derived(R[layout] ?? R.BSP)
</script>

<svg viewBox="-1 -1 26 26" width={size} height={size} aria-hidden="true">
  {#each rects as [x, y, w, h]}
    <rect {x} {y} width={w} height={h} rx="2.2" />
  {/each}
</svg>

<style>
  svg { fill: currentColor; overflow: hidden; flex: none; }
</style>
