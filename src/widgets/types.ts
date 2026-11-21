// The widget contract. Adding a widget = one folder with a Tile (and optionally a Card) + one line in
// ./index.ts. Shared plumbing (cards, menus, hover names, data files, icons) comes from the framework.
import type { Component } from 'svelte'
import type { DockItem, Layout } from '../lib/ipc'

export type Category =
  | 'system' | 'productivity' | 'media' | 'apps' | 'komorebi' | 'clocks' | 'weather' | 'wallpaper'
  | 'ai' | 'finance' | 'analytics' | 'social' | 'markets' | 'office'

export type MenuItem =
  | { label: string; action: string; danger?: boolean; checked?: boolean; disabled?: boolean }
  | { separator: true }

export interface DockContext {
  layout: Layout
  /** Open this widget's card (or toggle it closed) anchored to `el`. */
  openCard(item: DockItem, el: HTMLElement, payload?: unknown): void
  /** Open a menu anchored to `el`; the chosen action comes back through `onMenu`. */
  openMenu(item: DockItem, el: HTMLElement, items: MenuItem[]): void
  /** Persist new options for this dock item. */
  setOptions(item: DockItem, options: Record<string, any>): void
}

export interface TileProps {
  item: DockItem
  ctx: DockContext
  /** True while this widget's card is open (tiles can show a pressed state). */
  active: boolean
}

export interface CardProps {
  item: DockItem
  payload: any
  dock: string
  close: () => void
  /** Send an action back to the tile on the dock. */
  act: (action: string, data?: unknown) => void
}

export interface WidgetDef {
  id: string
  name: string
  category: Category
  description: string
  tile: Component<TileProps>
  card?: Component<CardProps>
  /** 'square' tiles are one dock-height wide; 'wide' tiles size to their content. */
  shape: 'square' | 'wide'
  /** If set, a click runs this instead of opening the card. Return true to also open the card. */
  click?: (item: DockItem, ctx: DockContext, el: HTMLElement, e: MouseEvent) => boolean | void
  /** Middle click. */
  middle?: (item: DockItem, ctx: DockContext) => void
  /** Right click: either a custom handler (return true to also show the menu) or extra menu items. */
  rightClick?: (item: DockItem, ctx: DockContext, el: HTMLElement) => boolean | void
  menu?: (item: DockItem) => MenuItem[]
  /** Called with menu actions this widget added. */
  onMenu?: (item: DockItem, action: string, ctx: DockContext) => void
  /** Wheel over the tile. */
  wheel?: (item: DockItem, delta: number, ctx: DockContext) => void
  /** Hover name; defaults to `name`. */
  title?: (item: DockItem) => string
  /** Kept only so existing docks keep working (e.g. merged widgets); not listed in the picker. */
  hidden?: boolean
  /** Not built yet: shows as a dimmed tile with an honest "not built yet" note. */
  planned?: boolean
}
