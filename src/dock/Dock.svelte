<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import { getCurrentWebview } from '@tauri-apps/api/webview'
  import { startSmart } from '../lib/smart.svelte'
  import { onMount } from 'svelte'
  import { ipc, type DockItem, type Layout } from '../lib/ipc'
  import { settings, startStores } from '../lib/stores.svelte'
  import { registry } from '../widgets'
  import type { DockContext, MenuItem } from '../widgets/types'
  import TileFrame from './TileFrame.svelte'
  import { flip } from 'svelte/animate'
  import { dockPointer } from '../lib/pointer.svelte'

  type YieldState = { active: boolean; reasons: string[] }
  startStores()

  let layout = $state<Layout | null>(null)
  let yielding = $state(false)
  let hovered = $state(false)
  let mouseX = $state<number | null>(null)
  let activeUid = $state<string | null>(null)
  // Widgets picker open: tiles show a remove badge and can be dragged to reorder.
  let editing = $state(false)
  let note = $state<string | null>(null), noteAction = $state<string | undefined>()
  let noteTimer: ReturnType<typeof setTimeout> | undefined
  let dragUid = $state<string | null>(null)
  let order = $state<DockItem[] | null>(null)
  let body: HTMLDivElement | undefined = $state()
  // Scrolling row inside the body; it reaches into the headroom so magnified tiles aren't clipped.
  let track: HTMLDivElement | undefined = $state()
  const items = $derived(order ?? (settings.value?.items ?? []).filter((i) => registry[i.widget]))

  function dragOver(uid: string) {
    if (!dragUid || dragUid === uid) return
    const list = [...items]
    const from = list.findIndex((i) => i.uid === dragUid)
    const to = list.findIndex((i) => i.uid === uid)
    if (from < 0 || to < 0) return
    list.splice(to, 0, list.splice(from, 1)[0])
    order = list
  }
  // Pointer-based reordering: press a tile and move it ~6 px. Allowed while the Widgets picker is open,
  // or any time unless "Lock dock" is on. A press without movement stays a normal click.
  let press: { uid: string; x: number; id: number } | null = null
  let suppressClick = false
  function tileDown(e: PointerEvent, uid: string) {
    if (e.button !== 0 || (!editing && settings.value?.lockDock)) return
    press = { uid, x: e.clientX, id: e.pointerId }
  }
  function onMove(e: PointerEvent) {
    if (!press || e.pointerId !== press.id) return
    if (!dragUid && Math.abs(e.clientX - press.x) > 6) { dragUid = press.uid; body?.setPointerCapture(e.pointerId) }
    if (!dragUid) return
    // Target = the slot under the pointer's x.
    for (const it of items) {
      const el = frames[it.uid]
      if (!el || it.uid === dragUid) continue
      const r = el.getBoundingClientRect()
      if (e.clientX > r.left && e.clientX < r.right) { dragOver(it.uid); break }
    }
  }
  function onUp(e: PointerEvent) {
    if (!press || e.pointerId !== press.id) return
    if (dragUid) { suppressClick = true; dragEnd(); setTimeout(() => (suppressClick = false), 0) }
    press = null
  }
  function dragEnd() {
    if (order) saveItems(order)
    order = null
    dragUid = null
  }
  const magnify = $derived(!!settings.value?.magnification && !yielding)
  $effect(() => { dockPointer.magnify = magnify; dockPointer.maxScale = settings.value?.magnificationScale ?? 1.35 })
  const maxScale = $derived(settings.value?.magnificationScale ?? 1.5)
  let frames: Record<string, HTMLElement> = {}

  function anchor(el: HTMLElement) {
    const r = el.getBoundingClientRect()
    return { x: r.left, y: r.top, w: r.width, h: r.height }
  }

  const ctx: DockContext = {
    get layout() { return layout! },
    openCard(item, el, payload) { ipc.cardOpen(item.uid, item.widget, anchor(el), 'card', payload ?? null) },
    openMenu(item, el, menu: MenuItem[]) { ipc.cardOpen(item.uid + ':menu', item.widget, anchor(el), 'menu', { items: menu, uid: item.uid }) },
    setOptions(item, options) { saveItems(items.map((i) => (i.uid === item.uid ? { ...i, options: { ...i.options, ...options } } : i))) },
  }

  // Right-click on the dock itself (not a tile), or the Dock Menu widget.
  function dockMenu(e: MouseEvent) {
    if (e.target !== body && e.target !== track) return
    e.preventDefault()
    openDockMenu({ x: e.clientX - 1, y: e.clientY - 1, w: 2, h: 2 })
  }
  $effect(() => {
    const h = (e: Event) => { const r = (e as CustomEvent<HTMLElement>).detail.getBoundingClientRect(); openDockMenu({ x: r.left, y: r.top, w: r.width, h: r.height }) }
    window.addEventListener('strata:dockmenu', h)
    return () => window.removeEventListener('strata:dockmenu', h)
  })
  function openDockMenu(at: { x: number; y: number; w: number; h: number }) {
    ipc.cardOpen('dock:menu', 'dock', at, 'menu', {
      uid: 'dock',
      items: [
        { label: 'Widgets…', action: 'widgets' },
        { label: 'Settings…', action: 'settings' },
        { separator: true },
        { label: 'Quit Strata', action: 'quit', danger: true },
      ],
    })
  }

  // ---- drop files, folders, shortcuts or programs from the desktop / Explorer to pin them here.
  let dropAt = $state<{ index: number; x: number } | null>(null)
  // Only over the pinned apps (the app icons group): that's where a pin belongs. Elsewhere: no drop.
  function dropSlot(x: number): { index: number; x: number } | null {
    const pins = items.map((it, i) => ({ i, it })).filter(({ it }) => it.widget === 'app')
    if (!pins.length) return null
    const first = pins[0].i, last = pins[pins.length - 1].i
    const rects = items.map((it) => frames[it.uid]?.getBoundingClientRect())
    const lo = rects[first], hi = rects[last]
    if (!lo || !hi) return null
    const gap = layout!.spacing
    if (x < lo.left - gap - 12 || x > hi.right + gap + 12) return null
    let index = last + 1
    for (let i = first; i <= last; i++) { const r = rects[i]; if (r && x < r.left + r.width / 2) { index = i; break } }
    const edge = index <= last ? rects[index]!.left - gap / 2 : hi.right + gap / 2
    return { index, x: edge - (body?.getBoundingClientRect().left ?? 0) }
  }
  function pinDropped(paths: string[], x: number) {
    const slot = dropSlot(x)
    if (!slot) return
    const at = slot.index
    const known = new Set(items.filter((i) => i.widget === 'app').map((i) => String(i.options.path ?? '').toLowerCase()))
    const added = paths.filter((p) => !known.has(p.toLowerCase())).map((p, n) => ({
      uid: `pin${Date.now().toString(36)}${n}`, widget: 'app',
      options: { path: p, name: (p.split(/[\\/]/).filter(Boolean).pop() ?? p).replace(/\.(lnk|exe|url)$/i, '') },
    }))
    if (added.length) saveItems([...items.slice(0, at), ...added, ...items.slice(at)])
  }

  function saveItems(next: DockItem[]) {
    if (settings.value) settings.value = { ...settings.value, items: next }
    ipc.setItems(next)
  }

  // Magnification: distance from each tile's *layout* centre (offsetLeft is not changed by the scale
  // transform, so the zoom stays under the cursor). Icons grow fully, wide tiles only a little, and
  // nothing reflows (neighbours are not pushed apart), so the dock never overflows.
  function scaleFor(uid: string) {
    if (!magnify || mouseX === null) return 1
    const el = frames[uid]
    const def = registry[items.find((i) => i.uid === uid)?.widget ?? '']
    if (!el || !def) return 1
    // The Apps tile magnifies each of its icons itself.
    if (def.id === 'apps') return 1
    const tile = layout ? layout.bodyHeight * 0.78 : 44
    const w = el.offsetWidth
    const centre = el.offsetLeft + w / 2 - (track?.scrollLeft ?? 0)
    // Distance from the pointer to the tile, measured from a centre band as wide as the tile minus
    // one icon: a wide tile under the pointer gets the full effect, like an icon does.
    const d = Math.max(0, Math.abs(mouseX - centre) - Math.max(0, w / 2 - tile / 2))
    const t = Math.max(0, Math.cos(Math.min(d / (tile * 2.6), 1) * Math.PI / 2))
    // Wide tiles grow by about the same number of pixels in height as an icon does, not by the
    // same factor (a wide tile scaled like an icon would cover its neighbours).
    const k = Math.min(1, tile / Math.max(tile, w)) * 0.9 + (w <= tile * 1.2 ? 0.1 : 0)
    return 1 + (maxScale - 1) * t * (def.shape === 'square' ? 1 : Math.max(0.25, k))
  }

  // Minimal (compact) height: tiles built from stacked lines (value over label, two limit rows, a
  // week strip) would spill out of a 28 px bar. Any column inside a tile that is taller than the tile
  // shows only its first line. Checked when tiles change; nothing runs at other sizes.
  const compact = $derived(!!layout && layout.bodyHeight < 40)
  $effect(() => {
    if (!compact || !track) return
    const root = track
    const tileH = Math.round((layout?.bodyHeight ?? 28) * 0.78)
    let raf = 0
    const fit = () => {
      raf = 0
      root.querySelectorAll<HTMLElement>('[data-compact]').forEach((el) => el.removeAttribute('data-compact'))
      for (const el of root.querySelectorAll<HTMLElement>('.frame *')) {
        // scrollHeight: a column squeezed to the tile height still overflows it.
        if (el.childElementCount < 2 || Math.max(el.offsetHeight, el.scrollHeight) <= tileH + 1) continue
        const cs = getComputedStyle(el)
        const column = (cs.display.includes('flex') && cs.flexDirection.startsWith('column')) || (cs.display.includes('grid') && el.children[0] instanceof HTMLElement && el.children[1] instanceof HTMLElement && (el.children[1] as HTMLElement).offsetTop > (el.children[0] as HTMLElement).offsetTop + 2)
        if (column) el.setAttribute('data-compact', '')
      }
    }
    const later = () => { if (!raf) raf = requestAnimationFrame(fit) }
    const mo = new MutationObserver(later)
    mo.observe(root, { childList: true, subtree: true, characterData: true })
    later()
    return () => { mo.disconnect(); cancelAnimationFrame(raf); root.querySelectorAll('[data-compact]').forEach((el) => el.removeAttribute('data-compact')) }
  })

  let lastExtra = 0
  let repaint = $state(false)
  // Auto-hide (brief §4.1): the dock slides out of view and back when the pointer touches the edge.
  const autoHide = $derived(!!settings.value?.autoHide && !editing)
  let shown = $state(true)
  let hideTimer: ReturnType<typeof setTimeout> | undefined
  function reveal() { clearTimeout(hideTimer); if (!shown) { shown = true; followGlass() } }
  function scheduleHide() {
    clearTimeout(hideTimer)
    if (!autoHide) return
    hideTimer = setTimeout(() => { if (!hovered && !activeUid) { shown = false; followGlass() } }, 650)
  }
  $effect(() => { if (autoHide) scheduleHide(); else { shown = true; followGlass() } })
  $effect(() => { if (!activeUid) scheduleHide() })
  // Keep the glass glued to the body while it slides (only during the 240 ms slide).
  function followGlass() {
    const end = performance.now() + 280
    const step = () => { report(); if (performance.now() < end) requestAnimationFrame(step) }
    requestAnimationFrame(step)
  }

  // Drag the dock by an empty spot into the top or bottom half of the screen to move it there.
  let dragEdge = $state<{ startY: number; moved: boolean } | null>(null)
  function edgeDown(e: PointerEvent) {
    if (e.button !== 0 || (e.target !== body && e.target !== track) || editing) return
    dragEdge = { startY: e.screenY, moved: false }
    body?.setPointerCapture(e.pointerId)
  }
  function edgeMove(e: PointerEvent) {
    if (!dragEdge) return
    if (Math.abs(e.screenY - dragEdge.startY) > 12) dragEdge.moved = true
  }
  function edgeUp(e: PointerEvent) {
    if (!dragEdge) return
    const d = dragEdge
    dragEdge = null
    body?.releasePointerCapture(e.pointerId)
    if (!d.moved || !layout) return
    const physY = e.screenY * (window.devicePixelRatio || 1)
    const target = e.screenY < window.screen.height / 2 ? 'top' : 'bottom'
    void physY
    if (target !== layout.edge) invoke('settings_update', { patch: { edge: target } })
  }

  // Glass body + clickable region (headroom only while hovered, for names and magnified tiles).
  function report() {
    if (!body || !layout) return
    const k = window.devicePixelRatio
    const r = body.getBoundingClientRect()
    const radius = Math.min(r.height * 0.42, 28)
    invoke('glass_rect', { rect: { x: r.left * k, y: r.top * k, w: r.width * k, h: r.height * k, radius: radius * k, alpha: 1 } })
    // Grow upward only for magnification (magnified icons extend above the body). Nothing else may
    // draw outside the body, or the enlarged region shows as a pale band over the desktop.
    if (autoHide && !shown) {
      const wh = window.innerHeight
      const y0 = layout.edge === 'bottom' ? wh - 2 : 0
      invoke('dock_region', { rects: [[0, y0 * k, window.innerWidth * k, 2 * k, 0]] })
      invoke('glass_rect', { rect: { x: r.left * k, y: r.top * k, w: r.width * k, h: r.height * k, radius: radius * k, alpha: 0 } })
      return
    }
    const extra = hovered && magnify ? layout.headroom : 0
    const y = layout.edge === 'bottom' ? r.top - extra : r.top
    invoke('dock_region', { rects: [[r.left * k, y * k, r.width * k, (r.height + extra) * k, radius * k]] })
    if (extra !== lastExtra) { lastExtra = extra; repaint = !repaint }
  }
  $effect(() => { void hovered; void layout; void items.length; void shown; void autoHide; report() })
  $effect(() => {
    if (!body) return
    const ro = new ResizeObserver(report)
    ro.observe(body)
    return () => ro.disconnect()
  })

  onMount(() => {
    ipc.layout().then((l) => { if (l) layout = l })
    // Reveal only once the dock has settled: tiles fill in asynchronously (cover art, weather, icons,
    // fonts) and each changes the pill's width. We wait until the body's size has been stable for
    // 150 ms (cap 1.5 s), so it never appears at one size and then jumps to another.
    let revealed = false
    const reveal = () => { if (!revealed) { revealed = true; requestAnimationFrame(() => invoke('dock_ready')) } }
    const waitLayout = setInterval(() => {
      if (!(layout && settings.value && body)) return
      clearInterval(waitLayout)
      let quiet: ReturnType<typeof setTimeout> | undefined
      const settle = () => { clearTimeout(quiet); quiet = setTimeout(() => { ro.disconnect(); reveal() }, 150) }
      const ro = new ResizeObserver(settle)
      ro.observe(body)
      document.fonts.ready.then(settle)
      settle()
      setTimeout(() => { ro.disconnect(); reveal() }, 1500)
    }, 30)
    startSmart()
    const label = (window as any).__TAURI_INTERNALS__?.metadata?.currentWindow?.label
    const subs = [
      listen<Layout>('dock:layout', (e) => (layout = e.payload)),
      listen<YieldState>('dock:yield', (e) => (yielding = e.payload.active)),
      listen<boolean>('picker:state', (e) => (editing = e.payload)),
      getCurrentWebview().onDragDropEvent((e) => {
        const p = e.payload
        if (p.type === 'leave') { dropAt = null; return }
        const x = p.position.x / window.devicePixelRatio
        if (p.type === 'drop') { if (p.paths.length) pinDropped(p.paths, x); dropAt = null }
        else if (layout) { reveal(); dropAt = dropSlot(x) }
      }),
      listen<string | { text: string; action?: string }>('dock:notice', (e) => {
        const p = typeof e.payload === 'string' ? { text: e.payload } : e.payload
        note = p.text; noteAction = p.action
        clearTimeout(noteTimer); noteTimer = setTimeout(() => (note = null), p.action ? 20000 : 7000)
      }),
      listen<{ uid: string; dock: string }>('card:shown', (e) => (activeUid = e.payload.dock === label ? e.payload.uid : null)),
      listen('card:closed', () => (activeUid = null)),
      listen<{ uid: string; action: string; data: any }>('card:action', (e) => {
        const { uid, action, data } = e.payload
        if (uid === 'dock') {
          if (action === 'menu:settings') invoke('open_settings').catch((e) => invoke('ui_error', { message: `open settings: ${e}` }))
          else if (action === 'menu:widgets') ipc.overlayOpen('widgets')
          else if (action === 'menu:quit') invoke('quit')
          return
        }
        const item = items.find((i) => i.uid === uid)
        if (!item) return
        if (action === 'set-options') ctx.setOptions(item, data)
        else if (action === 'add-item') saveItems([...items, { uid: `${data.widget}-${Date.now().toString(36)}`, widget: data.widget, options: data.options ?? {} }])
        else if (action === 'menu:remove') saveItems(items.filter((i) => i.uid !== uid))
        else if (action.startsWith('menu:')) registry[item.widget]?.onMenu?.(item, action.slice(5), ctx)
      }),
    ]
    return () => subs.forEach((p) => p.then((f) => f()))
  })
