// Svelte action for menus inside cards: positions an absolutely placed element at the pointer
// (client coordinates) and keeps it inside its positioned parent: it opens upward when there is no
// room below and shifts left near the right edge. Cards are their own small windows, so anything
// outside the parent would be cut off.
export function fitAt(node: HTMLElement, at: { x: number; y: number }) {
  function place(p: { x: number; y: number }) {
    const parent = (node.offsetParent as HTMLElement | null) ?? document.body
    const pr = parent.getBoundingClientRect()
    const w = node.offsetWidth, h = node.offsetHeight, m = 6
    let left = p.x - pr.left
    let top = p.y - pr.top
    if (top + h > pr.height - m) top = Math.max(m, top - h)
    if (left + w > pr.width - m) left = Math.max(m, pr.width - w - m)
    node.style.left = `${left}px`
    node.style.top = `${top}px`
  }
  place(at)
  return { update: place }
}
