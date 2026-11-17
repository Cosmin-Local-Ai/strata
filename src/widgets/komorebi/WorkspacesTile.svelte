<script lang="ts">
  // Komorebi workspaces for *this dock's* monitor.
  // Styles (item option `style`): 'dots' (default: circles, the focused one
  // stretches into a pill), 'names' (pills with workspace names), 'numbers'.
  // Palette (option `palette`): 'theme' (accent colour) or 'pastel'.
  import { komorebi } from '../../lib/stores.svelte'
  import { ipc } from '../../lib/ipc'
  import type { TileProps } from '../types'
  import { monitorFor } from './komorebi'
  let { item, ctx }: TileProps = $props()
  const mon = $derived(monitorFor(ctx.layout.hmonitor))
  const style = $derived((item.options.style as string) ?? 'dots')
  const palette = $derived((item.options.palette as string) ?? 'theme')
  const plural = (n: number) => `${n} window${n === 1 ? '' : 's'}`
</script>

{#if !komorebi.value.online}
  <button class="offline" onclick={(e) => { e.stopPropagation(); ipc.komorebiControl('start') }}
          title="Komorebi is not running. Click to start komorebi and whkd.">Komorebi offline</button>
{:else if mon}
  <div class="ws {style} {palette}">
    {#each mon.workspaces as w (w.index)}
      <button class="w" class:focused={w.index === mon.focusedWorkspace} class:populated={w.windows.length > 0}
              title={`${w.name} · ${plural(w.windows.length)}`}
              onclick={(e) => { e.stopPropagation(); ipc.komorebi('focus-monitor-workspace', mon.index, w.index) }}>
        {#if style === 'names'}{w.name}{:else if style === 'numbers'}{w.index + 1}{/if}
      </button>
    {/each}
  </div>
{/if}

<style>
  .ws { display: flex; align-items: center; height: 100%; padding: 0 12px;
        --empty: var(--fg-3); --pop: var(--fg-2); --focus: var(--accent); --hover: var(--fg);
        --empty-bg: var(--surface); --pop-bg: var(--surface-hover); }
  .ws.pastel { --empty: #9cd1dd; --pop: #ea7094; --focus: #c0aae8; --hover: #91b2e7; }
  .w { display: grid; place-items: center; flex: none;
       transition: width var(--t-slow) var(--ease-spring), min-width var(--t-slow) var(--ease-spring),
                   background var(--t-med) var(--ease-out), color var(--t-med) var(--ease-out); }
  .w:active { transform: scale(0.92); }

  /* Dots: 10 px circles; the focused workspace stretches into a 30 px pill . */
  .dots { gap: 7px; }
  .dots .w { width: 10px; height: 10px; border-radius: 5px; background: var(--empty); opacity: 0.55; }
  .dots .w.populated { background: var(--pop); opacity: 1; }
  .dots .w.focused { width: 30px; background: var(--focus); opacity: 1; }
  .dots .w:hover:not(.focused) { background: var(--hover); opacity: 1; }
  .dots.theme .w.populated:not(.focused) { background: var(--fg-2); }

  /* Names / numbers: pills with text. */
  .names, .numbers { gap: 6px; }
  .names .w, .numbers .w { height: 22px; min-width: 22px; padding: 0 7px; border-radius: 11px; font: 600 11.5px var(--font);
                           background: var(--empty-bg); color: var(--fg-3); }
  .names .w.populated, .numbers .w.populated { color: var(--fg-2); background: var(--pop-bg); }
  .names .w.focused, .numbers .w.focused { min-width: 44px; background: var(--focus); color: white; }
  .names .w:hover:not(.focused), .numbers .w:hover:not(.focused) { color: var(--fg); background: var(--surface-press); }
  .pastel.names .w.populated, .pastel.numbers .w.populated { color: var(--pop); }
  .pastel.names .w, .pastel.numbers .w { color: var(--empty); }
  .pastel.names .w.focused, .pastel.numbers .w.focused { color: #1a1b26; }

  .offline { padding: 0 14px; color: var(--fg-2); font-size: 12px; height: 100%; }
</style>
