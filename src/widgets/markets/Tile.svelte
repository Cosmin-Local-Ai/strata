<script lang="ts">
  // Currency (ECB rates via frankfurter.app, no key) · Crypto (Binance public prices, no key).
  import { TrendingDown, TrendingUp } from '@lucide/svelte'
  import { poll } from '../../lib/poll'
  import { invoke } from '@tauri-apps/api/core'
  // Public JSON through Strata's own request path (no CORS/redirect issues in the page).
  const getJson = <T = any>(url: string) => invoke<T>('http_get', { url })
  const FX = 'https://api.frankfurter.dev/v1'

  import type { TileProps } from '../types'
  let { item }: TileProps = $props()
  const id = $derived(item.widget)

  const from = $derived(item.options.from ?? 'EUR')
  const to = $derived(item.options.to ?? 'RON')
  let rate = $state<number | null>(null)
  $effect(() => {
    if (id !== 'currency') return
    return poll(`fx:${from}`, 60 * 60_000, async () => (await getJson(`${FX}/latest?base=${from}`)).rates, (r: any) => { rate = r?.[to] ?? null })
  })

  type Tick = { symbol: string; lastPrice: string; priceChangePercent: string }
  const pairs = $derived<string[]>(item.options.pairs ?? ['BTCUSDT'])
  let ticks = $state<Tick[]>([])
  $effect(() => {
    if (id !== 'crypto') return
    const q = encodeURIComponent(JSON.stringify(pairs))
    return poll(`crypto:${pairs.join(',')}`, 30_000, async () => await getJson(`https://api.binance.com/api/v3/ticker/24hr?symbols=${q}`), (v: any) => { if (Array.isArray(v)) { ticks = v; localStorage.setItem('crypto:last', JSON.stringify(v)) } })
  })
  const money = (s: string) => { const n = +s; return n >= 1000 ? n.toLocaleString(undefined, { maximumFractionDigits: 0 }) : n.toPrecision(4) }
</script>

{#if id === 'currency'}
  <span class="fx"><small>1 {from}</small><strong>{rate ? rate.toFixed(rate < 10 ? 4 : 2) : '…'} {to}</strong></span>
{:else if id === 'crypto'}
  <span class="cr">
    {#each ticks.length ? ticks : pairs.map((p) => ({ symbol: p, lastPrice: '', priceChangePercent: '0' })) as t (t.symbol)}
      <span class="one"><small>{t.symbol.replace(/USDT$|USDC$|FDUSD$|EUR$/, '')}</small>
      <strong>{t.lastPrice ? money(t.lastPrice) : '…'}</strong>
      {#if t.lastPrice}<span class="ch" class:down={+t.priceChangePercent < 0}>{#if +t.priceChangePercent < 0}<TrendingDown size={11} />{:else}<TrendingUp size={11} />{/if}{Math.abs(+t.priceChangePercent).toFixed(1)}%</span>{/if}</span>
    {/each}
  </span>
{/if}

<style>
  .one { display: flex; align-items: center; gap: 5px; }
  .fx, .cr { display: flex; align-items: center; gap: 6px; padding: 0 12px; height: 100%; white-space: nowrap; }
  small { font-size: 11px; color: var(--fg-2); font-weight: 600; }
  strong { font: 600 13px var(--font-display); font-variant-numeric: tabular-nums; }
  .ch { display: flex; align-items: center; gap: 2px; font-size: 11px; color: var(--good); font-variant-numeric: tabular-nums; }
  .ch.down { color: var(--danger); }
</style>
