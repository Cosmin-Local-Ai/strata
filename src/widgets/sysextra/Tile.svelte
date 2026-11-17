<script lang="ts">
  // Temperature, Battery Ring / Core / Dots / Gauge, Brightness, Do Not Disturb, Quick Toggles.
  import { BellOff, Bell, Plug, Sun, Thermometer, ToggleRight } from '@lucide/svelte'
  import { invoke } from '@tauri-apps/api/core'
  import Ring from '../system/Ring.svelte'
  import type { TileProps } from '../types'
  import { pickDefault, readSensors, toUnit, type Sensor } from './temp.svelte'
  let { item }: TileProps = $props()
  const id = $derived(item.widget)

  // ---- temperature (every 5 s while visible; 10-minute history shared with the card)
  let sensors = $state<Sensor[]>([])
  let tempErr = $state(false)
  const sensor = $derived(sensors.find((s) => s.id === item.options.sensor) ?? pickDefault(sensors))
  const f = $derived(item.options.unit === 'F')
  $effect(() => {
    if (id !== 'temperature') return
    const tick = async () => {
      if (document.visibilityState !== 'visible') return
      try {
        sensors = await readSensors(); tempErr = false
        const s = sensors.find((x) => x.id === item.options.sensor) ?? pickDefault(sensors)
        if (s) {
          const h: [number, number][] = JSON.parse(localStorage.getItem('temp:hist') ?? '[]').filter(([t]: [number]) => Date.now() - t < 600_000)
          h.push([Date.now(), s.value]); localStorage.setItem('temp:hist', JSON.stringify(h))
        }
      } catch { tempErr = true }
    }
    tick()
    const t = setInterval(tick, 5000)
    return () => clearInterval(t)
  })

  // ---- battery
  type B = { present: boolean; percent: number; charging: boolean; plugged: boolean }
  let bat = $state<B | null>(null)
  $effect(() => {
    if (!id.startsWith('battery-')) return
    const load = () => invoke<B>('battery').then((b) => (bat = b))
    load()
    const t = setInterval(load, 30_000)
    return () => clearInterval(t)
  })
  const pct = $derived(bat?.present ? bat.percent : 100)
  const batColor = $derived(pct <= 15 ? 'var(--danger)' : pct <= 30 ? '#ff9f0a' : 'var(--good)')

  // ---- brightness
  let bright = $state<number | null>(null)
  let brightErr = $state('')
  // Hardware (DDC/CI) when the monitor allows it; otherwise software brightness and warmth (colour ramp).
  const soft = $derived(item.options.soft === true || !!brightErr)
  $effect(() => { if (id === 'brightness') invoke<number>('brightness_get').then((v) => (bright = v)).catch((e) => (brightErr = String(e))) })
  $effect(() => {
    if (id !== 'brightness' || !soft) return
    const apply = () => {
      let b = item.options.sb ?? 100, w = item.options.sw ?? 0
      if (item.options.auto) {
        // Warm after sunset until sunrise (sun times from Weather, cached).
        const h = new Date().getHours()
        const night = h >= (item.options.nightFrom ?? 20) || h < (item.options.nightTo ?? 7)
        if (night) { w = Math.max(w, item.options.nightWarmth ?? 55); b = Math.min(b, item.options.nightBright ?? 85) }
      }
      if (b !== 100 || w !== 0) invoke('gamma_set', { brightness: b, warmth: w }).catch(() => {})
      else invoke('gamma_reset')
    }
    apply()
    const t = setInterval(apply, 60_000)
    return () => clearInterval(t)
  })

  // ---- toggles (DND + quick toggles count)
  type T = { id: string; on: boolean; available: boolean }
  let toggles = $state<T[]>([])
  $effect(() => {
    if (id !== 'do-not-disturb' && id !== 'quick-toggles') return
    const load = () => invoke<T[]>('toggles_get').then((t) => (toggles = t))
    load()
    const t = setInterval(load, 10_000)
    const ev = () => load()
    window.addEventListener('toggles:changed', ev)
    return () => { clearInterval(t); window.removeEventListener('toggles:changed', ev) }
  })
  const dnd = $derived(toggles.find((t) => t.id === 'dnd')?.on ?? false)
  const onCount = $derived(toggles.filter((t) => t.available && t.on && (!item.options.toggles || item.options.toggles.includes(t.id))).length)
</script>

