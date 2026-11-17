<script lang="ts">
  // Month calendar. The unified Calendar (iCal feeds, renewals, AI resets) will replace this card.
  import { ChevronLeft, ChevronRight } from '@lucide/svelte'
  import type { CardProps } from '../types'
  let { }: CardProps = $props()
  const today = new Date()
  let month = $state(new Date(today.getFullYear(), today.getMonth(), 1))
  const days = $derived.by(() => {
    const first = (month.getDay() + 6) % 7 // Monday first
    const n = new Date(month.getFullYear(), month.getMonth() + 1, 0).getDate()
    return [...Array(first).fill(null), ...Array.from({ length: n }, (_, i) => i + 1)]
  })
  const isToday = (d: number) => d === today.getDate() && month.getMonth() === today.getMonth() && month.getFullYear() === today.getFullYear()
  const shift = (k: number) => (month = new Date(month.getFullYear(), month.getMonth() + k, 1))
</script>

<div class="cal">
  <header>
    <strong>{month.toLocaleDateString([], { month: 'long', year: 'numeric' })}</strong>
    <span>
      <button onclick={() => shift(-1)} aria-label="Previous month"><ChevronLeft size={16} /></button>
      <button onclick={() => (month = new Date(today.getFullYear(), today.getMonth(), 1))}>Today</button>
      <button onclick={() => shift(1)} aria-label="Next month"><ChevronRight size={16} /></button>
    </span>
  </header>
  <div class="grid">
    {#each ['Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa', 'Su'] as d}<span class="dow">{d}</span>{/each}
    {#each days as d}
      <span class="day" class:today={d && isToday(d)}>{d ?? ''}</span>
    {/each}
  </div>
</div>

<style>
  .cal { width: 300px; padding: 14px; }
  header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 10px; }
  header strong { font: 600 15px var(--font-display); }
  header span { display: flex; gap: 2px; }
  header button { padding: 4px 7px; border-radius: 7px; color: var(--fg-2); font-size: 12px; display: grid; place-items: center; }
  header button:hover { background: var(--surface-hover); color: var(--fg); }
  .grid { display: grid; grid-template-columns: repeat(7, 1fr); gap: 2px; text-align: center; }
  .dow { font-size: 11px; color: var(--fg-3); padding-bottom: 4px; }
  .day { height: 34px; line-height: 34px; border-radius: 17px; font-variant-numeric: tabular-nums; }
  .day.today { background: var(--accent); color: white; font-weight: 600; }
</style>