</script>

{#if layout}
  <div class="repaint" style="background: rgba(0,0,0,{repaint ? 0.004 : 0.003})"></div>
  <main class:top={layout.edge === 'top'} class:compact={layout.bodyHeight < 40} class:yield={yielding} class="style-{settings.value?.style ?? 'liquidGlass'}"
        style="--body-h:{layout.bodyHeight}px; --dist:{layout.edgeDistance}px; --pad:{layout.padding}px; --gap:{layout.spacing}px;
               --max-w:{layout.maxWidth}px; --tile:{Math.round(layout.bodyHeight * 0.78)}px; --tile-r:{Math.round(layout.bodyHeight * 0.78 * 0.24)}px;
               --origin:{layout.edge === 'top' ? 'top' : 'bottom'}; --accent:{settings.value?.accent ?? '#0a84ff'}">
    {#if autoHide}<div class="trigger" class:top={layout.edge === 'top'} role="presentation" onmouseenter={reveal}></div>{/if}
    <div class="body" class:full={layout.fullWidth} class:hidden={autoHide && !shown} class:dragging-edge={dragEdge?.moved} bind:this={body} role="toolbar" tabindex="-1"
         onmouseenter={() => { hovered = true; reveal() }} onmousemove={(e) => { mouseX = e.clientX; dockPointer.x = e.clientX }}
         onmouseleave={() => { hovered = false; mouseX = null; dockPointer.x = null; scheduleHide() }} oncontextmenu={dockMenu}
         onpointerdown={edgeDown} onpointermove={(e) => { edgeMove(e); onMove(e) }} onpointerup={(e) => { edgeUp(e); onUp(e) }}
         onclickcapture={(e) => { if (suppressClick) { e.stopPropagation(); e.preventDefault() } }}
         onwheelcapture={(e) => { if (settings.value?.wheelScrollsDock && !e.ctrlKey && track && track.scrollWidth > track.clientWidth) { e.preventDefault(); e.stopPropagation(); track.scrollBy({ left: (e.deltaY + e.deltaX) * 1.2, behavior: 'smooth' }) } }}
         onwheel={(e) => { if (!e.defaultPrevented && track && track.scrollWidth > track.clientWidth) { e.preventDefault(); track.scrollBy({ left: e.deltaY + e.deltaX, behavior: 'smooth' }) } }}>
      <div class="track" bind:this={track} style="--room:{magnify ? layout.headroom : 0}px">
      {#each items as item (item.uid)}
        <div class="slot" class:grow={layout.fullWidth && item.widget === 'apps'} class:editing class:dragging={dragUid === item.uid} bind:this={frames[item.uid]} animate:flip={{ duration: 160 }}
             role="listitem" onpointerdown={(e) => tileDown(e, item.uid)}>
          {#if editing}<button class="remove" aria-label="Remove from dock" onclick={() => saveItems(items.filter((i) => i.uid !== item.uid))}><svg viewBox="0 0 10 10" width="10" height="10"><path d="M2 5h6" stroke="white" stroke-width="1.8" stroke-linecap="round" /></svg></button>{/if}
          <TileFrame {item} def={registry[item.widget]} {ctx} active={activeUid === item.uid}
                     scale={scaleFor(item.uid)} showName={settings.value?.hoverNames ?? true} />
        </div>
      {/each}
      </div>
      {#if dropAt}<div class="dropline" style="left:{dropAt.x}px"></div>{/if}
      {#if note}<button class="note" class:act={noteAction} title={noteAction ? '' : 'Dismiss'} onclick={() => { if (noteAction === 'update') invoke('open_update'); note = null }}>{note}</button>{/if}
    </div>
  </main>
{/if}

<style>
  main { position: fixed; inset: 0; display: flex; justify-content: center; align-items: flex-end; padding-bottom: var(--dist);
         --tile-bg: rgba(255, 255, 255, 0.07); --tile-bg-hover: rgba(255, 255, 255, 0.13); --tile-bg-active: rgba(255, 255, 255, 0.2); }
  main.top { align-items: flex-start; padding-bottom: 0; padding-top: var(--dist); }
  /* White style: dark content on a light body. */
  main.style-white { --fg: rgba(0,0,0,.88); --fg-2: rgba(0,0,0,.6); --fg-3: rgba(0,0,0,.38); --surface: rgba(0,0,0,.05); --surface-hover: rgba(0,0,0,.08); --surface-press: rgba(0,0,0,.12);
                     --tile-bg: rgba(0,0,0,.05); --tile-bg-hover: rgba(0,0,0,.09); --tile-bg-active: rgba(0,0,0,.14); color: var(--fg); }
  main.style-black, main.style-midnight { --tile-bg: rgba(255,255,255,.06); }
  main.style-clear { --tile-bg: transparent; --tile-bg-hover: rgba(255,255,255,.10); --tile-bg-active: rgba(255,255,255,.16); text-shadow: 0 1px 3px rgba(0,0,0,.55); }
  main.style-clear :global(svg) { filter: drop-shadow(0 1px 2px rgba(0,0,0,.5)); }
  .body { display: flex; align-items: stretch; height: var(--body-h); max-width: var(--max-w); border-radius: calc(var(--body-h) * 0.42); }
  .body.full { width: var(--max-w); }
  /* The row scrolls sideways (which clips vertically too), so it is extended by the magnification
     headroom on the side tiles grow into: magnified icons have room instead of being cut off. */
  .track { flex: 1 1 auto; min-width: 0; display: flex; align-items: center; gap: var(--gap); padding: var(--room) var(--pad) 0; margin-top: calc(-1 * var(--room));
           height: calc(100% + var(--room)); box-sizing: border-box; overflow-x: auto; overflow-y: hidden; scrollbar-width: none; }
  main.top .track { padding: 0 var(--pad) var(--room); margin: 0 0 calc(-1 * var(--room)); }
  .body.full .track { width: 100%; }
  /* GPU yield (games, local AI): the glass steps aside; a plain background keeps the dock readable. */
  main.yield .body { background: rgba(24, 24, 28, 0.82); box-shadow: inset 0 0 0 1px rgba(255,255,255,.08); }
  main.yield.style-white .body { background: rgba(246, 246, 248, 0.9); box-shadow: inset 0 0 0 1px rgba(0,0,0,.08); }
  .slot { position: relative; display: flex; }
  /* Full width / Minimal: the Apps slot absorbs the free space, giving left and right groups like a classic bar. */
  .slot.grow { flex: 1 1 auto; }
  .track { mask-image: var(--fade, none); }
  .slot { touch-action: none; }
  .body { transition: transform 240ms var(--ease-out), opacity 240ms var(--ease-out); }
  .body.hidden { transform: translateY(calc(100% + var(--dist) + 6px)); opacity: 0; }
  main.top .body.hidden { transform: translateY(calc(-100% - var(--dist) - 6px)); }
  .body.dragging-edge { cursor: grabbing; opacity: 0.85; }
  .trigger { position: fixed; left: 0; right: 0; bottom: 0; height: 2px; }
  .trigger.top { top: 0; bottom: auto; }
  .repaint { position: fixed; inset: 0; pointer-events: none; }
  /* Minimal / compact heights: single-line tiles. */
  main.compact :global(.date), main.compact :global(.a), main.compact :global(.text .a) { display: none; }
  main.compact :global(.time) { font-size: 12.5px; }
  main.compact :global(.clock) { flex-direction: row; gap: 6px; }
  main.compact :global([data-compact] > :not(:first-child)) { display: none !important; }
  main.compact :global(.frame) { overflow: hidden; }
  .slot.editing { cursor: grab; animation: wiggle 0.28s ease-in-out infinite alternate; }
  .slot.editing:nth-child(even) { animation-delay: -0.14s; }
  .slot.dragging { opacity: 0.4; }
  @keyframes wiggle { from { transform: rotate(-1.2deg); } to { transform: rotate(1.2deg); } }
  .body { position: relative; }
  .note { position: absolute; inset: 3px; z-index: 70; border-radius: inherit; background: rgba(22, 22, 26, 0.95); color: #fff; font-size: 12.5px; line-height: 1.3;
          display: flex; align-items: center; justify-content: center; padding: 0 18px; text-align: center; overflow: hidden; animation: note-in 160ms var(--ease-out); }
  .note.act { background: var(--accent); }
  /* Where a dropped file will be pinned. */
  .dropline { position: absolute; top: 18%; bottom: 18%; width: 3px; margin-left: -1.5px; border-radius: 2px; background: var(--accent);
              box-shadow: 0 0 10px var(--accent); pointer-events: none; z-index: 80; }
  @keyframes note-in { from { opacity: 0; } }
  .remove { position: absolute; top: -5px; left: -5px; z-index: 60; width: 18px; height: 18px; border-radius: 9px; background: #ff3b30; color: white;
            display: grid; place-items: center; padding: 0; box-shadow: 0 1px 4px rgba(0,0,0,.35); }
</style>
