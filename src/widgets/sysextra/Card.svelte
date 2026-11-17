<script lang="ts">
  // Cards: Temperature (sensor picker, °C/°F, ten-minute history, CPU and memory load), Brightness
  // (slider in the accent colour), Quick Toggles (grid; choose which appear), Battery (details).
  import { BatteryCharging, Bluetooth, Camera, Coffee, Monitor, MonitorSmartphone, Moon, Mic, Plane, Settings2, Sun, Wifi, Leaf, Cable, Radio, Check } from '@lucide/svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { ipc } from '../../lib/ipc'
  import Spark from '../../lib/ui/Spark.svelte'
  import { stats, useStats } from '../system/stats.svelte'
  import type { CardProps } from '../types'
  import { pickDefault, readSensors, toUnit, type Sensor } from './temp.svelte'
  let { item, act }: CardProps = $props()
  const id = item.widget

  // ---- temperature
  let sensors = $state<Sensor[]>([])
  let tempErr = $state(false)
  let sel = $state<string | undefined>(item.options.sensor)
  let f = $state(item.options.unit === 'F')
  let hist = $state<number[]>([])
  $effect(() => {
    if (id !== 'temperature') return
    const unsub = useStats()
    const tick = async () => {
      try { sensors = await readSensors(); tempErr = false } catch { tempErr = true }
      hist = (JSON.parse(localStorage.getItem('temp:hist') ?? '[]') as [number, number][]).filter(([t]) => Date.now() - t < 600_000).map(([, v]) => v)
    }
    tick()
    const t = setInterval(tick, 5000)
    return () => { clearInterval(t); unsub() }
  })
  const sensor = $derived(sensors.find((s) => s.id === sel) ?? pickDefault(sensors))
  const groups = $derived([...new Set(sensors.map((s) => s.hardware))])

  // ---- brightness
  let bright = $state(50)
  let brightErr = $state('')
  let bt: ReturnType<typeof setTimeout> | undefined
  if (id === 'brightness') invoke<number>('brightness_get').then((v) => (bright = v)).catch((e) => (brightErr = String(e)))
  // Software mode (monitor without DDC/CI): brightness + warmth through the colour ramp.
  let sb = $state<number>(item.options.sb ?? 100), sw = $state<number>(item.options.sw ?? 0), auto = $state<boolean>(item.options.auto === true)
  let softErr = $state('')
  let st2: ReturnType<typeof setTimeout> | undefined
  function soft(b: number, w: number) {
    sb = b; sw = w; softErr = ''
    clearTimeout(st2)
    st2 = setTimeout(() => {
      (b === 100 && w === 0 ? invoke('gamma_reset') : invoke('gamma_set', { brightness: b, warmth: w })).catch((e) => (softErr = String(e)))
      act('set-options', { sb: b, sw: w, auto, soft: true })
    }, 120)
  }
  function setBright(v: number) { bright = v; clearTimeout(bt); bt = setTimeout(() => invoke('brightness_set', { value: v }).catch((e) => (brightErr = String(e))), 120) }

  // ---- quick toggles
  type T = { id: string; on: boolean; available: boolean }
  const ALL: { id: string; label: string; icon: any; settings?: string }[] = [
    { id: 'wifi', label: 'Wi-Fi', icon: Wifi }, { id: 'bluetooth', label: 'Bluetooth', icon: Bluetooth },
    { id: 'hotspot', label: 'Hotspot', icon: Radio, settings: 'ms-settings:network-mobilehotspot' }, { id: 'airplane', label: 'Airplane mode', icon: Plane, settings: 'ms-settings:network-airplanemode' },
    { id: 'ethernet', label: 'Ethernet', icon: Cable, settings: 'ms-settings:network-ethernet' }, { id: 'energy', label: 'Energy saver', icon: Leaf, settings: 'ms-settings:batterysaver' },
    { id: 'dnd', label: 'Do not disturb', icon: Moon }, { id: 'microphone', label: 'Microphone', icon: Mic }, { id: 'camera', label: 'Camera', icon: Camera },
    { id: 'keep-awake', label: 'Keep awake', icon: Coffee }, { id: 'desktop-icons', label: 'Desktop icons', icon: Monitor }, { id: 'display', label: 'Display mode', icon: MonitorSmartphone },
  ]
  let toggles = $state<T[]>([])
  let chosen = $state<string[]>(item.options.toggles ?? ['wifi', 'bluetooth', 'dnd', 'microphone', 'camera', 'keep-awake', 'desktop-icons', 'display'])
  let editing = $state(false)
  let tErr = $state('')
  const loadT = () => invoke<T[]>('toggles_get').then((t) => (toggles = t))
  if (id === 'quick-toggles') loadT()
  const stOf = (tid: string) => toggles.find((t) => t.id === tid)
  const shown = $derived(ALL.filter((x) => chosen.includes(x.id) && (stOf(x.id)?.available ?? true)))
  async function flip(x: (typeof ALL)[number]) {
    tErr = ''
    if (x.id === 'display') return invoke('sys_action', { action: 'project' })
    if (x.settings) return ipc.launch(x.settings)
    const s = stOf(x.id)
    try { await invoke('toggle_set', { id: x.id, on: !(s?.on ?? false) }); await loadT() } catch (e) { tErr = String(e) }
  }

  // ---- battery
  type B = { present: boolean; percent: number; charging: boolean; plugged: boolean }
  let bat = $state<B | null>(null)
  if (id.startsWith('battery-')) invoke<B>('battery').then((b) => (bat = b))
