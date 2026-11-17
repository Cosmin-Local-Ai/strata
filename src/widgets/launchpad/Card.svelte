<script lang="ts">
  // Launchpad: your apps in named sections, with search and right-click
  // actions: Open, Run as administrator, Open file location, Edit, Move to section, Remove from
  // section, Delete; Add new app, New section; sort A–Z / Z–A / Recent / Oldest.
  import { Search } from '@lucide/svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { iconUrl, ipc } from '../../lib/ipc'
  import { dataFile } from '../../lib/stores.svelte'
  import { fitAt } from '../../lib/fit'
  import type { CardProps } from '../types'

  type App = { id: string; title: string; path: string; icon?: string | null; group?: string | null }
  type Data = { apps: App[]; groups?: string[]; sort?: 'custom' | 'az' | 'za' | 'recent' | 'oldest' }
  let { close, act }: CardProps = $props()
  const data = dataFile<Data>('launchpad', { apps: [] })
  let q = $state('')
  let input: HTMLInputElement
  $effect(() => { input?.focus() })

  const groups = $derived([...new Set([...(data.value.groups ?? []), ...data.value.apps.map((a) => a.group).filter((g): g is string => !!g)])])
  const order = (l: App[]) => {
    const s = data.value.sort ?? 'custom'
    const idx = (a: App) => data.value.apps.indexOf(a)
    if (s === 'az') return [...l].sort((a, b) => a.title.localeCompare(b.title))
    if (s === 'za') return [...l].sort((a, b) => b.title.localeCompare(a.title))
    if (s === 'recent') return [...l].sort((a, b) => idx(b) - idx(a))
    return l
  }
  const match = (a: App) => !q || a.title.toLowerCase().includes(q.trim().toLowerCase())
  const sections = $derived([
    { name: '', apps: order(data.value.apps.filter((a) => !a.group && match(a))) },
    ...groups.map((g) => ({ name: g, apps: order(data.value.apps.filter((a) => a.group === g && match(a))) })),
  ].filter((s) => s.apps.length || (s.name && !q)))

  const save = (patch: Partial<Data>) => data.save({ ...data.value, ...patch })
  const icon = (a: App) => (a.icon && /\.(png|jpe?g|ico|bmp|webp)$/i.test(a.icon) ? `http://thumb.localhost/${encodeURIComponent(a.icon)}?w=96` : iconUrl(a.path, 96))
  async function open(a: App, admin = false) {
    try { admin ? await invoke('launch_admin', { target: a.path }) : await ipc.launch(a.path) } catch (e) { alert(String(e)) }
    close()
  }
  function key(e: KeyboardEvent) {
    if (e.key === 'Enter') { const first = sections.flatMap((s) => s.apps)[0]; if (first) open(first) }
  }

  // ---- context menu (inside the card) ----
  type Item = { label: string; run?: () => void; sub?: Item[]; danger?: boolean } | 'sep'
  let menu = $state<{ x: number; y: number; items: Item[] } | null>(null)
  let sub = $state<Item[] | null>(null)
  function showMenu(e: MouseEvent, items: Item[]) { e.preventDefault(); e.stopPropagation(); sub = null; menu = { x: e.clientX, y: e.clientY, items } }
  const common = (): Item[] => [
    { label: 'Add new app…', run: () => (edit = { id: '', title: '', path: '', icon: '', group: null }) },
    { label: 'New section…', run: () => (newGroup = '') },
    { label: 'Sort', sub: (['custom', 'az', 'za', 'recent', 'oldest'] as const).map((s) => ({ label: { custom: 'Custom order', az: 'A–Z', za: 'Z–A', recent: 'Recent first', oldest: 'Oldest first' }[s] + (s === (data.value.sort ?? 'custom') ? '  ✓' : ''), run: () => save({ sort: s }) })) },
  ]
  function appMenu(e: MouseEvent, a: App) {
    showMenu(e, [
      { label: 'Open', run: () => open(a) },
      { label: 'Run as administrator', run: () => open(a, true) },
      { label: 'Open file location', run: () => invoke('reveal', { target: a.path }) },
      { label: 'Pin to dock', run: () => act('add-item', { widget: 'app', options: { path: a.path, title: a.title, ...(a.icon ? { icon: a.icon } : {}) } }) },
      'sep',
      { label: 'Edit…', run: () => (edit = { ...a }) },
      ...groups.filter((g) => g !== a.group).map((g) => ({ label: `Move to ${g}`, run: () => setApp(a, { group: g }) })),
      { label: 'Move to a new section…', run: () => { newGroup = ''; moveAfterCreate = a } },
      ...(a.group ? [{ label: `Remove from ${a.group}`, run: () => setApp(a, { group: null }) }] : []),
      { label: 'Delete from Launchpad', danger: true, run: () => save({ apps: data.value.apps.filter((x) => x.id !== a.id) }) },
      'sep',
      ...common(),
    ])
  }
  function groupMenu(e: MouseEvent, g: string) {
    showMenu(e, [
      { label: 'Rename section…', run: () => { renaming = g; newGroup = g } },
      { label: 'Delete section (keeps its apps)', danger: true, run: () => save({ groups: (data.value.groups ?? []).filter((x) => x !== g), apps: data.value.apps.map((x) => (x.group === g ? { ...x, group: null } : x)) }) },
      'sep', ...common(),
    ])
  }
  // Re-fit the menu once a submenu has made it taller.
  function refit(n: HTMLElement) { const m = n.closest('.menu') as HTMLElement | null; if (m && menu) fitAt(m, { x: menu.x, y: menu.y }) }
  function setApp(a: App, patch: Partial<App>) { save({ apps: data.value.apps.map((x) => (x.id === a.id ? { ...x, ...patch } : x)) }) }

  // ---- edit / add dialog, new section ----
  let edit = $state<App | null>(null)
  let newGroup = $state<string | null>(null)
  let renaming = $state<string | null>(null)
  let moveAfterCreate: App | null = null
  function saveEdit() {
    if (!edit || !edit.path.trim()) return
    const a = { ...edit, title: edit.title.trim() || edit.path.split('\\').pop()!.replace(/\.(lnk|exe)$/i, ''), path: edit.path.trim(), icon: edit.icon?.trim() || null }
    save({ apps: a.id ? data.value.apps.map((x) => (x.id === a.id ? a : x)) : [...data.value.apps, { ...a, id: String(Date.now()) }] })
    edit = null
  }
  function saveGroup() {
    const n = (newGroup ?? '').trim()
    if (!n) { newGroup = null; return }
    if (renaming) {
      save({ groups: (data.value.groups ?? []).map((g) => (g === renaming ? n : g)), apps: data.value.apps.map((x) => (x.group === renaming ? { ...x, group: n } : x)) })
    } else {
      const m = moveAfterCreate
      save({ groups: [...new Set([...(data.value.groups ?? []), n])], apps: m ? data.value.apps.map((x) => (x.id === m.id ? { ...x, group: n } : x)) : data.value.apps })
    }
    newGroup = null; renaming = null; moveAfterCreate = null
  }
