<script lang="ts">
  // Filled line chart. Scales between the data's min and max (so a stock at 352.1–352.9 isn't a flat
  // line); pass min/max for fixed ranges (e.g. CPU 0–100, revenue from 0).
  let { values, width = 60, height = 22, fill = true, color = 'var(--accent)', dots = false, min, max }:
    { values: number[]; width?: number; height?: number; fill?: boolean; color?: string; dots?: boolean; min?: number; max?: number } = $props()
  const uid = `sp${Math.random().toString(36).slice(2, 8)}`
  const lo = $derived(min ?? Math.min(...values))
  const hi = $derived(Math.max(max ?? -Infinity, ...values, lo + 1e-9))
  const pad = 2
  const pts = $derived(values.map((v, i) => [values.length > 1 ? (i / (values.length - 1)) * width : width / 2, height - pad - ((v - lo) / (hi - lo)) * (height - pad * 2)] as const))
  const line = $derived(pts.map(([x, y], i) => `${i ? 'L' : 'M'}${x.toFixed(1)},${y.toFixed(1)}`).join(' '))
</script>

<svg viewBox="0 0 {width} {height}" {width} {height} class="spark" aria-hidden="true">
  <defs>
    <linearGradient id={uid} x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color={color} stop-opacity="0.38" /><stop offset="1" stop-color={color} stop-opacity="0.02" />
    </linearGradient>
  </defs>
  {#if values.length > 1}
    {#if fill}<path d="{line} L{width},{height} L0,{height} Z" fill="url(#{uid})" />{/if}
    <path d={line} fill="none" stroke={color} stroke-width="1.7" stroke-linejoin="round" stroke-linecap="round" />
    {#if dots}{#each pts as [x, y]}<circle cx={x} cy={y} r="1.6" fill={color} />{/each}{/if}
  {/if}
</svg>

<style>
  .spark { display: block; overflow: visible; }
</style>
