<script lang="ts">
  // Progress ring (CPU, RAM, battery, progress widgets). value: 0..1.
  let { value, size = 30, stroke = 3.2, label = '', color = 'var(--accent)' }: { value: number; size?: number; stroke?: number; label?: string; color?: string } = $props()
  const r = $derived((size - stroke) / 2)
  const c = $derived(2 * Math.PI * r)
</script>

<span class="ring" style="width:{size}px; height:{size}px">
  <svg viewBox="0 0 {size} {size}" width={size} height={size}>
    <circle cx={size / 2} cy={size / 2} {r} fill="none" stroke="var(--surface-press)" stroke-width={stroke} />
    <circle cx={size / 2} cy={size / 2} {r} fill="none" stroke={color} stroke-width={stroke} stroke-linecap="round"
            stroke-dasharray={c} stroke-dashoffset={c * (1 - Math.max(0, Math.min(1, value)))} transform="rotate(-90 {size / 2} {size / 2})"
            style="transition: stroke-dashoffset 600ms var(--ease-out)" />
  </svg>
  {#if label}<span class="lbl">{label}</span>{/if}
</span>

<style>
  .ring { position: relative; display: inline-grid; place-items: center; flex: none; }
  .lbl { position: absolute; font: 600 9.5px var(--font); font-variant-numeric: tabular-nums; }
</style>
