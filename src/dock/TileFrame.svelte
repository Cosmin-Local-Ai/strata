<script lang="ts">
  import type { DockItem } from '../lib/ipc'
  import type { DockContext, MenuItem, WidgetDef } from '../widgets/types'

  let { item, def, ctx, active, scale = 1, showName = true }:
    { item: DockItem; def: WidgetDef; ctx: DockContext; active: boolean; scale?: number; showName?: boolean } = $props()

  let el: HTMLDivElement
  let hover = $state(false)
  let nameVisible = $state(false)
  let nameTimer: ReturnType<typeof setTimeout> | undefined
  const Tile = $derived(def.tile)
  const name = $derived(def.title?.(item) ?? def.name)

  function click(e: MouseEvent) {
    if (e.button !== 0) return
    nameVisible = false
    if (def.click) {
      if (def.click(item, ctx, el, e) === true && def.card) ctx.openCard(item, el)
    } else if (def.card) {
      ctx.openCard(item, el)
    }
  }
  function aux(e: MouseEvent) {
    if (e.button === 1 && def.middle) { e.preventDefault(); def.middle(item, ctx) }
  }
  function context(e: MouseEvent) {
    e.preventDefault()
    nameVisible = false
    if (def.rightClick && def.rightClick(item, ctx, el) !== true) return
    const items: MenuItem[] = [...(def.menu?.(item) ?? [])]
    if (items.length) items.push({ separator: true })
    items.push({ label: 'Remove from dock', action: 'remove', danger: true })
    ctx.openMenu(item, el, items)
  }
  function wheel(e: WheelEvent) {
    if (def.wheel) { e.preventDefault(); def.wheel(item, Math.sign(e.deltaY || e.deltaX), ctx) }
  }
  function enter() {
    hover = true
    clearTimeout(nameTimer)
    nameTimer = setTimeout(() => (nameVisible = hover), 450)
  }
  function leave() { hover = false; nameVisible = false; clearTimeout(nameTimer) }
</script>

<div class="frame {def.shape}" class:active class:planned={def.planned} bind:this={el} role="button" tabindex="-1"
     style="--s:{scale}" title={showName && !active && item.widget !== 'apps' ? name : undefined}
     onclick={click} onauxclick={aux} onmousedown={(e) => e.button === 1 && e.preventDefault()}
     oncontextmenu={context} onwheel={wheel} onmouseenter={enter} onmouseleave={leave}>
  <Tile {item} {ctx} {active} />
</div>

<style>
  .frame { position: relative; flex: none; height: var(--tile); display: grid; place-items: center; border-radius: var(--tile-r);
           background: var(--tile-bg); color: var(--fg);
           transform-origin: var(--origin, bottom) center; transform: scale(var(--s));
           will-change: transform; z-index: calc(var(--s) * 10);
           transition: transform 120ms var(--ease-out), background var(--t-fast) var(--ease-out); }
  .frame.square { width: var(--tile); }
  .frame.wide { min-width: var(--tile); }
  .frame:hover { background: var(--tile-bg-hover); }
  .frame:active { transform: scale(calc(var(--s) * 0.94)); }
  .frame.active { background: var(--tile-bg-active); }
  .name { position: absolute; bottom: calc(100% + 10px); left: 50%; transform: translateX(-50%); white-space: nowrap;
          padding: 4px 9px; border-radius: 8px; font-size: 12px; font-weight: 600; pointer-events: none;
          background: rgba(30, 30, 34, 0.92); color: white; box-shadow: 0 4px 14px rgba(0, 0, 0, 0.3);
          animation: pop var(--t-med) var(--ease-out); }
  .name.below { bottom: auto; top: calc(100% + 10px); }
  @keyframes pop { from { opacity: 0; transform: translate(-50%, 4px); } }
</style>
