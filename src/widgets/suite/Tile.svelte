<script lang="ts">
  // Microsoft Office / Google Workspace. Default: one square tile (a 2×2 glyph in the suite's colours)
  // that opens the card with every app and pinned / recent documents. Right-click → "Show app icons":
  // one click per app straight from the dock, plus a folder button for the card.
  import { FolderOpen } from '@lucide/svelte'
  import { iconUrl, ipc } from '../../lib/ipc'
  import type { TileProps } from '../types'
  import { GOOGLE, GOOGLE_DEFAULT, OFFICE_DEFAULT } from './suite'
  import { officeApps } from './apps.svelte'
  let { item }: TileProps = $props()
  const google = $derived(item.widget === 'google-workspace')
  const compact = $derived(item.options.style !== 'apps')
  const chosen = $derived<string[]>(item.options.apps ?? (google ? GOOGLE_DEFAULT : OFFICE_DEFAULT))
  const office = officeApps()
  // Default set: one Outlook (classic when installed, else the new one).
  const msApps = $derived(office.value.filter((a) => chosen.includes(a.id) && !(a.id === 'outlook-new' && !item.options.apps && office.value.some((x) => x.id === 'outlook'))))
  const gApps = $derived(GOOGLE.filter((a) => chosen.includes(a.id)))
  const colors = $derived(google ? ['#4285f4', '#ea4335', '#fbbc04', '#34a853'] : ['#2b7cd3', '#21a366', '#d35230', '#7719aa'])
  const cells = [[3, 3], [13, 3], [3, 13], [13, 13]]
  const open = (e: MouseEvent, target: string) => { e.stopPropagation(); ipc.launch(target).catch(() => {}) }
</script>

{#if compact}
  <span class="sq">
    <svg class="g" viewBox="0 0 24 24" aria-hidden="true">
      {#each colors as c, i}
        {#if google}<circle cx={cells[i][0] + 4} cy={cells[i][1] + 4} r="4" fill={c} />{:else}<rect x={cells[i][0]} y={cells[i][1]} width="8" height="8" rx="2" fill={c} />{/if}
      {/each}
    </svg>
  </span>
{:else}
  <span class="suite">
    {#if google}
      {#each gApps as a (a.id)}
        <button class="ap" title={a.name} onclick={(e) => open(e, a.url)} style="--c:{a.color}"><a.icon size="58%" strokeWidth={2} /></button>
      {/each}
    {:else}
      {#each msApps as a (a.id)}
        <button class="ap" title={a.name} onclick={(e) => open(e, a.target)}><img src={iconUrl(a.target, 64)} alt="" draggable="false" /></button>
      {:else}
        <span class="none">Office</span>
      {/each}
    {/if}
    <span class="docs" title="Documents"><FolderOpen size="52%" strokeWidth={1.9} /></span>
  </span>
{/if}

<style>
  .sq { width: var(--tile); height: 100%; display: grid; place-items: center; }
  .g { width: 50%; height: 50%; }
  .suite { display: flex; align-items: center; gap: 2px; height: 100%; padding: 0 4px; }
  .ap, .docs { width: calc(var(--tile) * 0.8); height: calc(var(--tile) * 0.8); border-radius: calc(var(--tile-r) * 0.8); display: grid; place-items: center; flex: none; }
  .ap { color: var(--c, var(--fg)); }
  .ap:hover { background: var(--tile-bg-hover); }
  .ap:active { transform: scale(0.9); }
  .ap img { width: 70%; height: 70%; object-fit: contain; pointer-events: none; }
  .docs { color: var(--fg-2); }
  .none { font-size: 12px; color: var(--fg-2); padding: 0 6px; }
</style>
