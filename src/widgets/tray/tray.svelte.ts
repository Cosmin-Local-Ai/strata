import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

export type TrayIcon = { key: string; tooltip: string; icon: string; hidden: boolean; exe: string }
export const tray = $state<{ value: TrayIcon[] }>({ value: [] })
let started = false
export function startTray() {
  if (started) return
  started = true
  invoke<TrayIcon[]>('tray_icons').then((l) => (tray.value = l))
  listen<TrayIcon[]>('tray:icons', (e) => (tray.value = e.payload))
}
