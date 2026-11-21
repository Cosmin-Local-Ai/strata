<script lang="ts">
  // Cloud Storage tiles. Square: a cloud with one usage bar per account. Wide: each account's icon
  // and percentage. Large: each account with a bar and "used of total". Click: the card (details per
  // account; click an account to open it on the web).
  import { Cloud, HardDrive } from '@lucide/svelte'
  import type { TileProps } from '../types'
  import { cloud, pct, size } from './cloud.svelte'
  let { item }: TileProps = $props()
  const ids = $derived<string[]>(item.options.accounts ?? ['drive', 'digi'])
  const box = cloud(() => ids, () => String(item.options.digiEmail ?? ''))
  const accts = $derived(box.accounts.filter((a) => ids.includes(a.id)))
  const color = (id: string) => (id === 'drive' ? '#1fa463' : '#1e88e5')
  const id = $derived(item.widget)
</script>

{#if id === 'cloud-storage'}
  <span class="sq">
    <Cloud size="46%" strokeWidth={1.9} />
    <span class="bars">{#each accts as a (a.id)}<i style="--p:{pct(a) * 100}%; --c:{color(a.id)}" class:off={a.state !== 'ok'}></i>{/each}</span>
  </span>
{:else if id === 'cloud-storage-wide'}
  <span class="row">
    {#each accts as a (a.id)}
      <span class="acc" style="--c:{color(a.id)}">{#if a.id === 'drive'}<HardDrive size={15} />{:else}<Cloud size={15} />{/if}<b>{a.state === 'ok' ? `${Math.round(pct(a) * 100)}%` : '–'}</b></span>
    {:else}<Cloud size={16} />{/each}
  </span>
{:else}
  <span class="lg">
    {#each accts as a (a.id)}
      <span class="line" style="--c:{color(a.id)}">
        <span class="nm">{a.id === 'drive' ? 'Drive' : 'Digi'}</span>
        <span class="bar"><i style="width:{pct(a) * 100}%"></i></span>
        <span class="val">{a.state === 'ok' ? `${size(a.used)} / ${size(a.total)}` : a.state === 'connect' ? 'Connect' : 'Error'}</span>
      </span>
    {:else}<span class="line"><Cloud size={14} /> Cloud</span>{/each}
  </span>
{/if}

<style>
  .sq { position: relative; width: 100%; height: 100%; display: grid; place-items: center; }
  .bars { position: absolute; left: 22%; right: 22%; bottom: 14%; display: flex; flex-direction: column; gap: 2px; }
  .bars i { display: block; height: 3px; border-radius: 2px; background: linear-gradient(90deg, var(--c) var(--p), var(--surface-press) var(--p)); }
  .bars i.off { opacity: 0.35; }
  .row { display: flex; align-items: center; gap: 12px; padding: 0 12px; height: 100%; }
  .acc { display: flex; align-items: center; gap: 5px; color: var(--c); }
  .acc b { font: 600 12.5px var(--font-display); color: var(--fg); font-variant-numeric: tabular-nums; }
  .lg { display: flex; flex-direction: column; justify-content: center; gap: 4px; padding: 0 12px; height: 100%; min-width: 210px; }
  .line { display: grid; grid-template-columns: 34px 1fr auto; align-items: center; gap: 7px; font-size: 11px; }
  .nm { color: var(--fg-2); font-weight: 600; }
  .bar { height: 5px; border-radius: 3px; background: var(--surface-press); overflow: hidden; }
  .bar i { display: block; height: 100%; background: var(--c); border-radius: 3px; }
  .val { color: var(--fg-2); font-variant-numeric: tabular-nums; white-space: nowrap; }
</style>
