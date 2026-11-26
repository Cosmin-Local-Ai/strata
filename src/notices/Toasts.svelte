<script lang="ts">
  // Strata's notifications: Windows notifications (when Strata shows them instead of Windows) and
  // smart ones from widgets. Newest nearest the dock, at most three; each goes after 8 s unless the
  // pointer is on it. Click: open the app it came from.
  import { X } from '@lucide/svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import { onMount, tick } from 'svelte'
  import { iconUrl, ipc } from '../lib/ipc'
  import { settings, startStores } from '../lib/stores.svelte'
  import type { Toast } from '../lib/notify'
  startStores()
  // Same glass as the cards: the window has the acrylic backdrop, this tints it for the dock style.
  document.body.classList.add('card-surface')

  type Shown = Toast & { at: number; left: number; leaving?: boolean }
  let list = $state<Shown[]>([])
  let root = $state<HTMLDivElement>()
  let hover = $state(false)
  const top = $derived(settings.value?.edge === 'top')
  const seen = new Set<string>()

  async function report() {
    await tick()
    const r = root?.getBoundingClientRect()
    invoke('toast_layout', { width: Math.ceil(r?.width ?? 0), height: list.length ? Math.ceil(r?.height ?? 0) : 0 })
  }
  function push(t: Toast) {
    if (seen.has(t.id)) return
    seen.add(t.id)
    list = [...list, { ...t, at: Date.now(), left: 8000 }].slice(-3)
    report()
  }
  function dismiss(id: string) {
    const t = list.find((x) => x.id === id)
    if (!t || t.leaving) return
    t.leaving = true
    setTimeout(() => { list = list.filter((x) => x.id !== id); report() }, 180)
  }
  function open(t: Shown) {
    if (t.target) ipc.launch(t.target).catch(() => {})
    dismiss(t.id)
  }
  onMount(() => {
    const un = listen<Toast>('toast:push', (e) => push(e.payload))
    invoke<Toast[]>('toast_pending').then((l) => l.forEach(push))
    // Count down only while the pointer is away.
    const timer = setInterval(() => {
      if (hover) return
      for (const t of list) { t.left -= 250; if (t.left <= 0) dismiss(t.id) }
    }, 250)
    return () => { un.then((f) => f()); clearInterval(timer) }
  })
  const shown = $derived(top ? [...list].reverse() : list)
</script>

<div class="stack" class:top bind:this={root} role="presentation" onmouseenter={() => (hover = true)} onmouseleave={() => (hover = false)}>
  {#each shown as t (t.id)}
    <div class="toast" class:leaving={t.leaving} role="button" tabindex="-1" onclick={() => open(t)} onkeydown={() => {}}>
      <div class="from">
        {#if t.target}<img src={iconUrl(t.target, 32)} alt="" onerror={(e) => ((e.currentTarget as HTMLImageElement).style.display = 'none')} />{/if}
        <span>{t.app}</span>
      </div>
      <strong>{t.title}</strong>
      {#if t.body}<p>{t.body}</p>{/if}
      <button class="x" aria-label="Dismiss" onclick={(e) => { e.stopPropagation(); dismiss(t.id) }}><X size={15} /></button>
    </div>
  {/each}
</div>

<style>
  :global(html), :global(body) { overflow: hidden; }
  /* One glass panel (the window's acrylic, tinted by the dock style); notifications separated by a line. */
  .stack { display: flex; flex-direction: column; width: 440px; }
  .toast { position: relative; display: flex; flex-direction: column; gap: 3px; padding: 13px 46px 14px 18px; color: var(--fg); cursor: default;
           transition: background var(--t-fast); animation: in 220ms var(--ease-out); }
  .toast + .toast { border-top: 1px solid var(--border); }
  .top .toast { animation-name: in-top; }
  .toast:hover { background: var(--surface-hover); }
  .toast.leaving { opacity: 0; transition: opacity 160ms; }
  @keyframes in { from { opacity: 0.4; transform: translateY(8px); } }
  @keyframes in-top { from { opacity: 0.4; transform: translateY(-8px); } }
  .from { display: flex; align-items: center; gap: 6px; font-size: 11.5px; color: var(--fg-3); }
  .from img { width: 14px; height: 14px; object-fit: contain; }
  strong { font: 600 15px var(--font-display); line-height: 1.3; overflow-wrap: anywhere; }
  p { margin: 0; font-size: 13px; line-height: 1.4; color: var(--fg-2); white-space: pre-line; overflow: hidden; display: -webkit-box; -webkit-line-clamp: 3; line-clamp: 3; -webkit-box-orient: vertical; }
  .x { position: absolute; top: 11px; right: 11px; width: 26px; height: 26px; border-radius: 13px; display: grid; place-items: center; color: var(--fg-3); }
  .x:hover { background: var(--surface-press); color: var(--fg); }
</style>
