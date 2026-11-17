<script lang="ts">
  // The shared card/menu window. Renders whatever card the dock asked for, reports its size so the
  // Rust side can place it over the tile, and closes on Esc (focus loss is handled natively).
  import { listen } from '@tauri-apps/api/event'
  import { onMount, tick } from 'svelte'
  import { ipc } from '../lib/ipc'
  import { settings, startStores } from '../lib/stores.svelte'
  import { registry } from '../widgets'
  import type { MenuItem } from '../widgets/types'
  import Previews from './Previews.svelte'

  type Open = { uid: string; widget: string; kind: 'card' | 'menu' | 'preview'; payload: any; dock: string; edge: 'top' | 'bottom'; light: boolean }
  startStores()
  document.body.classList.add('card-surface')
  let open = $state<Open | null>(null)
  let root: HTMLDivElement | undefined = $state()
  let gen = $state(0)

  const item = $derived(open ? settings.value?.items.find((i) => i.uid === (open!.kind === 'menu' ? open!.payload?.uid : open!.uid)) : undefined)
  const Card = $derived(open && open.kind === 'card' ? registry[open.widget]?.card : undefined)

  function close() { ipc.cardClose() }
  function act(action: string, data?: unknown) {
    if (open && item) ipc.cardAction(open.dock, item.uid, action, data ?? null)
  }
  function menu(m: MenuItem) {
    if ('separator' in m || m.disabled || !open) return
    ipc.cardAction(open.dock, open.payload.uid, 'menu:' + m.action)
    close()
  }

  $effect(() => {
    if (!root) return
    const ro = new ResizeObserver(() => {
      if (open && root) ipc.cardReady(Math.ceil(root.offsetWidth), Math.ceil(root.offsetHeight))
    })
    ro.observe(root)
    return () => ro.disconnect()
  })

  onMount(() => {
    const subs = [
      listen<Open>('card:open', async (e) => {
        open = e.payload
        gen++
        document.documentElement.classList.toggle('light', e.payload.light)
        await tick()
        if (root) ipc.cardReady(Math.ceil(root.offsetWidth), Math.ceil(root.offsetHeight))
        // Images (cover art, icons) may still be loading; re-measure once they have.
        const imgs = [...(root?.querySelectorAll('img') ?? [])].filter((i) => !i.complete)
        if (imgs.length) Promise.all(imgs.map((i) => new Promise((r) => { i.onload = i.onerror = r }))).then(() => {
          if (root) ipc.cardReady(Math.ceil(root.offsetWidth), Math.ceil(root.offsetHeight))
        })
      }),
      listen('card:close', () => (open = null)),
    ]
    const esc = (e: KeyboardEvent) => { if (e.key === 'Escape') close() }
    window.addEventListener('keydown', esc)
    return () => { subs.forEach((p) => p.then((f) => f())); window.removeEventListener('keydown', esc) }
  })
</script>

{#if open}
  {#key gen}
    <div class="root" class:up={open.edge === 'bottom'} bind:this={root}>
      {#if open.kind === 'preview'}
        <Previews payload={open.payload} uid={open.uid} />
      {:else if open.kind === 'menu'}
        <ul class="menu">
          {#each open.payload.items as m}
            {#if 'separator' in m}<li class="sep"></li>
            {:else}<li><button class:danger={m.danger} disabled={m.disabled} onclick={() => menu(m)}><span class="chk">{m.checked ? '✓' : ''}</span>{m.label}</button></li>{/if}
          {/each}
        </ul>
      {:else if Card && item}
        <Card {item} payload={open.payload} dock={open.dock} {close} {act} />
      {:else}
        <p class="missing">This widget has no card yet.</p>
      {/if}
    </div>
  {/key}
{/if}

<style>
  .root { display: inline-block; animation: in 180ms var(--ease-out) 50ms both; }
  .root.up { animation-name: in-up; }
  @keyframes in { from { opacity: 0; transform: translateY(-8px) scale(0.97); } }
  @keyframes in-up { from { opacity: 0; transform: translateY(8px) scale(0.97); } }
  .menu { list-style: none; margin: 0; padding: 6px; min-width: 200px; }
  .menu button { width: 100%; text-align: left; padding: 6px 10px 6px 6px; border-radius: 7px; font-size: 13px; display: flex; gap: 4px; }
  .chk { width: 16px; text-align: center; flex: none; }
  .menu button:hover:not(:disabled) { background: var(--accent); color: white; }
  .menu button.danger:hover { background: var(--danger); }
  .menu button:disabled { color: var(--fg-3); }
  .sep { height: 1px; background: var(--border); margin: 5px 6px; }
  .missing { padding: 14px; margin: 0; color: var(--fg-2); }
</style>