</script>

<svelte:window onclick={() => { menu = null; sub = null }} onkeydown={(e) => e.key === 'Escape' && (menu = null)} />

<div class="lp" role="presentation" oncontextmenu={(e) => showMenu(e, common())}>
  <label class="search">
    <Search size={15} />
    <input bind:this={input} bind:value={q} onkeydown={key} placeholder="Search apps" spellcheck="false" />
  </label>
  <div class="scroll">
    {#each sections as s (s.name)}
      {#if s.name}<h3 role="presentation" oncontextmenu={(e) => groupMenu(e, s.name)}>{s.name}</h3>{/if}
      <div class="grid">
        {#each s.apps as a (a.id)}
          <button class="app" onclick={() => open(a)} oncontextmenu={(e) => appMenu(e, a)} title={a.path}>
            <img src={icon(a)} alt="" draggable="false" />
            <span>{a.title}</span>
          </button>
        {/each}
      </div>
    {:else}
      <p class="empty">{q ? 'No matches.' : 'No apps yet. Right-click to add one.'}</p>
    {/each}
  </div>

  {#if menu}
    <ul class="menu" use:fitAt={{ x: menu.x, y: menu.y }} role="menu">
      {#each menu.items as it}
        {#if it === 'sep'}<li class="sep"></li>
        {:else}<li><button class:danger={it.danger} onclick={(e) => { e.stopPropagation(); if (it.sub) { sub = it.sub } else { it.run?.(); menu = null } }}>{it.label}{#if it.sub}<span class="arrow">›</span>{/if}</button></li>{/if}
      {/each}
      {#if sub}
        <li class="sep" use:refit></li>
        {#each sub as it}{#if it !== 'sep'}<li><button class="subitem" onclick={(e) => { e.stopPropagation(); it.run?.(); menu = null; sub = null }}>{it.label}</button></li>{/if}{/each}
      {/if}
    </ul>
  {/if}

  {#if edit}
    <div class="dialog" role="dialog" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={() => {}}>
      <h4>{edit.id ? 'Edit app' : 'Add new app'}</h4>
      <label>Name<input bind:value={edit.title} placeholder="Shown under the icon" /></label>
      <label>Path<input bind:value={edit.path} placeholder="C:\…\App.exe, a shortcut (.lnk), a folder or a URL" /></label>
      <label>Icon (optional)<input bind:value={edit.icon} placeholder="Image file; empty = the app's own icon" /></label>
      <label>Section<select bind:value={edit.group}><option value={null}>None</option>{#each groups as g}<option value={g}>{g}</option>{/each}</select></label>
      <div class="btns"><button onclick={() => (edit = null)}>Cancel</button><button class="primary" onclick={saveEdit}>Save</button></div>
    </div>
  {/if}
  {#if newGroup !== null}
    <div class="dialog small" role="dialog" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={() => {}}>
      <h4>{renaming ? 'Rename section' : 'New section'}</h4>
      <input bind:value={newGroup} placeholder="e.g. Gaming, Utilities" onkeydown={(e) => e.key === 'Enter' && saveGroup()} />
      <div class="btns"><button onclick={() => { newGroup = null; renaming = null }}>Cancel</button><button class="primary" onclick={saveGroup}>Save</button></div>
    </div>
  {/if}
</div>

<style>
  .lp { position: relative; width: 640px; padding: 14px; display: flex; flex-direction: column; gap: 10px; }
  .search { display: flex; align-items: center; gap: 8px; padding: 7px 10px; border-radius: 10px; background: var(--surface); border: 1px solid var(--border); color: var(--fg-2); }
  .search input { flex: 1; background: none; border: 0; outline: none; color: var(--fg); }
  .scroll { max-height: 470px; overflow-y: auto; }
  h3 { margin: 12px 4px 4px; font: 600 11px var(--font); color: var(--fg-3); text-transform: uppercase; letter-spacing: .06em; border-bottom: 1px solid var(--border); padding-bottom: 5px; }
  .grid { display: grid; grid-template-columns: repeat(6, 1fr); gap: 4px; }
  .app { display: flex; flex-direction: column; align-items: center; gap: 6px; padding: 10px 4px; border-radius: 12px; transition: background var(--t-fast) var(--ease-out), transform var(--t-fast) var(--ease-out); }
  .app:hover { background: var(--surface-hover); }
  .app:active { transform: scale(0.95); }
  .app img { width: 44px; height: 44px; object-fit: contain; border-radius: 10px; }
  .app span { font-size: 11.5px; color: var(--fg-2); max-width: 92px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .empty { color: var(--fg-2); margin: 8px 4px; }
  .menu { position: absolute; z-index: 10; list-style: none; margin: 0; padding: 5px; min-width: 200px; border-radius: 10px;
          background: rgba(32, 32, 38, 0.98); border: 1px solid var(--border); box-shadow: 0 10px 30px rgba(0, 0, 0, 0.45); animation: pop var(--t-fast) var(--ease-out); }
  .menu button { width: 100%; text-align: left; padding: 6px 10px; border-radius: 6px; font-size: 12.5px; display: flex; justify-content: space-between; }
  .menu button:hover { background: var(--accent); color: white; }
  .menu button.danger:hover { background: var(--danger); }
  .menu .subitem { padding-left: 22px; }
  .arrow { color: var(--fg-3); }
  .sep { height: 1px; background: var(--border); margin: 4px 6px; }
  .dialog { position: absolute; z-index: 20; left: 50%; top: 50%; transform: translate(-50%, -50%); width: 380px; padding: 16px; border-radius: 14px;
            background: rgba(30, 30, 36, 0.99); border: 1px solid var(--border); box-shadow: 0 20px 60px rgba(0, 0, 0, 0.55); display: flex; flex-direction: column; gap: 8px; }
  .dialog.small { width: 300px; }
  .dialog h4 { margin: 0 0 4px; font: 600 14px var(--font-display); }
  .dialog label { display: flex; flex-direction: column; gap: 4px; font-size: 11.5px; color: var(--fg-2); }
  .dialog input, .dialog select { padding: 7px 9px; border-radius: 8px; border: 1px solid var(--border); background: var(--surface); color: var(--fg); outline: none; font-size: 12.5px; }
  .dialog select option { background: #222; }
  .btns { display: flex; justify-content: flex-end; gap: 6px; margin-top: 6px; }
  .btns button { padding: 6px 14px; border-radius: 8px; background: var(--surface-press); }
  .btns .primary { background: var(--accent); color: white; }
  @keyframes pop { from { opacity: 0; transform: scale(0.97); } }
</style>
