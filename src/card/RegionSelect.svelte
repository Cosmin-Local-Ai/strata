<script lang="ts">
  // Region capture: the screen as it was when you clicked (a still image), dimmed; drag a rectangle.
  // Esc or right-click cancels. Coordinates are sent in physical pixels.
  import { invoke } from '@tauri-apps/api/core'
  let { close, text }: { close: () => void; text: boolean } = $props()
  let path = $state('')
  // The temp file name is fixed on the Rust side.
  $effect(() => { invoke<string>('capture_temp_path').then((p) => (path = p)).catch(() => {}) })
  let a = $state<{ x: number; y: number } | null>(null)
  let b = $state<{ x: number; y: number } | null>(null)
  let msg = $state('')
  const r = $derived(a && b ? { x: Math.min(a.x, b.x), y: Math.min(a.y, b.y), w: Math.abs(a.x - b.x), h: Math.abs(a.y - b.y) } : null)
  async function done() {
    if (!r || r.w < 4 || r.h < 4) { a = b = null; return }
    const k = window.devicePixelRatio || 1
    try {
      const res = await invoke<{ path: string; text?: string | null }>('capture_region_finish', { x: Math.round(r.x * k), y: Math.round(r.y * k), w: Math.round(r.w * k), h: Math.round(r.h * k), text })
      msg = text ? (res.text ? 'Text copied' : 'No text found') : 'Copied and saved'
      setTimeout(close, 650)
    } catch (e) { msg = String(e); setTimeout(close, 1500) }
  }
  function cancel() { invoke('capture_region_cancel'); close() }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && cancel()} />

<div class="rs" role="presentation" oncontextmenu={(e) => { e.preventDefault(); cancel() }}
     onpointerdown={(e) => { if (e.button === 0) { a = { x: e.clientX, y: e.clientY }; b = a } }}
     onpointermove={(e) => { if (a && e.buttons & 1) b = { x: e.clientX, y: e.clientY } }}
     onpointerup={done}>
  {#if path}<img src={`http://thumb.localhost/${encodeURIComponent(path)}?raw=1&t=${Date.now()}`} alt="" draggable="false" />{/if}
  <div class="dim" style={r ? `clip-path: polygon(0 0, 100% 0, 100% 100%, 0 100%, 0 ${r.y}px, ${r.x}px ${r.y}px, ${r.x}px ${r.y + r.h}px, ${r.x + r.w}px ${r.y + r.h}px, ${r.x + r.w}px ${r.y}px, 0 ${r.y}px)` : ''}></div>
  {#if r}<div class="sel" style="left:{r.x}px; top:{r.y}px; width:{r.w}px; height:{r.h}px"><span>{Math.round(r.w * devicePixelRatio)} × {Math.round(r.h * devicePixelRatio)}</span></div>{/if}
  <div class="hint">{msg || (text ? 'Drag over the text to copy · Esc to cancel' : 'Drag to capture · Esc to cancel')}</div>
</div>

<style>
  .rs { position: fixed; inset: 0; cursor: crosshair; user-select: none; }
  img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: fill; pointer-events: none; }
  .dim { position: absolute; inset: 0; background: rgba(0, 0, 0, 0.45); pointer-events: none; }
  .sel { position: absolute; border: 1.5px solid var(--accent); box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.4); pointer-events: none; }
  .sel span { position: absolute; left: 0; top: -22px; font: 600 11px var(--font); padding: 2px 6px; border-radius: 5px; background: var(--accent); color: white; white-space: nowrap; }
  .hint { position: absolute; left: 50%; top: 24px; transform: translateX(-50%); padding: 7px 14px; border-radius: 999px; background: rgba(20, 20, 24, 0.85); color: white; font-size: 13px; pointer-events: none; }
</style>
