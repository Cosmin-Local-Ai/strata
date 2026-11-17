import { komorebi } from '../../lib/stores.svelte'
import type { KMonitor } from '../../lib/ipc'

/** komorebic change-layout argument ↔ name in komorebi's state. */
export const LAYOUTS: { arg: string; state: string; label: string }[] = [
  { arg: 'bsp', state: 'BSP', label: 'BSP' },
  { arg: 'columns', state: 'Columns', label: 'Columns' },
  { arg: 'rows', state: 'Rows', label: 'Rows' },
  { arg: 'grid', state: 'Grid', label: 'Grid' },
  { arg: 'vertical-stack', state: 'VerticalStack', label: 'Vertical stack' },
  { arg: 'horizontal-stack', state: 'HorizontalStack', label: 'Horizontal stack' },
  { arg: 'ultrawide-vertical-stack', state: 'UltrawideVerticalStack', label: 'Ultrawide stack' },
  { arg: 'right-main-vertical-stack', state: 'RightMainVerticalStack', label: 'Right main stack' },
  { arg: 'scrolling', state: 'Scrolling', label: 'Scrolling' },
]
/** Layout cycle order. */
export const CYCLE = ['bsp', 'columns', 'rows', 'grid', 'vertical-stack', 'horizontal-stack', 'ultrawide-vertical-stack', 'right-main-vertical-stack']

/** The komorebi monitor this dock sits on (matched by HMONITOR), falling back to the focused one. */
export function monitorFor(hmonitor: number): KMonitor | undefined {
  const k = komorebi.value
  return k.monitors.find((m) => m.hmonitor === hmonitor) ?? k.monitors[k.focusedMonitor]
}

export function layoutArg(stateName: string): string | undefined {
  return LAYOUTS.find((l) => l.state === stateName)?.arg
}
