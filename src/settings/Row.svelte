<script lang="ts">
  import { Info } from '@lucide/svelte'
  import type { Snippet } from 'svelte'
  let { label, tip = '', children }: { label: string; tip?: string; children: Snippet } = $props()
  let tipPos = $state<{ x: number; y: number } | null>(null)
  function showAt(e: MouseEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect()
    // Fixed position, clamped to the window: never clipped by the scrolling page.
    tipPos = { x: Math.min(r.right + 8, window.innerWidth - 300), y: Math.min(r.top - 4, window.innerHeight - 120) }
  }
</script>

<div class="row">
  <div class="label">
    {label}
    {#if tip}
      <span class="tipwrap" role="note" onmouseenter={showAt} onmouseleave={() => (tipPos = null)}>
        <Info size={13} />
        {#if tipPos}<span class="tip" style="left:{tipPos.x}px; top:{tipPos.y}px">{tip}</span>{/if}
      </span>
    {/if}
  </div>
  <div class="ctl">{@render children()}</div>
</div>

<style>
  .row { display: flex; align-items: center; justify-content: space-between; gap: 16px; padding: 10px 14px; min-height: 44px; }
  .row + :global(.row) { border-top: 1px solid var(--border); }
  .label { display: flex; align-items: center; gap: 6px; }
  .tipwrap { position: relative; color: var(--fg-3); display: inline-flex; }
  .tip { position: fixed; width: 280px; z-index: 50; padding: 8px 10px; border-radius: 8px;
         background: rgba(30, 30, 34, 0.97); color: white; font-size: 12px; line-height: 1.4; box-shadow: 0 6px 20px rgba(0,0,0,.35); }
  .ctl { display: flex; align-items: center; gap: 10px; }
</style>
