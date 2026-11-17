<script lang="ts">
  // Grid of tray icons. Left-click, right-click (the app's own menu) and double-click are forwarded to
  // the app exactly as the taskbar would (system/tray.rs).
  import { invoke } from '@tauri-apps/api/core'
  import type { CardProps } from '../types'
  import { startTray, tray, type TrayIcon } from './tray.svelte'
  let { close }: CardProps = $props()
  startTray()

  function send(i: TrayIcon, button: string, e: MouseEvent) {
    e.preventDefault()
    if (button === 'right') invoke('card_sticky', { on: true })
    invoke('tray_click', { key: i.key, button, x: e.screenX, y: e.screenY }).catch(() => {})
    // Apps open their own windows or menus; our card steps aside.
    if (button !== 'right') close()
    else setTimeout(() => invoke('card_sticky', { on: false }), 8000) // never sticky forever
  }
</script>

<div class="tc">
  {#if tray.value.length === 0}
    <p>No tray icons yet. They appear as apps add them (a few seconds after Strata starts).</p>
  {:else}
    <div class="grid">
      {#each tray.value as i (i.key)}
        <button class="ic" title={i.tooltip || i.exe}
                onclick={(e) => send(i, 'left', e)} ondblclick={(e) => send(i, 'double', e)}
                oncontextmenu={(e) => send(i, 'right', e)} onauxclick={(e) => e.button === 1 && send(i, 'middle', e)}>
          {#if i.icon}<img src={i.icon} alt="" draggable="false" />{:else}<span class="ph">{(i.tooltip || i.exe || '?').slice(0, 1)}</span>{/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .tc { padding: 10px; }
  .grid { display: grid; grid-template-columns: repeat(6, 40px); gap: 4px; }
  .ic { width: 40px; height: 40px; border-radius: 10px; display: grid; place-items: center;
        transition: background var(--t-fast) var(--ease-out), transform var(--t-fast) var(--ease-out); }
  .ic:hover { background: var(--surface-hover); }
  .ic:active { transform: scale(0.9); background: var(--surface-press); }
  .ic img { width: 20px; height: 20px; image-rendering: auto; }
  .ph { font-weight: 600; color: var(--fg-2); }
  p { margin: 4px; width: 240px; color: var(--fg-2); font-size: 12px; line-height: 1.45; }
</style>
