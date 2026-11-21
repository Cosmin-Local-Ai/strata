<script lang="ts">
  // Full-colour weather glyphs (sun, moon, cloud, rain, snow, storm, fog) for the Weather Card look.
  let { code, day = true, size = 40 }: { code: number; day?: boolean; size?: number } = $props()
  const kind = $derived(
    code === 0 ? 'clear' : code <= 2 ? 'partly' : code === 3 ? 'cloudy' : code === 45 || code === 48 ? 'fog'
      : (code >= 51 && code <= 67) || (code >= 80 && code <= 82) ? 'rain' : (code >= 71 && code <= 77) || code === 85 || code === 86 ? 'snow' : code >= 95 ? 'storm' : 'cloudy')
  const sunOrMoon = $derived(kind === 'clear' || kind === 'partly')
</script>

<svg viewBox="0 0 64 64" width={size} height={size} aria-hidden="true">
  <defs>
    <radialGradient id="sun" cx="40%" cy="35%" r="70%"><stop offset="0" stop-color="#ffe680" /><stop offset="1" stop-color="#ffb300" /></radialGradient>
    <linearGradient id="cloud" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#ffffff" /><stop offset="1" stop-color="#d6e1ef" /></linearGradient>
    <linearGradient id="dark" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#c9d3df" /><stop offset="1" stop-color="#8e9aab" /></linearGradient>
  </defs>
  {#if sunOrMoon}
    {#if day}
      <g transform={kind === 'partly' ? 'translate(10 -4)' : 'translate(0 0)'}>
        {#each Array(8) as _, i}<rect x="31" y={kind === 'partly' ? 7 : 4} width="2.6" height="7" rx="1.3" fill="#ffc21a" transform="rotate({i * 45} 32 {kind === 'partly' ? 26 : 32})" />{/each}
        <circle cx="32" cy={kind === 'partly' ? 26 : 32} r={kind === 'partly' ? 11 : 14} fill="url(#sun)" />
      </g>
    {:else}
      <path transform={kind === 'partly' ? 'translate(10 -6)' : ''} d="M40 14a16 16 0 1 0 12 22 13 13 0 0 1-12-22z" fill="#f4e7b5" />
    {/if}
  {/if}
  {#if kind !== 'clear'}
    <g transform={kind === 'partly' ? 'translate(-2 8)' : 'translate(0 2)'}>
      <path d="M18 46h28a10 10 0 0 0 0-20 14 14 0 0 0-27-2 11 11 0 0 0-1 22z" fill={kind === 'storm' || kind === 'rain' ? 'url(#dark)' : 'url(#cloud)'} />
    </g>
  {/if}
  {#if kind === 'rain'}
    {#each [22, 32, 42] as x}<path d="M{x} 52l-3 7" stroke="#4fc3ff" stroke-width="3" stroke-linecap="round" />{/each}
  {:else if kind === 'snow'}
    {#each [22, 32, 42] as x}<circle cx={x} cy="56" r="2.4" fill="#ffffff" />{/each}
  {:else if kind === 'storm'}
    <path d="M33 48l-6 9h6l-3 7 9-11h-6l3-5z" fill="#ffd60a" />
  {:else if kind === 'fog'}
    {#each [52, 58] as y}<rect x="14" y={y} width="36" height="3" rx="1.5" fill="#e3e9f1" opacity=".9" />{/each}
  {/if}
</svg>
