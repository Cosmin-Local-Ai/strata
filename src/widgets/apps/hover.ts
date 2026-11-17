import { invoke } from '@tauri-apps/api/core'
import { ipc } from '../../lib/ipc'
import { settings } from '../../lib/stores.svelte'

/** Hover previews for an app icon (Settings > Dock > Window previews, on by default). */
export function previewHover(uidOf: () => string, payloadOf: () => unknown) {
  let timer: ReturnType<typeof setTimeout> | undefined
  return {
    enter(el: HTMLElement) {
      if (settings.value?.windowPreviews === false) return
      clearTimeout(timer)
      timer = setTimeout(() => {
        const r = el.getBoundingClientRect()
        invoke('preview_hover', { on: false })
        ipc.cardOpen(uidOf(), 'apps', { x: r.left, y: r.top, w: r.width, h: r.height }, 'preview', payloadOf())
      }, 400)
    },
    leave() {
      clearTimeout(timer)
      // Give the pointer time to move onto the preview (which then keeps itself open).
      setTimeout(() => invoke('card_close_if', { uid: uidOf() }), 140)
    },
    cancel() {
      clearTimeout(timer)
      invoke('card_close_if', { uid: uidOf() })
    },
  }
}