{#if id === 'temperature'}
  <span class="row" title={sensor ? `${sensor.hardware} · ${sensor.name}` : 'Temperature'}>
    {#if sensor}
      <Ring value={Math.min(1, sensor.value / 100)} size={30} label={`${Math.round(toUnit(sensor.value, f))}°`} color={sensor.value > 85 ? 'var(--danger)' : sensor.value > 70 ? '#ff9f0a' : 'var(--good)'} />
      <small>{sensor.hardware.split(' ').slice(0, 2).join(' ') || sensor.name}</small>
    {:else}<Thermometer size={18} /><small>{tempErr ? 'No sensors' : '…'}</small>{/if}
  </span>
{:else if id === 'battery-ring'}
  <span class="row"><span class="br" class:charging={bat?.plugged}><Ring value={pct / 100} size={30} color={batColor} label={bat?.present ? `${pct}` : ''} />{#if !bat?.present || bat?.plugged}<span class="spark"><Plug size={11} /></span>{/if}</span></span>
{:else if id === 'battery-core'}
  <span class="row"><span class="cells">{#each Array(12) as _, i}<i class:on={i < Math.round(pct / 100 * 12)} style="--c:{batColor}"></i>{/each}</span><small>{bat?.present ? `${pct}%` : 'AC'}</small></span>
{:else if id === 'battery-dots'}
  <span class="row"><span class="dots">{#each Array(5) as _, i}<i class:on={i < Math.ceil(pct / 20)} style="--c:{batColor}"></i>{/each}</span><small>{bat?.present ? `${pct}%` : 'AC'}</small></span>
{:else if id === 'battery-gauge'}
  <span class="row gauge"><svg viewBox="0 0 44 26" width="44" height="26"><path d="M4 24 A18 18 0 0 1 40 24" fill="none" stroke="var(--surface-press)" stroke-width="3.5" stroke-linecap="round" />
    <path d="M4 24 A18 18 0 0 1 40 24" fill="none" stroke={batColor} stroke-width="3.5" stroke-linecap="round" pathLength="100" stroke-dasharray="{pct} 100" />
    <line x1="22" y1="24" x2={22 + 14 * Math.cos(Math.PI * (1 - pct / 100))} y2={24 - 14 * Math.sin(Math.PI * (1 - pct / 100))} stroke="var(--fg)" stroke-width="1.8" stroke-linecap="round" />
    <text x="2" y="22" font-size="6" fill="var(--fg-3)">E</text><text x="38" y="22" font-size="6" fill="var(--fg-3)">F</text></svg>
    <small>{bat?.present ? `${pct}%` : 'AC'}</small></span>
{:else if id === 'brightness'}
  <span class="row" title={soft ? 'Software brightness and warmth' : 'Scroll to change'}><Sun size={18} /><small>{soft ? `${item.options.sb ?? 100}%${(item.options.sw ?? 0) > 0 || item.options.auto ? ' · warm' : ''}` : bright == null ? '…' : `${bright}%`}</small></span>
{:else if id === 'do-not-disturb'}
  <span class="sq" class:on={dnd} title={dnd ? 'Do not disturb is on' : 'Do not disturb is off'}><BellOff size="46%" strokeWidth={1.9} /></span>
{:else if id === 'quick-toggles'}
  <span class="row"><ToggleRight size={18} /><small>{onCount} on</small></span>
{/if}

<style>
  .row { display: flex; align-items: center; gap: 7px; padding: 0 10px; height: 100%; }
  small { font-size: 11px; color: var(--fg-2); white-space: nowrap; font-variant-numeric: tabular-nums; }
  .br { position: relative; display: grid; }
  .spark { position: absolute; right: -4px; bottom: -3px; width: 15px; height: 15px; border-radius: 8px; display: grid; place-items: center; background: var(--good); color: #000; }
  .br.charging .spark { animation: spark 1.6s ease-in-out infinite; }
  @keyframes spark { 50% { transform: scale(1.18); box-shadow: 0 0 8px var(--good); } }
  .cells { display: grid; grid-template-columns: repeat(6, 5px); grid-template-rows: repeat(2, 9px); gap: 2px; }
  .cells i, .dots i { background: var(--surface-press); border-radius: 2px; }
  .cells i.on, .dots i.on { background: var(--c); }
  .dots { display: flex; gap: 3px; }
  .dots i { width: 7px; height: 7px; border-radius: 4px; }
  .sq { display: grid; place-items: center; width: 100%; height: 100%; }
  .sq { color: var(--fg-3); }
  .sq.on { color: var(--accent); }
</style>
