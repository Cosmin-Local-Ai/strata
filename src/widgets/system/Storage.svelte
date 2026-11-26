<script lang="ts">
  // Storage card: a drive's free space up front, what fills it (top-level folders and files, sized
  // on request, kept 5 minutes), and the other drives one click away. Click an item to open it.
  import { File, Folder, FolderOpen, HardDrive, RefreshCw, Usb } from '@lucide/svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import { ipc } from '../../lib/ipc'
  import { gb, stats, type Drive } from './stats.svelte'
  let { picked, onpick }: { picked: string | undefined; onpick: (mount: string) => void } = $props()

  type Item = { name: string; path: string; dir: boolean; bytes: number }
  type Scan = { root: string; done: number; total: number; items: Item[]; finished: boolean; elapsedMs: number; at: number }
  const COLORS = ['#ff9f0a', '#0a84ff', '#30d158', '#bf5af2', '#ff375f', '#64d2ff', '#ffd60a']

  const drives = $derived<Drive[]>(stats.value?.drives ?? [])
  const d = $derived(drives.find((x) => x.mount === picked) ?? drives.find((x) => x.mount.toUpperCase().startsWith('C')) ?? drives[0])
  const used = $derived(d ? d.total - d.free : 0)
  const pct = $derived(d ? used / Math.max(1, d.total) : 0)

  let scan = $state<Scan | null>(null)
  let skipped = $state(false)
  const same = (a: string, b: string) => a.replace(/[\\/]+$/, '').toUpperCase() === b.replace(/[\\/]+$/, '').toUpperCase()
  $effect(() => {
    const root = d?.mount
    if (!root) return
    scan = null; skipped = false
    invoke<Scan | null>('storage_scan_cached', { root }).then((s) => { if (s && same(s.root, root)) scan = s })
  })
  $effect(() => {
    const un = listen<Scan>('storage:scan', (e) => { if (d && same(e.payload.root, d.mount)) scan = e.payload })
    return () => { un.then((f) => f()) }
  })
  const start = () => d && invoke('storage_scan', { root: d.mount })

  const items = $derived(scan?.items.filter((i) => i.bytes > 0) ?? [])
  const top = $derived(items.slice(0, 6))
  const other = $derived(Math.max(0, used - top.reduce((n, i) => n + i.bytes, 0)))
  const big = $derived(Math.max(1, items[0]?.bytes ?? 1))
  const size = (b: number) => b >= 1024 ** 3 ? gb(b) : b >= 1024 ** 2 ? `${(b / 1024 ** 2).toFixed(1)} MB` : `${Math.max(1, Math.round(b / 1024))} KB`
  const share = (b: number) => { const p = (b / Math.max(1, used)) * 100; return p < 1 ? '<1%' : `${Math.round(p)}%` }
  const ago = (ms: number) => { const m = Math.round((Date.now() - ms) / 60000); return m < 1 ? 'just now' : `${m} min ago` }
  function open(i: Item) { if (i.dir) ipc.launch(i.path); else invoke('reveal', { target: i.path }) }
</script>

