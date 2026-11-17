<script lang="ts">
  // Notes: list + editor. Plain text, saved as you type (debounced), never leaves the PC.
  import { Check, Copy, Plus, Trash2 } from '@lucide/svelte'
  import { dataFile } from '../../lib/stores.svelte'
  import type { CardProps } from '../types'
  import { noteText, ts, type Note, type NotesFile } from './notes'

  let { }: CardProps = $props()
  const data = dataFile<NotesFile>('notes', { notes: [] })
  const sorted = $derived([...data.value.notes].sort((a, b) => ts(b) - ts(a)))
  let selected = $state<string | null>(null)
  const current = $derived(sorted.find((n) => n.id === selected) ?? sorted[0])
  let text = $state('')
  let title = $state('')
  let timer: ReturnType<typeof setTimeout> | undefined

  $effect(() => { if (current) { text = noteText(current); title = current.title } })

  function persist(patch: Partial<Note>) {
    if (!current) return
    const id = current.id
    clearTimeout(timer)
    timer = setTimeout(() => {
      const notes = data.value.notes.map((n) => (n.id === id ? { ...n, ...patch, html: undefined, updated: Date.now() } : n))
      data.save({ notes })
    }, 350)
  }
  function add() {
    const n: Note = { id: 'n' + Date.now(), title: 'New note', text: '', updated: Date.now() }
    data.save({ notes: [n, ...data.value.notes] })
    selected = n.id
  }
  let copied = $state(false)
  async function copy() {
    await navigator.clipboard.writeText(text)
    copied = true
    setTimeout(() => (copied = false), 1200)
  }
  function remove(n: Note) {
    data.save({ notes: data.value.notes.filter((x) => x.id !== n.id) })
    if (selected === n.id) selected = null
  }
</script>

<div class="notes">
  <aside>
    <button class="new" onclick={add}><Plus size={14} /> New note</button>
    <ul>
      {#each sorted as n (n.id)}
        <li class:sel={current?.id === n.id}>
          <button onclick={() => (selected = n.id)}>
            <strong>{n.title || 'Untitled'}</strong>
            <span>{noteText(n).slice(0, 60) || 'Empty'}</span>
          </button>
        </li>
      {/each}
    </ul>
  </aside>
  <section>
    {#if current}
      <div class="head">
        <input class="title" bind:value={title} oninput={() => persist({ title })} />
        <button class="icon copy" title="Copy to clipboard" onclick={copy}>{#if copied}<Check size={15} />{:else}<Copy size={15} />{/if}</button>
        <button class="icon del" title="Delete note" onclick={() => remove(current)}><Trash2 size={15} /></button>
      </div>
      <textarea bind:value={text} oninput={() => persist({ text })} spellcheck="true"></textarea>
    {:else}
      <p class="empty">No notes yet.</p>
    {/if}
  </section>
</div>

<style>
  .notes { display: flex; width: 560px; height: 360px; }
  aside { width: 200px; border-right: 1px solid var(--border); display: flex; flex-direction: column; padding: 10px 6px; gap: 6px; }
  .new { display: flex; align-items: center; gap: 6px; padding: 6px 8px; border-radius: 8px; color: var(--accent); }
  .new:hover { background: var(--surface-hover); }
  ul { list-style: none; margin: 0; padding: 0 2px 0 0; overflow-y: auto; flex: 1; min-height: 0; }
  li button { width: 100%; text-align: left; padding: 7px 8px; border-radius: 8px; display: flex; flex-direction: column; gap: 2px; }
  li button:hover { background: var(--surface-hover); }
  li.sel button { background: var(--surface-press); }
  li strong { font-weight: 600; font-size: 12.5px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  li span { font-size: 11.5px; color: var(--fg-3); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  section { flex: 1; display: flex; flex-direction: column; padding: 12px; gap: 8px; }
  .head { display: flex; gap: 8px; align-items: center; }
  .title { flex: 1; background: none; border: 0; outline: none; font: 600 16px var(--font-display); }
  .icon { padding: 6px; border-radius: 8px; color: var(--fg-2); }
  .icon:hover { background: var(--surface-hover); color: var(--fg); }
  .icon.del:hover { color: var(--danger); }
  textarea { flex: 1; resize: none; background: none; border: 0; outline: none; line-height: 1.5; color: var(--fg); }
  .empty { color: var(--fg-2); }
</style>