</script>

<div class="sc">
  {#if id === 'temperature'}
    {#if tempErr}
      <h4>Temperature</h4>
      <p class="muted">Strata reads temperatures from LibreHardwareMonitor so it never loads a sensor or GPU driver itself.</p>
      <ol class="muted"><li>Install LibreHardwareMonitor and run it (it can start with Windows, minimised).</li><li>Options → Remote Web Server → Run (port 8085).</li></ol>
      <button class="link" onclick={() => ipc.launch('https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/releases')}>Download LibreHardwareMonitor</button>
    {:else if sensor}
      <div class="big">{Math.round(toUnit(sensor.value, f))}°{f ? 'F' : 'C'}</div>
      <div class="muted center">{sensor.hardware} · {sensor.name}</div>
      <Spark values={hist} width={300} height={56} />
      <small class="muted">Last ten minutes</small>
      <div class="stats"><div><small>CPU load</small><strong>{Math.round(stats.value?.cpu ?? 0)}%</strong></div><div><small>Memory</small><strong>{stats.value ? Math.round(stats.value.ramUsed / stats.value.ramTotal * 100) : 0}%</strong></div></div>
      <div class="seg"><button class:on={!f} onclick={() => { f = false; act('set-options', { unit: 'C' }) }}>°C</button><button class:on={f} onclick={() => { f = true; act('set-options', { unit: 'F' }) }}>°F</button></div>
      <div class="head">Sensor</div>
      <div class="list">
        {#each groups as g}
          <div class="grp">{g}</div>
          {#each sensors.filter((s) => s.hardware === g) as s (s.id)}
            <button class:sel={s.id === sensor.id} onclick={() => { sel = s.id; act('set-options', { sensor: s.id }) }}><span>{s.name}</span><span>{Math.round(toUnit(s.value, f))}°</span></button>
          {/each}
        {/each}
      </div>
    {:else}<p class="muted">Reading sensors…</p>{/if}
  {:else if id === 'brightness'}
    {#if !brightErr}
      <div class="brow"><Sun size={16} /><input type="range" min="0" max="100" value={bright} oninput={(e) => setBright(+e.currentTarget.value)} /><span class="pct">{bright}%</span></div>
      <p class="muted">Main monitor, over DDC/CI. Other screens are never touched.</p>
      <div class="head">Software</div>
    {:else}
      <p class="muted">Your monitor doesn't take brightness commands (DDC/CI is off in its own menu), so Strata adjusts the picture instead. Your monitor's settings are untouched, and everything goes back to normal when Strata quits.</p>
    {/if}
    <div class="brow"><Sun size={16} /><input type="range" min="40" max="100" value={sb} oninput={(e) => soft(+e.currentTarget.value, sw)} /><span class="pct">{sb}%</span></div>
    <div class="brow"><Moon size={16} /><input type="range" min="0" max="100" value={sw} oninput={(e) => soft(sb, +e.currentTarget.value)} class="warm" /><span class="pct">{sw}%</span></div>
    <div class="presets">
      <button onclick={() => soft(100, 0)}>Day</button><button onclick={() => soft(90, 35)}>Evening</button><button onclick={() => soft(75, 70)}>Night</button>
    </div>
    <label class="opt"><span>Warm automatically at night (20:00–07:00)</span><input type="checkbox" class="sw" checked={auto} onchange={(e) => { auto = e.currentTarget.checked; act('set-options', { sb, sw, auto, soft: true }) }} /></label>
    {#if softErr}<p class="err">{softErr}</p>{/if}
    <p class="muted">Doesn't apply while Windows HDR is on. Brightness can't go below 40% (a Windows limit).</p>
  {:else if id === 'quick-toggles'}
    <div class="qhead"><span>Quick toggles</span><button aria-label="Choose toggles" class:on={editing} onclick={() => (editing = !editing)}><Settings2 size={13} /></button></div>
    {#if editing}
      <div class="pick">
        {#each ALL as x}
          {@const avail = stOf(x.id)?.available ?? true}
          <label class:na={!avail}><input type="checkbox" checked={chosen.includes(x.id)} disabled={!avail} onchange={(e) => { chosen = e.currentTarget.checked ? [...chosen, x.id] : chosen.filter((c) => c !== x.id); act('set-options', { toggles: $state.snapshot(chosen) }) }} />{x.label}{#if !avail}<small> (not on this PC)</small>{/if}</label>
        {/each}
      </div>
    {:else}
      <div class="qgrid">
        {#each shown as x (x.id)}
          {@const s = stOf(x.id)}
          <button class="q" class:on={s?.on} onclick={() => flip(x)}>
            <span class="qi"><x.icon size={17} /></span><span class="ql">{x.label}</span>
            <small>{x.settings || x.id === 'display' ? 'Open' : s?.on ? 'On' : 'Off'}</small>
          </button>
        {/each}
      </div>
    {/if}
    {#if tErr}<p class="err">{tErr}</p>{/if}
  {:else}
    <div class="big">{bat?.present ? `${bat.percent}%` : 'AC power'}</div>
    <p class="muted center">{bat?.present ? (bat.charging ? 'Charging' : bat.plugged ? 'Plugged in' : 'On battery') : 'This PC has no battery.'}</p>
    <button class="link" onclick={() => ipc.launch('ms-settings:batterysaver')}><BatteryCharging size={13} /> Power & battery settings</button>
  {/if}
</div>

<style>
  .sc { width: 330px; padding: 16px; display: flex; flex-direction: column; gap: 10px; max-height: 620px; overflow-y: auto; }
  h4 { margin: 0; font: 600 15px var(--font-display); }
  .big { font: 600 40px var(--font-display); text-align: center; font-variant-numeric: tabular-nums; }
  .center { text-align: center; }
  .muted { color: var(--fg-3); font-size: 12px; margin: 0; line-height: 1.45; }
  ol.muted { padding-left: 18px; }
  .link { align-self: flex-start; display: flex; align-items: center; gap: 6px; font-size: 12px; color: var(--accent); }
  .stats { display: grid; grid-template-columns: 1fr 1fr; gap: 6px; }
  .stats div { padding: 7px 9px; border-radius: 9px; background: var(--surface); display: flex; flex-direction: column; }
  .stats small { font-size: 10.5px; color: var(--fg-3); }
  .stats strong { font: 600 15px var(--font-display); }
  .seg { display: flex; gap: 4px; padding: 3px; border-radius: 9px; background: var(--surface); align-self: flex-start; }
  .seg button { padding: 4px 12px; border-radius: 7px; font-size: 12px; color: var(--fg-2); }
  .seg button.on { background: var(--surface-press); color: var(--fg); }
  .head { font-size: 11px; font-weight: 600; color: var(--fg-3); text-transform: uppercase; letter-spacing: .04em; }
  .list { display: flex; flex-direction: column; gap: 1px; }
  .grp { font-size: 11px; color: var(--fg-3); margin: 6px 0 2px; }
  .list button { display: flex; justify-content: space-between; padding: 5px 8px; border-radius: 7px; font-size: 12.5px; }
  .list button:hover { background: var(--surface-hover); }
  .list button.sel { background: color-mix(in srgb, var(--accent) 22%, transparent); }
  .brow { display: grid; grid-template-columns: 18px 1fr 40px; gap: 10px; align-items: center; }
  .brow input { width: 100%; accent-color: var(--accent); }
  .brow input.warm { accent-color: #ff9f0a; }
  .presets { display: flex; gap: 6px; }
  .presets button { flex: 1; padding: 7px 0; border-radius: 9px; background: var(--surface); font-size: 12.5px; }
  .presets button:hover { background: var(--accent); color: white; }
  .opt { display: flex; justify-content: space-between; align-items: center; gap: 8px; font-size: 12.5px; }
  .pct { text-align: right; font-size: 12.5px; font-variant-numeric: tabular-nums; }
  .qhead { display: flex; justify-content: space-between; align-items: center; font: 600 14px var(--font-display); }
  .qhead button { width: 26px; height: 26px; border-radius: 8px; display: grid; place-items: center; color: var(--fg-2); }
  .qhead button:hover, .qhead button.on { background: var(--surface-hover); color: var(--fg); }
  .qgrid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 6px; }
  .q { display: flex; flex-direction: column; align-items: flex-start; gap: 4px; padding: 9px; border-radius: 12px; background: var(--surface); text-align: left; }
  .q:hover { background: var(--surface-hover); }
  .q.on { background: var(--accent); color: white; }
  .q.on small { color: rgba(255, 255, 255, 0.8); }
  .qi { display: grid; }
  .ql { font-size: 12px; font-weight: 600; line-height: 1.2; }
  .q small { font-size: 10.5px; color: var(--fg-3); }
  .pick { display: flex; flex-direction: column; gap: 6px; font-size: 12.5px; }
  .pick label { display: flex; align-items: center; gap: 8px; }
  .pick label.na { color: var(--fg-3); }
  .pick input { accent-color: var(--accent); }
  .err { color: var(--danger); font-size: 12px; margin: 0; }
</style>
