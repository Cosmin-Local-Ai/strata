<script lang="ts">
  // Widgets picker (brief §4.4): sidebar of categories that tracks scrolling, a grid of live widget
  // previews, click to add / click again to remove, checkmarks for widgets already on the dock, search.
  // While it's open the dock shows a − badge on every tile and tiles can be dragged to reorder.
  import { Check, Search } from '@lucide/svelte'
  import { ipc, type DockItem, type Layout } from '../lib/ipc'
  import { settings, startStores } from '../lib/stores.svelte'
  import { widgets } from '../widgets'
  import type { Category, DockContext, WidgetDef } from '../widgets/types'

  let { close }: { close: () => void } = $props()
  startStores()

  const CATS: { id: Category; label: string }[] = [
    { id: 'apps', label: 'Apps & Files' }, { id: 'productivity', label: 'Productivity' }, { id: 'media', label: 'Media & Audio' },
    { id: 'system', label: 'System' }, { id: 'clocks', label: 'Clocks' }, { id: 'weather', label: 'Weather' },
    { id: 'wallpaper', label: 'Wallpaper' }, { id: 'komorebi', label: 'Komorebi' }, { id: 'ai', label: 'AI' },
    { id: 'finance', label: 'Finance' }, { id: 'analytics', label: 'Analytics' }, { id: 'social', label: 'Social & Mail' },
    { id: 'markets', label: 'Markets' },
  ]
  let q = $state('')
  // One category per page (user preference), plus All.
  let current = $state<Category | 'all'>('all')

  const items = $derived(settings.value?.items ?? [])
  const onDock = (id: string) => items.some((i) => i.widget === id)
  // The "app" widget is a pinned app (needs a path): pinned from the Apps tile's menu instead.
  const shown = $derived(widgets.filter((w) => w.id !== 'app' && !w.hidden && (!q || (w.name + ' ' + w.description).toLowerCase().includes(q.toLowerCase()))))
  const allGroups = $derived(CATS.map((c) => ({ ...c, list: shown.filter((w) => w.category === c.id) })).filter((g) => g.list.length))
  // Searching always looks across every category.
  const groups = $derived(current === 'all' || q ? allGroups : allGroups.filter((g) => g.id === current))

  function toggle(w: WidgetDef) {
    if (w.planned) return
    const next: DockItem[] = onDock(w.id)
      ? items.filter((i) => i.widget !== w.id)
      : [...items, { uid: w.id + '-' + Date.now().toString(36), widget: w.id, options: {} }]
    if (settings.value) settings.value = { ...settings.value, items: next }
    ipc.setItems(next)
  }

  // Previews render the real tiles with a stand-in dock context.
  const layout: Layout = { displayId: '', displayName: '', hmonitor: 0, edge: 'bottom', scale: 1, bodyHeight: 56, edgeDistance: 5,
    headroom: 0, padding: 8, spacing: 8, fullWidth: false, maxWidth: 400, windowWidth: 400, windowHeight: 80 }
  const ctx: DockContext = { layout, openCard() {}, openMenu() {}, setOptions() {} }
  const fakeItem = (w: WidgetDef): DockItem => items.find((i) => i.widget === w.id) ?? { uid: 'preview-' + w.id, widget: w.id, options: {} }
</script>

