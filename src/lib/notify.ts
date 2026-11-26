import { invoke } from '@tauri-apps/api/core'

export type Toast = { id: string; app: string; target?: string; title: string; body?: string; source?: string }

/** A Strata notification (shown above the dock). `id` also keeps the same one from showing twice. */
export function notify(t: Toast) {
  invoke('toast_push', { toast: { source: 'strata', target: '', body: '', ...t } }).catch(() => {})
}
