<script lang="ts">
  // A limit window from its start to its reset: your readings, "now", the 100 % line, and a dashed
  // projection at the current pace, with what that pace means by the reset.
  let { points, start, end, label }: { points: [number, number][]; start: number; end: number; label: string } = $props()
  const W = 312, H = 92, P = 4
  const now = Date.now()
  const x = (t: number) => P + ((Math.min(Math.max(t, start), end) - start) / (end - start)) * (W - P * 2)
  const y = (v: number) => H - 14 - (Math.min(v, 110) / 110) * (H - 22)
  const pts = $derived([[start, 0] as [number, number], ...points])
  const last = $derived(pts[pts.length - 1])
  const pace = $derived((last[1]) / Math.max(1, last[0] - start)) // % per ms
  const projected = $derived(Math.round(last[1] + pace * (end - last[0])))
  const line = $derived(pts.map(([t, v], i) => `${i ? 'L' : 'M'}${x(t).toFixed(1)},${y(v).toFixed(1)}`).join(' '))
  const fmt = (t: number) => end - start > 2 * 864e5 ? new Date(t).toLocaleDateString(undefined, { weekday: 'short' }) : new Date(t).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
</script>

<div class="wc">
  <svg viewBox="0 0 {W} {H}" width={W} height={H}>
    <line x1={P} x2={W - P} y1={y(100)} y2={y(100)} class="limit" />
    <text x={W - P} y={y(100) - 3} text-anchor="end" class="lab">100%</text>
    <path d="{line} L{x(last[0])},{H - 14} L{x(start)},{H - 14} Z" class="area" />
    <path d={line} class="line" />
    <line x1={x(last[0])} y1={y(last[1])} x2={x(end)} y2={y(projected)} class="proj" />
    <line x1={x(now)} x2={x(now)} y1={6} y2={H - 14} class="now" />
    <text x={P} y={H - 2} class="lab">{fmt(start)}</text>
    <text x={x(now)} y={H - 2} text-anchor="middle" class="lab">now</text>
    <text x={W - P} y={H - 2} text-anchor="end" class="lab">reset {fmt(end)}</text>
  </svg>
  <small class:hot={projected >= 100}>{label}: at this pace, {projected >= 100 ? 'you hit the limit before the reset' : `about ${projected}% by the reset`}{points.length < 3 ? ' (more readings arrive every few minutes)' : ''}</small>
</div>

<style>
  .wc { display: flex; flex-direction: column; gap: 3px; }
  .limit { stroke: var(--danger); stroke-width: 1; stroke-dasharray: 3 3; opacity: 0.6; }
  .area { fill: color-mix(in srgb, var(--accent) 25%, transparent); }
  .line { fill: none; stroke: var(--accent); stroke-width: 1.8; stroke-linejoin: round; }
  .proj { stroke: var(--accent); stroke-width: 1.5; stroke-dasharray: 4 3; opacity: 0.7; }
  .now { stroke: var(--fg-3); stroke-width: 1; }
  .lab { fill: var(--fg-3); font-size: 9.5px; font-family: var(--font); }
  small { font-size: 11px; color: var(--fg-3); }
  small.hot { color: var(--danger); }
</style>
