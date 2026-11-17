// Dock pointer position + magnification settings, shared with tiles that magnify their own icons.
export const dockPointer = $state<{ x: number | null; magnify: boolean; maxScale: number }>({ x: null, magnify: false, maxScale: 1.35 })

/** Scale for an element centred at window x = `centre` (cosine falloff over 130 px). */
export function magnifyAt(centre: number): number {
  if (!dockPointer.magnify || dockPointer.x === null) return 1
  const t = Math.max(0, Math.cos(Math.min(Math.abs(dockPointer.x - centre) / 130, 1) * Math.PI / 2))
  return 1 + (dockPointer.maxScale - 1) * t
}
