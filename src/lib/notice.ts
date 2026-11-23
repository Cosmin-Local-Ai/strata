import { emit } from '@tauri-apps/api/event'

/** A short message shown on the dock for a few seconds. Used instead of alert(), whose modal dialog
 *  blocks the whole dock until it is dismissed (and can end up where it can't be clicked). */
export function notice(msg: unknown) {
  const text = msg instanceof Error ? msg.message : String(msg)
  console.warn(text)
  emit('dock:notice', text).catch(() => {})
}
