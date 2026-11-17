// Shared reactive state for a window (dock or card). Each store subscribes to Rust events once.
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { ipc, type KState, type Settings } from './ipc'

export const settings = $state<{ value: Settings | null }>({ value: null })
export const komorebi = $state<{ value: KState }>({
  value: { online: false, paused: false, focusedMonitor: 0, monitors: [], focusedWindow: null },
})

/** Accent colour and dock style apply to every Strata window (dock, cards, panels, Settings). */
export function applyTheme(s: Settings) {
  const root = document.documentElement
  root.style.setProperty('--accent', s.accent || '#0a84ff')
  root.dataset.style = s.style || 'liquidGlass'
}

let started = false
export function startStores() {
  if (started) return
  started = true
  ipc.settings().then((s) => { settings.value = s; applyTheme(s) })
  listen<Settings>('settings:changed', (e) => { settings.value = e.payload; applyTheme(e.payload) })
  ipc.komorebiState().then((s) => (komorebi.value = s))
  listen<KState>('komorebi:state', (e) => (komorebi.value = e.payload))
}

/** A JSON data file (launchpad, notes, ...) kept in sync across windows. */
export function dataFile<T>(name: string, fallback: T) {
  const box = $state<{ value: T; loaded: boolean }>({ value: fallback, loaded: false })
  const load = () => ipc.dataRead<T>(name).then((v) => { box.value = (v ?? fallback) as T; box.loaded = true })
  load()
  listen<string>('data:changed', (e) => { if (e.payload === name) load() })
  return {
    get value() { return box.value },
    get loaded() { return box.loaded },
    save(v: T) { box.value = v; return ipc.dataWrite(name, v) },
  }
}

/** Ticks every `ms`, aligned to the wall clock, and stops while the document is hidden. */
export function clock(ms = 1000) {
  const box = $state({ now: new Date() })
  let t: ReturnType<typeof setTimeout> | undefined
  const tick = () => {
    box.now = new Date()
    if (document.visibilityState === 'visible') t = setTimeout(tick, ms - (Date.now() % ms) + 5)
  }
  document.addEventListener('visibilitychange', () => { clearTimeout(t); if (document.visibilityState === 'visible') tick() })
  tick()
  return box
}

export type MediaState = {
  active: boolean; app: string; title: string; artist: string; album: string; playing: boolean
  canPrev: boolean; canNext: boolean; canPlayPause: boolean; position: number; duration: number; sampledAt: number; art: string
}
export type AudioState = { available: boolean; volume: number; muted: boolean; device: string; devices: { id: string; name: string; default: boolean }[] }

export const media = $state<{ value: MediaState | null }>({ value: null })
export const audio = $state<{ value: AudioState | null }>({ value: null })

let mediaStarted = false
/** Media + audio live state (event-driven from Rust). Only windows that show them subscribe. */
export function startMediaAudio() {
  if (mediaStarted) return
  mediaStarted = true
  invoke<MediaState>('media_state').then((s) => (media.value = s))
  listen<MediaState>('media:state', (e) => (media.value = e.payload))
  invoke<AudioState>('audio_state').then((s) => (audio.value = s))
  listen<AudioState>('audio:state', (e) => (audio.value = e.payload))
}
