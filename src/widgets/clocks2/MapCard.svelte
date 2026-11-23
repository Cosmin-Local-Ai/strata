<script lang="ts">
  import { sysClock } from '../../lib/clock.svelte'
  // World Map card: the big map with live day/night and city times; click cities below to show/hide them.
  import type { CardProps } from '../types'
  import WorldMap, { CITIES, DEFAULT_CITIES } from './WorldMap.svelte'
  let { item, act }: CardProps = $props()
  let cities = $state<string[]>(item.options.cities ?? DEFAULT_CITIES)
  let hour12 = $state<boolean>(item.options.hour12 ?? sysClock.h12)
  function toggle(n: string) {
    cities = cities.includes(n) ? cities.filter((c) => c !== n) : [...cities, n]
    act('set-options', { cities: $state.snapshot(cities), hour12 })
  }
</script>

<div class="mc">
  <WorldMap width={780} {cities} {hour12} />
  <div class="chips">
    {#each CITIES as [name]}
      <button class:on={cities.includes(name)} onclick={() => toggle(name)}>{name}</button>
    {/each}
  </div>
  <label class="opt"><span>24-hour clock</span><input type="checkbox" class="sw" checked={!hour12} onchange={(e) => { hour12 = !e.currentTarget.checked; act('set-options', { cities: $state.snapshot(cities), hour12 }) }} /></label>
</div>

<style>
  .mc { padding: 14px; display: flex; flex-direction: column; gap: 12px; }
  .chips { display: flex; flex-wrap: wrap; gap: 5px; max-width: 780px; }
  .chips button { font-size: 11.5px; padding: 4px 9px; border-radius: 999px; background: var(--surface); color: var(--fg-3); border: 1px solid transparent; }
  .chips button.on { background: color-mix(in srgb, var(--accent) 22%, transparent); color: var(--fg); border-color: color-mix(in srgb, var(--accent) 50%, transparent); }
  .opt { display: flex; justify-content: space-between; align-items: center; font-size: 12.5px; }
</style>
