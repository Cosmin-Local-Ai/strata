<script lang="ts">
  // Project Launcher tile: the project's icon (or a rocket for several), a green dot while any runs.
  // Click opens the card with every project and its Open / Start / Stop buttons.
  import { LoaderCircle, Rocket } from '@lucide/svelte'
  import { iconUrl } from '../../lib/ipc'
  import type { TileProps } from '../types'
  import { iconSrc, poll, projectsOf, projState } from './state.svelte'
  let { item }: TileProps = $props()
  const list = $derived(projectsOf(item))
  const states = $derived(list.map((p) => projState(item.uid, p.id)))
  const anyRunning = $derived(states.some((s) => s.running))
  const anyBusy = $derived(states.some((s) => s.busy))
  const single = $derived(list.length === 1 ? list[0] : null)
  const src = $derived(single ? iconSrc(single.icon, iconUrl) : '')
  let iconFailed = $state(false)
  $effect(() => {
    const uid = item.uid, ids = list.map((p) => p.id)
    ids.forEach((id) => poll(uid, id))
    const t = setInterval(() => ids.forEach((id) => poll(uid, id)), 5000)
    return () => clearInterval(t)
  })
</script>

<span class="la" class:off={!anyRunning && !anyBusy}>
  {#if src && !iconFailed}
    <img {src} alt="" draggable="false" onerror={() => (iconFailed = true)} />
  {:else}
    <Rocket size="46%" strokeWidth={1.9} />
  {/if}
  {#if list.length > 1}<span class="n">{list.length}</span>{/if}
  {#if anyBusy}<span class="spin"><LoaderCircle size={12} /></span>{:else if list.length}<span class="dot" class:on={anyRunning}></span>{/if}
</span>

<style>
  .la { position: relative; width: 100%; height: 100%; display: grid; place-items: center; }
  .la img { width: 62%; height: 62%; object-fit: contain; pointer-events: none; border-radius: 22%; }
  .la.off img, .la.off :global(svg:first-child) { opacity: 0.75; }
  .n { position: absolute; top: 3px; right: 3px; min-width: 14px; height: 14px; padding: 0 3px; border-radius: 7px; background: var(--surface-press); font: 700 9px/14px var(--font-display); text-align: center; }
  .dot { position: absolute; bottom: 4px; right: 5px; width: 7px; height: 7px; border-radius: 4px; background: var(--fg-3); box-shadow: 0 0 0 1.5px rgba(0,0,0,.35); }
  .dot.on { background: #30d158; }
  .spin { position: absolute; bottom: 1px; right: 2px; color: var(--accent); display: grid; animation: r 0.9s linear infinite; }
  @keyframes r { to { transform: rotate(360deg); } }
</style>
