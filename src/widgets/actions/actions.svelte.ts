// Shared state for the action tiles: refreshed only for the tiles on the dock, cheaply and rarely.
import { invoke } from '@tauri-apps/api/core'

type Layout = { id: string; locale: string; current: boolean }
type Bat = { present: boolean; percent: number; charging: boolean; plugged: boolean }

export const act = $state({
  light: false, mic: null as boolean | null, bin: { items: 0, bytes: 0 }, layouts: [] as Layout[], battery: null as Bat | null,
  watch(id: string) {
    const load = () => {
      if (document.visibilityState !== 'visible') return
      if (id === 'theme-toggle') invoke<boolean>('theme_is_light').then((v) => (act.light = v))
      if (id === 'microphone') invoke<boolean | null>('mic_muted').then((v) => (act.mic = v))
      if (id === 'recycle-bin') invoke<{ items: number; bytes: number }>('recycle_bin').then((v) => (act.bin = v))
      if (id === 'language') invoke<Layout[]>('kb_layouts').then((v) => (act.layouts = v))
      if (id === 'battery') invoke<Bat>('battery').then((v) => (act.battery = v))
    }
    load()
    // Slow refresh; language follows the foreground app so it is checked more often.
    const t = setInterval(load, id === 'language' ? 1500 : 10_000)
    return () => clearInterval(t)
  },
})

export async function toggleTheme() { act.light = !act.light; await invoke('theme_set_light', { light: act.light }) }
export async function toggleMic() { const m = !(act.mic ?? false); act.mic = m; await invoke('mic_set_muted', { muted: m }) }
