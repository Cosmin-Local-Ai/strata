// Typed wrappers around the Rust commands (src-tauri/src/commands.rs).
import { invoke } from '@tauri-apps/api/core'

export type Edge = 'top' | 'bottom'
export type DockItem = { uid: string; widget: string; options: Record<string, any> }
export type Settings = {
  items: DockItem[]; edge: Edge; dockHeight: number; style: string; accent: string
  magnification: boolean; magnificationScale: number; hoverNames: boolean; lockDock: boolean
  [k: string]: any
}
export type Layout = {
  displayId: string; displayName: string; hmonitor: number; edge: Edge; scale: number; bodyHeight: number
  edgeDistance: number; headroom: number; padding: number; spacing: number; fullWidth: boolean
  maxWidth: number; windowWidth: number; windowHeight: number
}
export type Anchor = { x: number; y: number; w: number; h: number }

export type KWindow = { hwnd: number; exe: string; title: string }
export type KWorkspace = {
  index: number; name: string; layout: string; monocle: boolean; maximized: boolean
  windows: KWindow[]; floating: number; stack: KWindow[]; stackFocused: number; flip: string | null
}
export type KMonitor = { index: number; hmonitor: number; name: string; focusedWorkspace: number; workspaces: KWorkspace[] }
export type KState = { online: boolean; paused: boolean; focusedMonitor: number; monitors: KMonitor[]; focusedWindow: KWindow | null }

export const ipc = {
  settings: () => invoke<Settings>('settings_get'),
  setItems: (items: DockItem[]) => invoke<Settings>('settings_set_items', { items }),
  layout: () => invoke<Layout | null>('dock_layout'),
  cardOpen: (uid: string, widget: string, anchor: Anchor, kind = 'card', payload: unknown = null) =>
    invoke('card_open', { uid, widget, anchor, kind, payload }),
  cardReady: (width: number, height: number) => invoke('card_ready', { width, height }),
  cardClose: () => invoke('card_close'),
  cardAction: (dock: string, uid: string, action: string, data: unknown = null) => invoke('card_action', { dock, uid, action, data }),
  launch: (target: string, args?: string) => invoke('launch', { target, args }),
  power: (action: string) => invoke('power', { action }),
  startMenu: () => invoke('start_menu'),
  winx: () => invoke('winx_menu'),
  uptime: () => invoke<number>('uptime'),
  komorebi: (...args: (string | number)[]) => invoke('komorebi_cmd', { args: args.map(String) }),
  komorebiControl: (action: 'start' | 'stop' | 'restart') => invoke('komorebi_control', { action }),
  komorebiState: () => invoke<KState>('komorebi_state'),
  overlayOpen: (kind: string) => invoke('overlay_open', { kind }),
  overlayClose: () => invoke('overlay_close'),
  dataRead: <T>(name: string) => invoke<T | null>('data_read', { name }),
  dataWrite: (name: string, value: unknown) => invoke('data_write', { name, value }),
}

/** PNG icon for an exe, shortcut, file, folder or shell:AppsFolder\AUMID, extracted by the Rust core. */
export function iconUrl(path: string, size = 64): string {
  return `http://icon.localhost/${encodeURIComponent(path)}?s=${size}`
}