<div class="st">
  {#if d}
    <div class="hd">
      <span class="logo">{#if d.removable}<Usb size={20} />{:else}<HardDrive size={20} />{/if}</span>
      <span class="tt"><strong>Storage</strong><small>{d.name || 'Local Disk'} ({d.mount.replace(/\\$/, '')}) · {gb(d.total)}</small></span>
      {#if scan?.finished}<button class="ic" title="Scan again" onclick={() => { scan = null; invoke('storage_scan', { root: d.mount, force: true }) }}><RefreshCw size={14} /></button>{/if}
    </div>
    <div class="free"><span><b>{gb(d.free)}</b> available</span><span class="u">{gb(used)} used · {Math.round(pct * 100)}%</span></div>
    {#if scan}
      <div class="stack" title="What fills the drive">
        {#each top as i, n (i.path)}<i style="width:{(i.bytes / Math.max(1, d.total)) * 100}%; background:{COLORS[n]}"></i>{/each}
        <i style="width:{(other / Math.max(1, d.total)) * 100}%; background: var(--fg-3)"></i>
      </div>
      <div class="legend">
        {#if !top.length}<span>Measuring…</span>{/if}
        {#each top.slice(0, 4) as i, n (i.path)}<span><em style="background:{COLORS[n]}"></em>{i.name}</span>{/each}
        <span><em style="background: var(--fg-3)"></em>Other</span>
      </div>
    {:else}
      <div class="bar"><i style="width:{pct * 100}%" class:full={pct > 0.9}></i></div>
    {/if}

    {#if drives.length > 1}
      <div class="chips">
        {#each drives as x (x.mount)}
          <button class:on={x.mount === d.mount} onclick={() => onpick(x.mount)} title="{gb(x.free)} free of {gb(x.total)}">
            {#if x.removable}<Usb size={12} />{:else}<HardDrive size={12} />{/if}{x.mount.replace(/\\$/, '')}
            <span class="mini"><i style="width:{(1 - x.free / Math.max(1, x.total)) * 100}%"></i></span>
          </button>
        {/each}
      </div>
    {/if}

    <div class="sep"></div>
    {#if !scan && !skipped}
      <div class="ask">
        <span class="big"><HardDrive size={22} /></span>
        <strong>See what fills your drive</strong>
        <p>A scan reads every folder on drive {d.mount.replace(/[:\\]+$/, '')}: usually 10–30 seconds of disk work. Results are kept for 5 minutes.</p>
        <div class="row"><button class="primary" onclick={start}>Scan now</button><button onclick={() => (skipped = true)}>Not now</button></div>
      </div>
    {:else if scan}
      <ul>
        {#each items as i, n (i.path)}
          <li><button onclick={() => open(i)} title="Open {i.path}">
            <span class="fi" style="--c:{COLORS[n] ?? 'var(--fg-3)'}">{#if i.dir}<Folder size={14} />{:else}<File size={14} />{/if}</span>
            <span class="nm"><span class="l1"><strong>{i.name}</strong><b>{size(i.bytes)}</b></span>
              <span class="l2"><span class="track"><i style="width:{(i.bytes / big) * 100}%; background:{COLORS[n] ?? 'var(--fg-3)'}"></i></span><small>{share(i.bytes)}</small></span></span>
          </button></li>
        {/each}
      </ul>
    {/if}
    <div class="ft">
      <span>{#if scan && !scan.finished}Scanning {scan.done} of {scan.total} top-level items · {(scan.elapsedMs / 1000).toFixed(1)}s
        {:else if scan}Scanned {ago(scan.at)} · {(scan.elapsedMs / 1000).toFixed(1)}s{:else}Nothing is measured until you start the scan.{/if}</span>
      <button onclick={() => ipc.launch('shell:MyComputerFolder')}><FolderOpen size={13} /> Open This PC</button>
    </div>
  {:else}
    <p class="muted">Reading your drives…</p>
  {/if}
</div>

<style>
  .st { width: 400px; display: flex; flex-direction: column; gap: 10px; }
  .hd { display: flex; align-items: center; gap: 11px; }
  .logo { width: 40px; height: 40px; border-radius: 11px; display: grid; place-items: center; color: white; background: linear-gradient(160deg, #ffb340, #ff8a00); flex: none; }
  .tt { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .tt strong { font: 600 15px var(--font-display); }
  .tt small { font-size: 11.5px; color: var(--fg-3); }
  .ic { width: 30px; height: 30px; border-radius: 15px; display: grid; place-items: center; background: var(--surface); color: var(--fg-2); }
  .ic:hover { background: var(--surface-hover); color: var(--fg); }
  .free { display: flex; align-items: baseline; justify-content: space-between; gap: 8px; color: var(--fg-3); font-size: 12px; }
  .free b { font: 700 26px var(--font-display); color: var(--fg); margin-right: 4px; font-variant-numeric: tabular-nums; }
  .free .u { font-variant-numeric: tabular-nums; }
  .stack, .bar { display: flex; height: 8px; border-radius: 4px; overflow: hidden; background: var(--surface-press); gap: 1px; }
  .stack i { display: block; height: 100%; min-width: 2px; }
  .bar i { display: block; height: 100%; background: var(--accent); border-radius: 4px; }
  .bar i.full { background: var(--danger); }
  .legend { display: flex; flex-wrap: wrap; gap: 4px 12px; font-size: 11.5px; color: var(--fg-2); }
  .legend span { display: inline-flex; align-items: center; gap: 5px; max-width: 150px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .legend em { width: 8px; height: 8px; border-radius: 4px; flex: none; }
  .chips { display: flex; flex-wrap: wrap; gap: 5px; }
  .chips button { display: inline-flex; align-items: center; gap: 5px; padding: 4px 8px; border-radius: 8px; background: var(--surface); font-size: 12px; color: var(--fg-2); }
  .chips button:hover { background: var(--surface-hover); }
  .chips button.on { background: color-mix(in srgb, var(--accent) 22%, transparent); color: var(--fg); }
  .mini { width: 26px; height: 4px; border-radius: 2px; background: var(--surface-press); overflow: hidden; }
  .mini i { display: block; height: 100%; background: var(--fg-2); }
  .sep { height: 1px; background: var(--border); }
  .ask { display: flex; flex-direction: column; align-items: center; text-align: center; gap: 7px; padding: 8px 10px 4px; }
  .ask .big { width: 46px; height: 46px; border-radius: 13px; display: grid; place-items: center; color: #ff9f0a; background: color-mix(in srgb, #ff9f0a 14%, transparent); }
  .ask strong { font-size: 14px; }
  .ask p { margin: 0; font-size: 12px; color: var(--fg-3); line-height: 1.45; }
  .row { display: flex; gap: 8px; margin-top: 4px; }
  .row button, .ft button { padding: 7px 16px; border-radius: 18px; background: var(--surface-press); font-size: 12.5px; display: inline-flex; align-items: center; gap: 6px; }
  .row button:hover, .ft button:hover { background: var(--surface-hover); }
  .row .primary { background: linear-gradient(160deg, #ffb340, #ff8a00); color: white; }
  .row .primary:hover { filter: brightness(1.08); background: linear-gradient(160deg, #ffb340, #ff8a00); }
  /* Fixed sizes while the scan fills in: a card that changes height every update makes the window
     resize (and flash) a few times a second. */
  ul { list-style: none; margin: 0; padding: 0; height: 330px; overflow-y: auto; display: flex; flex-direction: column; gap: 2px; }
  .legend { min-height: 34px; align-content: flex-start; }
  li button { width: 100%; display: flex; align-items: center; gap: 10px; padding: 7px 6px; border-radius: 10px; text-align: left; }
  li button:hover { background: var(--surface-hover); }
  .fi { width: 30px; height: 30px; border-radius: 9px; display: grid; place-items: center; flex: none; color: var(--c); background: color-mix(in srgb, var(--c) 15%, transparent); }
  .nm { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 4px; }
  .l1 { display: flex; justify-content: space-between; gap: 8px; }
  .l1 strong { font-size: 13px; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .l1 b { font-size: 12.5px; font-variant-numeric: tabular-nums; flex: none; }
  .l2 { display: flex; align-items: center; gap: 8px; }
  .track { flex: 1; height: 4px; border-radius: 2px; background: var(--surface-press); overflow: hidden; }
  .track i { display: block; height: 100%; border-radius: 2px; }
  .l2 small { width: 34px; text-align: right; font-size: 11px; color: var(--fg-3); font-variant-numeric: tabular-nums; }
  .ft { display: flex; align-items: center; justify-content: space-between; gap: 8px; padding-top: 8px; border-top: 1px solid var(--border); font-size: 11.5px; color: var(--fg-3); }
  .muted { color: var(--fg-3); font-size: 12px; margin: 0; }
</style>
