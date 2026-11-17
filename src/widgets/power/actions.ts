import { Lock, LogOut, Moon, Power, RotateCcw, Snowflake } from '@lucide/svelte'

/** Shut down on the far left:
 *  shutdown, restart, sign out, hibernate, sleep, then Lock.
 *  `confirm`: needs a second click, so a stray click can't end the session. */
export const POWER_ACTIONS = [
  { id: 'shutdown', label: 'Shut down', icon: Power, confirm: true },
  { id: 'restart', label: 'Restart', icon: RotateCcw, confirm: true },
  { id: 'signout', label: 'Sign out', icon: LogOut, confirm: true },
  { id: 'hibernate', label: 'Hibernate', icon: Snowflake, confirm: true },
  { id: 'sleep', label: 'Sleep', icon: Moon, confirm: false },
  { id: 'lock', label: 'Lock', icon: Lock, confirm: false },
] as const

export const formatUptime = (s: number) => `${Math.floor(s / 86400)}d ${Math.floor((s % 86400) / 3600)}h ${Math.floor((s % 3600) / 60)}m`
