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
  document.body.style.background = 'transparent'

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
  :global(html), :global(body) { background: transparent !important; overflow: hidden; }
  /* Separate cards, tinted for the dock style (Liquid Glass: translucent dark; Black: black; White: light). */
  .stack { display: inline-flex; flex-direction: column; gap: 8px; padding: 2px 2px 14px; width: 460px; --bg: rgba(24, 24, 28, 0.94); --edge: rgba(255, 255, 255, 0.1); }
  :global(:root[data-style='liquidGlass']) .stack { --bg: rgba(26, 26, 32, 0.9); --edge: rgba(255, 255, 255, 0.16); }
  :global(:root[data-style='iosGlass']) .stack { --bg: rgba(46, 46, 54, 0.9); --edge: rgba(255, 255, 255, 0.18); }
  :global(:root[data-style='clear']) .stack { --bg: rgba(20, 20, 24, 0.72); --edge: rgba(255, 255, 255, 0.14); }
  :global(:root[data-style='black']) .stack { --bg: rgba(8, 8, 10, 0.97); --edge: rgba(255, 255, 255, 0.08); }
  :global(:root[data-style='midnight']) .stack { --bg: rgba(12, 18, 46, 0.94); --edge: rgba(120, 150, 255, 0.2); }
  :global(:root[data-style='white']) .stack { --bg: rgba(250, 250, 252, 0.96); --edge: rgba(0, 0, 0, 0.08); }
  .toast { position: relative; display: flex; flex-direction: column; gap: 3px; padding: 12px 44px 13px 16px; border-radius: 18px;
           background: var(--bg); border: 1px solid var(--edge); color: var(--fg); cursor: default;
           box-shadow: 0 10px 30px rgba(0, 0, 0, 0.35); animation: in 220ms var(--ease-out); transition: filter var(--t-fast); }
  .top .toast { animation-name: in-top; }
  .toast:hover { filter: brightness(1.08); }
  .toast.leaving { opacity: 0; transform: translateY(6px); transition: opacity 160ms, transform 160ms; }
  @keyframes in { from { opacity: 0.4; transform: translateY(10px); } }
  @keyframes in-top { from { opacity: 0.4; transform: translateY(-10px); } }
  .from { display: flex; align-items: center; gap: 6px; font-size: 11.5px; color: var(--fg-3); }
  .from img { width: 14px; height: 14px; object-fit: contain; }
  strong { font: 600 15px var(--font-display); line-height: 1.3; overflow-wrap: anywhere; }
  p { margin: 0; font-size: 13px; line-height: 1.4; color: var(--fg-2); white-space: pre-line; overflow: hidden; display: -webkit-box; -webkit-line-clamp: 3; line-clamp: 3; -webkit-box-orient: vertical; }
  .x { position: absolute; top: 10px; right: 10px; width: 26px; height: 26px; border-radius: 13px; display: grid; place-items: center; color: var(--fg-3); }
  .x:hover { background: var(--surface-press); color: var(--fg); }
</style>
