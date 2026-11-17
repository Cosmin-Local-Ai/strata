<script lang="ts">
  import { FlipHorizontal2, FlipVertical2, Maximize2, Pause, Play, RefreshCw, Rows3, SquareStack, Power, RotateCw } from '@lucide/svelte'
  import { ipc } from '../../lib/ipc'
  import { komorebi } from '../../lib/stores.svelte'
  import type { CardProps } from '../types'
  import LayoutGlyph from './LayoutGlyph.svelte'
  import { LAYOUTS } from './komorebi'
  let { payload, close }: CardProps = $props()
  const k = $derived(komorebi.value)
  const mon = $derived(k.monitors.find((m) => m.hmonitor === payload?.hmonitor) ?? k.monitors[k.focusedMonitor])
  const ws = $derived(mon?.workspaces[mon.focusedWorkspace])
  const cmd = (...a: string[]) => ipc.komorebi(...a)
</script>

<div class="lc">
  {#if !k.online}
    <p class="muted">Komorebi is not running.</p>
    <button class="wide" onclick={() => { ipc.komorebiControl('start'); close() }}><Play size={15} /> Start komorebi + whkd</button>
  {:else}
    <header>{mon?.name ?? 'Monitor'} · workspace {ws?.name}</header>
    <div class="grid">
      {#each LAYOUTS as l}
        <button class="layout" class:sel={ws?.layout === l.state && !ws?.monocle} onclick={() => cmd('change-layout', l.arg)}>
          <LayoutGlyph layout={l.state} size={26} />
          <span>{l.label}</span>
        </button>
      {/each}
    </div>
    <div class="row">
      <button onclick={() => cmd('flip-layout', 'horizontal')}><FlipHorizontal2 size={15} /> Flip H</button>
      <button onclick={() => cmd('flip-layout', 'vertical')}><FlipVertical2 size={15} /> Flip V</button>
      <button class:on={ws?.monocle} onclick={() => cmd('toggle-monocle')}><Maximize2 size={15} /> Monocle</button>
      <button onclick={() => cmd('toggle-float')}><SquareStack size={15} /> Float</button>
    </div>
    <div class="row">
      <button class:on={k.paused} onclick={() => cmd('toggle-pause')}>
        {#if k.paused}<Play size={15} /> Resume{:else}<Pause size={15} /> Pause{/if}
      </button>
      <button onclick={() => cmd('toggle-tiling')}><Rows3 size={15} /> Tiling</button>
      <button onclick={() => cmd('retile')}><RefreshCw size={15} /> Retile</button>
    </div>
    <footer>
      <button onclick={() => { ipc.komorebiControl('restart'); close() }}><RotateCw size={14} /> Restart komorebi</button>
      <button class="danger" onclick={() => { ipc.komorebiControl('stop'); close() }}><Power size={14} /> Stop</button>
    </footer>
  {/if}
</div>

<style>
  .lc { width: 380px; padding: 14px; display: flex; flex-direction: column; gap: 10px; }
  header { font-weight: 600; color: var(--fg-2); font-size: 12px; }
  .grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 6px; }
  .layout { display: flex; flex-direction: column; align-items: center; gap: 6px; padding: 10px 4px; border-radius: 10px;
            background: var(--surface); color: var(--fg-2); font-size: 11.5px; transition: background var(--t-fast) var(--ease-out); }
  .layout:hover { background: var(--surface-hover); color: var(--fg); }
  .layout.sel { background: var(--accent); color: white; }
  .row, footer { display: flex; gap: 6px; }
  .row button, footer button, .wide { flex: 1; display: flex; align-items: center; justify-content: center; gap: 6px;
            padding: 7px 6px; border-radius: 9px; background: var(--surface); font-size: 12px; }
  .row button:hover, footer button:hover, .wide:hover { background: var(--surface-hover); }
  .row button.on { background: var(--accent); color: white; }
  footer { border-top: 1px solid var(--border); padding-top: 10px; }
  .danger:hover { color: var(--danger); }
  .muted { color: var(--fg-2); margin: 0; }
  button:active { transform: scale(0.97); }
</style>
