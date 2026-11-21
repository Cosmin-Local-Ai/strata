// Installed Microsoft Office apps (asked from Rust once per window).
import { invoke } from '@tauri-apps/api/core'

export type SuiteApp = { id: string; name: string; target: string }
let apps = $state<SuiteApp[]>([])
let asked = false
export function officeApps() {
  if (!asked) { asked = true; invoke<SuiteApp[]>('office_apps').then((v) => (apps = v)).catch(() => {}) }
  return { get value() { return apps } }
}
