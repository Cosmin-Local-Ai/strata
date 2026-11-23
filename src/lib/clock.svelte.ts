import { invoke } from '@tauri-apps/api/core'

/** Windows' own time format (Settings > Time & language > Language & region): 12- or 24-hour.
 *  Times follow it rather than the display language, so English Windows with a 24-hour region
 *  shows 22:51, not 10:51 PM. Clock widgets can still override it. */
export const sysClock = $state({ h12: false })
invoke<boolean>('clock_12h').then((v) => (sysClock.h12 = v)).catch(() => {})

/** Hours and minutes in the Windows time format. */
export const hm = (d: Date | number | string) => new Date(d).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: sysClock.h12 })
