<script lang="ts">
  // One tile for every adapter widget. Variants by data: ring (AI limits, goals), chart (ids ending in
  // "-chart": weekly number, change and a filled 14-day line), grid (contribution grid), or plain.
  import { useService } from '../../lib/services.svelte'
  import Spark from '../../lib/ui/Spark.svelte'
  import Ring from '../system/Ring.svelte'
  import type { TileProps } from '../types'
  import { ADAPTERS } from './adapters'
  let { item }: TileProps = $props()
  const a = $derived(ADAPTERS[item.widget])
  const chart = $derived(item.widget.endsWith('-chart'))
  // In the Widgets picker (overlay window) previews are static: only widgets on the dock make requests.
  const inPicker = location.hash.startsWith('#overlay')
  const svc = inPicker ? { state: { connected: true, view: null, error: '', loading: false } } : useService(item.widget, ADAPTERS[item.widget], () => item.options, () => ({ tab: chart ? 'week' : ADAPTERS[item.widget].tileTab ?? ADAPTERS[item.widget].tabs?.[0]?.id }))
  const v = $derived(svc.state.view)
  const Icon = $derived(a.icon)
  // "2h 40m" within a day, otherwise "Sat 21".
  function shortReset(r: string | number | null | undefined) {
    if (r == null) return ''
    const t = typeof r === 'number' ? (r < 1e12 ? r * 1000 : r) : Date.parse(r)
    const ms = t - Date.now()
    if (!isFinite(ms)) return ''
    if (ms < 864e5) { const m = Math.max(0, Math.round(ms / 60000)); return `${Math.floor(m / 60)}h ${m % 60}m` }
    return new Date(t).toLocaleDateString(undefined, { weekday: 'short', day: 'numeric' })
  }
</script>

{#if inPicker}
  <span class="t"><Icon size={16} /><span class="txt"><strong>{a.title}</strong><small>{a.preview ? 'Preview' : 'Live data on the dock'}</small></span></span>
{:else if !svc.state.connected}
  <span class="t off"><Icon size={16} /><span class="txt"><strong>{a.title}</strong><small>Click to connect</small></span></span>
{:else if !v}
  <span class="t"><Icon size={16} /><span class="txt"><strong>{svc.state.error ? '!' : '…'}</strong><small title={svc.state.error}>{svc.state.error ? 'Error' : a.title}</small></span></span>
{:else if item.widget === 'github-activity' && v.grid}
  {@const g = v.grid.slice(-15 * 7)}
  {@const m = Math.max(1, ...g)}
  <span class="t"><span class="grid">{#each g as c}<i style="opacity:{c ? 0.3 + 0.7 * (c / m) : 0.12}"></i>{/each}</span><span class="txt"><strong>{v.value}</strong><small>{v.label}</small></span></span>
{:else if chart && v.series}
  <span class="t"><span class="txt"><small>{a.title}</small><strong>{v.value}</strong></span>
    <Spark values={v.series.slice(-14)} width={64} height={26} min={v.seriesFromZero === false ? undefined : 0} />
    {#if v.delta != null}<span class="d" class:neg={v.delta < 0}>{v.delta >= 0 ? '+' : ''}{v.delta.toFixed(0)}%</span>{/if}
  </span>
{:else if v.limits && v.limits.length >= 2 && v.ring != null}
  <span class="t lim">
    {#each v.limits.slice(0, 2) as l}
      <span class="lr"><small class="ln">{l.label.replace(/ \(5 h\)/, '').replace('Codex weekly', 'Weekly').replace('Codex', 'Session')}</small><span class="lb"><i style="width:{Math.min(100, l.pct)}%" class:hot={l.pct > 85}></i></span><small class="lt">{shortReset(l.resets)}</small><strong class="lp">{Math.round(l.pct)}%</strong></span>
    {/each}
  </span>
{:else if v.ring != null}
  <span class="t"><Ring value={v.ring} size={30} label={`${Math.round(v.ring * 100)}`} color={v.ring > 0.85 ? 'var(--danger)' : v.ring > 0.6 ? '#ff9f0a' : 'var(--accent)'} />
    <span class="txt"><strong>{v.value}</strong><small>{v.label ?? a.title}</small></span></span>
{:else}
  <span class="t"><Icon size={16} /><span class="txt"><strong>{v.value}</strong><small>{v.label ?? a.title}</small></span>
    {#if v.delta != null}<span class="d" class:neg={v.delta < 0}>{v.delta >= 0 ? '+' : ''}{v.delta.toFixed(0)}%</span>{/if}
  </span>
{/if}

<style>
  .t { display: flex; align-items: center; gap: 8px; padding: 0 12px; height: 100%; white-space: nowrap; color: var(--fg); }
  .t.off { opacity: 0.75; }
  .txt { display: flex; flex-direction: column; line-height: 1.15; min-width: 0; }
  strong { font: 600 13px var(--font-display); font-variant-numeric: tabular-nums; max-width: 130px; overflow: hidden; text-overflow: ellipsis; }
  small { font-size: 10.5px; color: var(--fg-2); max-width: 130px; overflow: hidden; text-overflow: ellipsis; }
  .d { font-size: 11px; font-weight: 600; color: var(--good); font-variant-numeric: tabular-nums; }
  .d.neg { color: var(--danger); }
  .lim { flex-direction: column; justify-content: center; align-items: stretch; gap: 3px; }
  .lr { display: grid; grid-template-columns: 46px 64px 48px 30px; gap: 6px; align-items: center; }
  .ln { color: var(--fg); font-weight: 600; font-size: 10.5px; }
  .lb { height: 4px; border-radius: 2px; background: var(--surface-press); overflow: hidden; }
  .lb i { display: block; height: 100%; background: var(--good); border-radius: 2px; }
  .lb i.hot { background: var(--danger); }
  .lt { font-size: 10px; text-align: right; }
  .lp { font-size: 11px; text-align: right; }
  .grid { display: grid; grid-template-rows: repeat(7, 4px); grid-auto-flow: column; grid-auto-columns: 4px; gap: 1px; }
  .grid i { background: var(--good); border-radius: 1px; }
</style>