<div class="backdrop" role="presentation" onclick={close}>
  <div class="panel" role="dialog" aria-label="Widgets" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={() => {}}>
    <nav>
      <div class="title">Widgets</div>
      <button class:sel={current === 'all'} onclick={() => (current = 'all')}>All<span>{shown.length}</span></button>
      {#each allGroups as g}
        <button class:sel={current === g.id} onclick={() => (current = g.id)}>{g.label}<span>{g.list.length}</span></button>
      {/each}
      <div class="grow"></div>
      <p class="hint">Drag tiles on the dock to reorder them. Click − on a tile to remove it.</p>
    </nav>
    <section>
      <header>
        <label class="search"><Search size={15} /><input bind:value={q} placeholder="Search widgets" spellcheck="false" /></label>
        <span class="count">{widgets.length} widgets · {items.length} on dock</span>
        <button class="done" onclick={close}>Done</button>
      </header>
      <div class="scroll">
        {#each groups as g (g.id)}
          <h2>{g.label}</h2>
          <div class="grid">
            {#each g.list as w (w.id)}
              {@const Tile = w.tile}
              <button class="w" class:on={onDock(w.id)} class:planned={w.planned} onclick={() => toggle(w)} title={w.planned ? 'Not built yet' : w.description}>
                <span class="preview" class:wide={w.shape === 'wide'} style="--tile:40px; --tile-r:10px; --gap:6px">
                  <span class="tile {w.shape}"><Tile item={fakeItem(w)} {ctx} active={false} /></span>
                </span>
                <span class="meta"><strong>{w.name}</strong><small>{w.planned ? 'Not built yet' : w.description}</small></span>
                {#if onDock(w.id)}<span class="check"><Check size={13} strokeWidth={3} /></span>{/if}
              </button>
            {/each}
          </div>
        {/each}
      </div>
    </section>
  </div>
</div>

<style>
  .backdrop { position: fixed; inset: 0; display: grid; place-items: center; background: rgba(8, 8, 12, 0.45); animation: fade var(--t-med) var(--ease-out); }
  .panel { width: min(1180px, 72vw); height: 72vh; display: flex; border-radius: 30px; overflow: hidden;
           background: rgba(28, 28, 32, 0.82); border: 1px solid rgba(255, 255, 255, 0.12); box-shadow: 0 30px 80px rgba(0, 0, 0, 0.45);
           animation: rise var(--t-slow) var(--ease-out); }
  nav { width: 210px; padding: 20px 10px; display: flex; flex-direction: column; gap: 2px; border-right: 1px solid var(--border); }
  .title { font: 700 20px var(--font-display); padding: 0 10px 14px; }
  nav button { display: flex; justify-content: space-between; padding: 7px 10px; border-radius: 8px; color: var(--fg-2); text-align: left; }
  nav button span { color: var(--fg-3); font-size: 11.5px; }
  nav button:hover { background: var(--surface-hover); color: var(--fg); }
  nav button.sel { background: var(--accent); color: white; }
  nav button.sel span { color: rgba(255, 255, 255, 0.75); }
  section { flex: 1; display: flex; flex-direction: column; min-width: 0; }
  header { display: flex; align-items: center; gap: 14px; padding: 16px 22px; border-bottom: 1px solid var(--border); }
  .search { flex: 1; max-width: 360px; display: flex; align-items: center; gap: 8px; padding: 7px 10px; border-radius: 10px; background: var(--surface); border: 1px solid var(--border); color: var(--fg-2); }
  .search input { flex: 1; background: none; border: 0; outline: none; color: var(--fg); }
  .count { color: var(--fg-3); font-size: 12px; }
  .scroll { flex: 1; overflow-y: auto; padding: 4px 22px 24px; }
  h2 { font: 600 13px var(--font); color: var(--fg-2); margin: 18px 0 10px; text-transform: uppercase; letter-spacing: .05em; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 10px; }
  .w { position: relative; display: flex; align-items: center; gap: 12px; padding: 12px; border-radius: 16px; text-align: left;
       background: var(--surface); border: 1px solid transparent; transition: background var(--t-fast) var(--ease-out), border-color var(--t-fast), transform var(--t-fast); }
  .w:hover { background: var(--surface-hover); }
  .w:active:not(.planned) { transform: scale(0.98); }
  .w.on { border-color: var(--accent); }
  .w.planned { opacity: 0.45; }
  .preview { flex: none; display: grid; place-items: center; width: 64px; height: 56px; border-radius: 14px; background: rgba(255, 255, 255, 0.05); overflow: hidden; pointer-events: none; }
  .preview.wide { width: 128px; }
  .tile { display: grid; place-items: center; height: var(--tile); min-width: var(--tile); max-width: 124px; overflow: hidden; border-radius: var(--tile-r); background: rgba(255, 255, 255, 0.07); }
  .w { padding-right: 34px; }
  .grow { flex: 1; }
  .hint { font-size: 11px; color: var(--fg-3); padding: 0 10px; line-height: 1.4; margin: 0; }
  .done { margin-left: auto; padding: 6px 16px; border-radius: 9px; background: var(--accent); color: white; font-weight: 600; }
  .meta { display: flex; flex-direction: column; gap: 3px; min-width: 0; }
  .meta strong { font-size: 13px; }
  .meta small { font-size: 11px; color: var(--fg-3); line-height: 1.3; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .check { position: absolute; top: 8px; right: 8px; width: 20px; height: 20px; border-radius: 10px; background: var(--accent); color: white; display: grid; place-items: center; }
  @keyframes fade { from { opacity: 0; } }
  @keyframes rise { from { opacity: 0; transform: translateY(14px) scale(0.98); } }
</style>
