<script lang="ts">
  // Currency converter (keeps the typed amount; search by code or name) and Crypto pairs editor.
  import { ArrowLeftRight, X } from '@lucide/svelte'
  import type { CardProps } from '../types'
  import Combo from '../../lib/ui/Combo.svelte'
  import { invoke } from '@tauri-apps/api/core'
  // Public JSON through Strata's own request path (no CORS/redirect issues in the page).
  const getJson = <T = any>(url: string) => invoke<T>('http_get', { url })
  const FX = 'https://api.frankfurter.dev/v1'

  let { item, act }: CardProps = $props()
  const id = item.widget

  // Currency
  let names = $state<Record<string, string>>({})
  let from = $state<string>(item.options.from ?? 'EUR')
  let to = $state<string>(item.options.to ?? 'RON')
  let amount = $state<number>(+(localStorage.getItem('fx:amount') ?? '100'))
  let rates = $state<Record<string, number>>({})
  $effect(() => { if (id === 'currency') getJson<Record<string, string>>(`${FX}/currencies`).then((n) => (names = n)) })
  $effect(() => { if (id === 'currency') getJson(`${FX}/latest?base=${from}`).then((j) => (rates = { ...j.rates, [from]: 1 })) })
  $effect(() => localStorage.setItem('fx:amount', String(amount)))
  const result = $derived(rates[to] ? amount * rates[to] : null)
  let openFrom = $state(false), openTo = $state(false)
  $effect(() => { if (openFrom) openTo = false })
  $effect(() => { if (openTo) openFrom = false })
  const opts = $derived(Object.entries(names).map(([c, n]) => ({ value: c, label: n })))
  function save() { act('set-options', { from, to }) }

  // Crypto
  let pairs = $state<string[]>(item.options.pairs ?? ['BTCUSDT'])
  let newPair = $state('')
  function addPair() {
    let p = newPair.trim().toUpperCase().replace(/[^A-Z0-9]/g, '')
    if (p && !/(USDT|USDC|BUSD|FDUSD|BTC|ETH|EUR|TRY|BRL)$/.test(p)) p += 'USDT'
    if (p && !pairs.includes(p) && pairs.length < 4) { pairs = [...pairs, p]; act('set-options', { pairs }) }
    newPair = ''
  }
  type Tick = { symbol: string; lastPrice: string; priceChangePercent: string; highPrice: string; lowPrice: string }
  let ticks = $state<Tick[]>(JSON.parse(localStorage.getItem('crypto:last') ?? '[]').filter((t: Tick) => pairs.includes(t.symbol)))
  let bad = $state<string[]>([])
  $effect(() => {
    if (id !== 'crypto') return
    // One bad symbol makes Binance reject the whole batch: fall back to one request per pair.
    getJson(`https://api.binance.com/api/v3/ticker/24hr?symbols=${encodeURIComponent(JSON.stringify(pairs))}`).then((v) => { if (Array.isArray(v)) { ticks = v; bad = [] } else throw 0 }).catch(() =>
      Promise.all(pairs.map((p) => getJson(`https://api.binance.com/api/v3/ticker/24hr?symbol=${p}`).then((t) => (t?.symbol ? t : null)).catch(() => null))).then((r) => { ticks = r.filter(Boolean); bad = pairs.filter((p, i) => !r[i]) }))
    if (false) getJson(`https://api.binance.com/api/v3/ticker/24hr?symbols=${encodeURIComponent(JSON.stringify(pairs))}`).then((v) => { if (Array.isArray(v)) ticks = v })
  })
</script>

<div class="mc">
  {#if id === 'currency'}
    <input class="amt" type="number" bind:value={amount} />
    <div class="pair">
      <Combo value={from} options={opts} bind:open={openFrom} onpick={(v) => { from = v; openTo = false; save() }} />
      <button class="swap" aria-label="Swap" onclick={() => { [from, to] = [to, from]; save() }}><ArrowLeftRight size={14} /></button>
      <Combo value={to} options={opts} bind:open={openTo} onpick={(v) => { to = v; openFrom = false; save() }} />
    </div>
    <div class="res">{result !== null ? result.toLocaleString(undefined, { maximumFractionDigits: 2 }) : '…'} <small>{to}</small></div>
    <p class="muted">European Central Bank reference rates (updated on working days).</p>
  {:else}
    <ul>
      {#each ticks as t (t.symbol)}
        <li>
          <strong>{t.symbol}</strong>
          <span class="p">{(+t.lastPrice).toLocaleString(undefined, { maximumFractionDigits: +t.lastPrice < 10 ? 4 : 2 })}</span>
          <span class="c" class:down={+t.priceChangePercent < 0}>{(+t.priceChangePercent).toFixed(2)}%</span>
          <button aria-label="Remove" onclick={() => { pairs = pairs.filter((x) => x !== t.symbol); act('set-options', { pairs }) }}><X size={12} /></button>
        </li>
      {/each}
    </ul>
    {#if pairs.length < 4}
      <form class="add" onsubmit={(e) => { e.preventDefault(); addPair() }}><input bind:value={newPair} placeholder="Add a pair, e.g. ETHUSDT" /><button type="submit">Add</button></form>
    {/if}
    {#each bad as b}<p class="err">{b} isn't a Binance pair. <button class="lnk" onclick={() => { pairs = pairs.filter((x) => x !== b); act('set-options', { pairs }) }}>Remove</button></p>{/each}
    <p class="muted">Binance prices, 24 h change. Up to four pairs, all shown on the tile.</p>
  {/if}
</div>

<style>
  .mc { width: 330px; padding: 14px; display: flex; flex-direction: column; gap: 10px; }
  .amt, select, .add input { padding: 7px 10px; border-radius: 9px; border: 1px solid var(--border); background: var(--surface); color: var(--fg); outline: none; min-width: 0; }
  /* No browser spinner arrows; custom dropdown chevron. */
  .amt::-webkit-inner-spin-button, .amt::-webkit-outer-spin-button { -webkit-appearance: none; margin: 0; }
  select { appearance: none; padding-right: 26px; background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%23aaa' stroke-width='2.5' stroke-linecap='round'%3E%3Cpath d='m6 9 6 6 6-6'/%3E%3C/svg%3E"); background-repeat: no-repeat; background-position: right 9px center; text-overflow: ellipsis; }
  select option { background: #222; color: #eee; }
  .amt { font: 600 20px var(--font-display); }
  .pair { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; }
  .pair > :global(.field) { flex: 1 1 0; }
  .pair .swap { width: 30px; height: 30px; border-radius: 15px; display: grid; place-items: center; background: var(--surface-press); }
  .res { font: 600 28px var(--font-display); font-variant-numeric: tabular-nums; }
  .res small { font-size: 14px; color: var(--fg-2); }
  ul { list-style: none; margin: 0; padding: 0; }
  li { display: grid; grid-template-columns: 1fr auto 60px 20px; gap: 8px; align-items: center; padding: 6px 2px; font-size: 13px; font-variant-numeric: tabular-nums; }
  .c { text-align: right; color: var(--good); } .c.down { color: var(--danger); }
  li button { color: var(--fg-3); }
  .add { display: flex; gap: 6px; } .add input { flex: 1; }
  .add button { padding: 6px 12px; border-radius: 9px; background: var(--accent); color: white; }
  .muted { color: var(--fg-3); font-size: 11.5px; margin: 0; }
  .err { color: var(--danger); font-size: 12px; margin: 0; }
  .lnk { color: var(--accent); font-size: 12px; }
</style>
