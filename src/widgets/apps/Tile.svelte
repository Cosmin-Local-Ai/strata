<script lang="ts">
  // Apps: running windows like taskbar buttons (brief §4.3). One icon per app (Store apps grouped by
  // AUMID); dot = open, brighter/wider dot = in front; dimmed = on another komorebi workspace.
  // Click: bring forward, or minimise if already in front; several windows: window list card.
  // Middle-click: new window. Right-click: close / new window / pin.
  import { invoke } from '@tauri-apps/api/core'
  import { iconUrl, ipc } from '../../lib/ipc'
  import { komorebi, settings } from '../../lib/stores.svelte'
  import type { TileProps } from '../types'
  import { groupWindows, running, startWindows, windowsForPinned, type AppGroup } from './apps.svelte'
  import { previewHover } from './hover'
  import { magnifyAt } from '../../lib/pointer.svelte'
  const btns: Record<string, HTMLElement> = {}
  // Layout x of an element within the window (offsets are not affected by scale transforms).
  function layoutX(el: HTMLElement) { let x = 0; let n: HTMLElement | null = el; while (n) { x += n.offsetLeft - (n.scrollLeft || 0); n = n.offsetParent as HTMLElement | null }; return x }
  const scaleOf = (key: string) => { const el = btns[key]; return el ? magnifyAt(layoutX(el) + el.offsetWidth / 2) : 1 }
  let { item, ctx }: TileProps = $props()
  startWindows()

  // Pinned app tiles already represent these apps; don't show them twice.
  const pinnedPaths = $derived((settings.value?.items ?? []).filter((i) => i.widget === 'app').map((i) => String(i.options.path ?? '')))
  const pinnedHwnds = $derived(new Set(pinnedPaths.flatMap((p) => windowsForPinned(p, running.value).map((w) => w.hwnd))))
  // Scope option: 'all' (default) or 'workspace' (only windows on this monitor's focused komorebi workspace).
  const scopeHwnds = $derived.by(() => {
    if (item.options.scope !== 'workspace') return null
    const m = komorebi.value.monitors.find((x) => x.hmonitor === ctx.layout.hmonitor)
    const ws = m?.workspaces[m.focusedWorkspace]
    return ws ? new Set(ws.windows.map((w) => w.hwnd)) : null
  })
  const groups = $derived(
    groupWindows(running.value.filter((w) => (!scopeHwnds || scopeHwnds.has(w.hwnd)) && !pinnedHwnds.has(w.hwnd)))
  )

  let hoverKey = ''
  const hover = previewHover(() => 'preview:' + hoverKey, () => ({ key: hoverKey }))

  function click(g: AppGroup, el: HTMLElement, e: MouseEvent) {
    e.stopPropagation()
    hover.cancel()
    if (g.windows.length === 1) invoke('window_activate', { hwnd: g.windows[0].hwnd, toggle: true })
    else ctx.openCard(item, el, { key: g.key })
  }
  function middle(g: AppGroup, e: MouseEvent) {
    if (e.button !== 1) return
    e.preventDefault(); e.stopPropagation()
    ipc.launch(g.exe)
  }
  function menu(g: AppGroup, el: HTMLElement, e: MouseEvent) {
    e.preventDefault(); e.stopPropagation()
    const n = g.windows.length
    ctx.openMenu(item, el, [
      { label: 'New window', action: `new|${g.key}` },
      { label: 'Pin to dock', action: `pin|${g.key}` },
      { separator: true },
      { label: n > 1 ? `Close all ${n} windows` : 'Close window', action: `close|${g.key}`, danger: true },
    ])
  }
</script>

<span class="apps" class:empty={groups.length === 0}>
  {#each groups as g (g.key)}
    <button class="app" class:active={g.active} class:away={g.allCloaked} bind:this={btns[g.key]} style="transform: scale({scaleOf(g.key)})"
            onclick={(e) => click(g, e.currentTarget, e)}
            onmouseenter={(e) => { hoverKey = g.key; hover.enter(e.currentTarget) }} onmouseleave={() => hover.leave()} onauxclick={(e) => middle(g, e)} onmousedown={(e) => e.button === 1 && e.preventDefault()}
            oncontextmenu={(e) => { hover.cancel(); menu(g, e.currentTarget, e) }}>
      <img src={iconUrl(g.icon, 96)} alt="" draggable="false" />
      <span class="dot" class:many={g.windows.length > 1}></span>
    </button>
  {/each}
</span>

<style>
  .apps { display: flex; align-items: center; gap: var(--gap); height: 100%; padding: 0 2px; }
  .apps.empty { width: 0; padding: 0; }
  .app { position: relative; transform-origin: bottom center; will-change: transform; width: var(--tile); height: var(--tile); border-radius: var(--tile-r); display: grid; place-items: center;
         transition: background var(--t-fast) var(--ease-out), transform var(--t-fast) var(--ease-out), opacity var(--t-med) var(--ease-out); }
  .app:hover { background: var(--tile-bg-hover); }
  .app:active { transform: scale(0.92); }
  .app.active { background: var(--tile-bg-active); }
  .app.away { opacity: 0.55; }
  .app img { width: 66%; height: 66%; object-fit: contain; pointer-events: none; }
  .dot { position: absolute; bottom: 2px; left: 50%; width: 4px; height: 4px; margin-left: -2px; border-radius: 2px; background: var(--fg-2);
         transition: width var(--t-med) var(--ease-spring), margin var(--t-med) var(--ease-spring), background var(--t-med); }
  .dot.many { box-shadow: 6px 0 0 var(--fg-3), -6px 0 0 var(--fg-3); }
  .app.active .dot { width: 12px; margin-left: -6px; background: var(--fg); }
</style>
