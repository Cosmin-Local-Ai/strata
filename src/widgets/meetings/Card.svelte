<script lang="ts">
  // Meeting Notes: a list of meetings and an editor with title, time, attendees, agenda, notes, action
  // items (with owners), a meeting timer, and export as Markdown (copy or save). Saved as you type.
  import { ArrowLeft, Check, ClipboardCopy, Download, Pause, Play, Plus, Search, Trash2, X } from '@lucide/svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { clock, dataFile } from '../../lib/stores.svelte'
  import type { CardProps } from '../types'
  import { blank, elapsedOf, mmss, toMarkdown, uid, type Meeting, type MeetingsFile } from './meetings'
  let { }: CardProps = $props()
  const data = dataFile<MeetingsFile>('meetings', { list: [] })
  const now = clock(1000)
  let openId = $state<string | null>(null)
  let q = $state('')
  let toast = $state('')
  let tt: ReturnType<typeof setTimeout> | undefined
  const say = (m: string) => { toast = m; clearTimeout(tt); tt = setTimeout(() => (toast = ''), 2400) }
  $effect(() => { invoke('card_sticky', { on: !!openId }); return () => { invoke('card_sticky', { on: false }) } })

  const sorted = $derived([...data.value.list].sort((a, b) => b.when - a.when)
    .filter((m) => !q || `${m.title} ${m.attendees} ${m.notes} ${m.agenda}`.toLowerCase().includes(q.trim().toLowerCase())))
  const cur = $derived(data.value.list.find((m) => m.id === openId) ?? null)

  // Local draft so typing never waits for disk; written 400 ms after the last change.
  let draft = $state<Meeting | null>(null)
  $effect(() => { if (cur && draft?.id !== cur.id) draft = structuredClone($state.snapshot(cur)) })
  let saveT: ReturnType<typeof setTimeout> | undefined
  function persist(now = false) {
    if (!draft) return
    const d = { ...$state.snapshot(draft), updated: Date.now() }
    clearTimeout(saveT)
    const go = () => data.save({ list: [d, ...data.value.list.filter((m) => m.id !== d.id)] })
    if (now) go(); else saveT = setTimeout(go, 400)
  }
  function create(start: boolean) {
    const m = blank()
    if (start) m.timerStart = Date.now()
    const d = new Date(m.when)
    m.title = `Meeting ${d.toLocaleDateString(undefined, { day: 'numeric', month: 'short' })}`
    data.save({ list: [m, ...data.value.list] })
    draft = m; openId = m.id
  }
  function remove() {
    if (!draft) return
    data.save({ list: data.value.list.filter((m) => m.id !== draft!.id) }); openId = null; draft = null
  }
  function timer() {
    if (!draft) return
    if (draft.timerStart) { draft.elapsed += Date.now() - draft.timerStart; draft.timerStart = 0 } else draft.timerStart = Date.now()
    persist(true)
  }
  let newAction = $state(''), newOwner = $state('')
  function addAction() {
    if (!draft || !newAction.trim()) return
    draft.actions = [...draft.actions, { id: uid('a'), text: newAction.trim(), owner: newOwner.trim(), done: false }]
    newAction = ''; newOwner = ''; persist()
  }
  async function copyMd() { if (draft) { await invoke('clipboard_text', { text: toMarkdown(draft) }); say('Copied as Markdown') } }
  async function saveMd() {
    if (!draft) return
    const name = `${(draft.title || 'Meeting').replace(/[\\/:*?"<>|]/g, '-')} ${new Date(draft.when).toISOString().slice(0, 10)}.md`
    const p = await invoke<string | null>('save_text_as', { title: 'Save meeting notes', defaultName: name, text: toMarkdown(draft) })
    if (p) say('Saved')
  }
  const localInput = (ms: number) => { const d = new Date(ms); d.setMinutes(d.getMinutes() - d.getTimezoneOffset()); return d.toISOString().slice(0, 16) }
  const day = (ms: number) => new Date(ms).toLocaleDateString(undefined, { weekday: 'short', day: 'numeric', month: 'short' })
  const hm = (ms: number) => new Date(ms).toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })
</script>

<div class="mc">
  {#if draft && openId}
    <div class="hd">
      <button class="ic" aria-label="Back" onclick={() => { persist(true); openId = null; draft = null }}><ArrowLeft size={15} /></button>
      <input class="title" bind:value={draft.title} oninput={() => persist()} placeholder="Meeting title" />
      <button class="timer" class:on={!!draft.timerStart} onclick={timer} title={draft.timerStart ? 'Pause the timer' : 'Start the timer'}>
        {#if draft.timerStart}<Pause size={12} />{:else}<Play size={12} />{/if}{mmss(elapsedOf(draft, now.now.getTime()))}
      </button>
    </div>
    <div class="two">
      <label>When<input type="datetime-local" value={localInput(draft.when)} onchange={(e) => { draft!.when = new Date(e.currentTarget.value).getTime() || draft!.when; persist() }} /></label>
      <label>Attendees<input bind:value={draft.attendees} oninput={() => persist()} placeholder="Ana, Mihai, …" /></label>
    </div>
    <label>Agenda<textarea rows="2" bind:value={draft.agenda} oninput={() => persist()} placeholder="One topic per line"></textarea></label>
    <label>Notes<textarea rows="7" bind:value={draft.notes} oninput={() => persist()} placeholder="What was said and decided"></textarea></label>
    <div class="acts">
      <span class="lbl">Action items</span>
      {#each draft.actions as a (a.id)}
        <div class="act" class:done={a.done}>
          <button class="chk" class:on={a.done} aria-label="Done" onclick={() => { a.done = !a.done; persist() }}><Check size={10} /></button>
          <span class="tx">{a.text}</span>{#if a.owner}<span class="own">{a.owner}</span>{/if}
          <button class="x" aria-label="Remove" onclick={() => { draft!.actions = draft!.actions.filter((x) => x !== a); persist() }}><X size={12} /></button>
        </div>
      {/each}
      <form class="add" onsubmit={(e) => { e.preventDefault(); addAction() }}>
        <input bind:value={newAction} placeholder="Add an action item" />
        <input class="o" bind:value={newOwner} placeholder="Who" />
        <button type="submit" aria-label="Add"><Plus size={14} /></button>
      </form>
    </div>
    <div class="btns">
      <button class="danger" onclick={remove}><Trash2 size={13} /> Delete</button>
      <span class="sp"></span>
      <button onclick={copyMd}><ClipboardCopy size={13} /> Copy</button>
      <button onclick={saveMd}><Download size={13} /> Save .md</button>
    </div>
  {:else}
    <div class="top">
      <label class="search"><Search size={14} /><input bind:value={q} placeholder="Search meetings" spellcheck="false" /></label>
      <button class="new" onclick={() => create(true)} title="New meeting with the timer running"><Play size={13} /> Start</button>
      <button class="ic" aria-label="New" title="New meeting (no timer)" onclick={() => create(false)}><Plus size={16} /></button>
    </div>
    <ul>
      {#each sorted as m (m.id)}
        {@const open = m.actions.filter((a) => !a.done).length}
        <li>
          <button onclick={() => (openId = m.id)}>
            <span class="d"><b>{new Date(m.when).getDate()}</b><small>{new Date(m.when).toLocaleDateString(undefined, { month: 'short' })}</small></span>
            <span class="tx"><strong>{m.title || 'Meeting'}</strong><small>{day(m.when)} · {hm(m.when)}{m.attendees ? ` · ${m.attendees}` : ''}</small></span>
            {#if m.timerStart}<span class="live">● {mmss(elapsedOf(m, now.now.getTime()))}</span>{:else if open}<span class="cnt" title="Open action items">{open}</span>{/if}
          </button>
        </li>
      {:else}
        <li class="empty">{q ? 'No matches.' : 'No meetings yet. Press Start when a meeting begins.'}</li>
      {/each}
    </ul>
  {/if}
  {#if toast}<div class="toast">{toast}</div>{/if}
</div>

<style>
  .mc { position: relative; width: 420px; max-height: 640px; overflow-y: auto; padding: 14px; display: flex; flex-direction: column; gap: 9px; }
  .hd, .top { display: flex; align-items: center; gap: 6px; }
  .ic { width: 32px; height: 32px; border-radius: 9px; display: grid; place-items: center; background: var(--surface); flex: none; }
  .title { flex: 1; min-width: 0; font: 600 15px var(--font-display); background: none; border: 0; outline: none; color: var(--fg); padding: 4px 2px; }
  .timer { display: flex; align-items: center; gap: 5px; padding: 5px 10px; border-radius: 999px; background: var(--surface); font: 600 12px var(--font-display); font-variant-numeric: tabular-nums; flex: none; }
  .timer.on { background: color-mix(in srgb, #ff453a 22%, transparent); color: #ff6961; }
  .two { display: flex; gap: 8px; }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 11.5px; color: var(--fg-2); flex: 1; min-width: 0; }
  input, textarea { padding: 7px 9px; border-radius: 8px; border: 1px solid var(--border); background: var(--surface); outline: none; font-size: 12.5px; color: var(--fg); resize: none; font-family: inherit; min-width: 0; }
  input[type='datetime-local'] { color-scheme: dark; }
  .acts { display: flex; flex-direction: column; gap: 3px; }
  .lbl { font-size: 11.5px; color: var(--fg-2); }
  .act { display: flex; align-items: center; gap: 7px; padding: 4px 4px; border-radius: 7px; font-size: 12.5px; }
  .act:hover { background: var(--surface-hover); }
  .act.done .tx { text-decoration: line-through; color: var(--fg-3); }
  .chk { width: 16px; height: 16px; border-radius: 5px; border: 1.5px solid var(--fg-3); display: grid; place-items: center; color: transparent; flex: none; }
  .chk.on { background: var(--accent); border-color: var(--accent); color: white; }
  .act .tx { flex: 1; min-width: 0; }
  .own { font-size: 11px; color: var(--fg-2); padding: 1px 7px; border-radius: 999px; background: var(--surface); }
  .x { width: 22px; height: 22px; border-radius: 6px; display: grid; place-items: center; color: var(--fg-3); opacity: 0; }
  .act:hover .x { opacity: 1; }
  .add { display: flex; gap: 4px; }
  .add input { flex: 1; }
  .add .o { flex: 0 0 80px; }
  .add button { width: 32px; border-radius: 8px; display: grid; place-items: center; background: var(--surface); }
  .btns { display: flex; gap: 6px; align-items: center; }
  .sp { flex: 1; }
  .btns button { padding: 6px 12px; border-radius: 8px; background: var(--surface-press); font-size: 12.5px; display: flex; align-items: center; gap: 5px; }
  .btns .danger:hover { background: var(--danger); color: white; }
  .search { flex: 1; display: flex; flex-direction: row; align-items: center; gap: 7px; padding: 7px 10px; border-radius: 10px; background: var(--surface); color: var(--fg-3); }
  .search input { flex: 1; background: none; border: 0; padding: 0; }
  .new { display: flex; align-items: center; gap: 5px; padding: 0 12px; height: 32px; border-radius: 9px; background: var(--accent); color: white; font-size: 12.5px; }
  ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 2px; }
  li button { width: 100%; display: flex; align-items: center; gap: 10px; padding: 7px 6px; border-radius: 10px; text-align: left; }
  li button:hover { background: var(--surface-hover); }
  li.empty { font-size: 12px; color: var(--fg-3); padding: 6px; }
  .d { width: 36px; height: 36px; border-radius: 9px; background: var(--surface); display: flex; flex-direction: column; align-items: center; justify-content: center; line-height: 1; flex: none; }
  .d b { font: 700 14px var(--font-display); }
  .d small { font-size: 9.5px; color: var(--fg-3); text-transform: uppercase; }
  .tx { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .tx strong { font-size: 13px; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .tx small { font-size: 11px; color: var(--fg-3); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .live { font: 600 11.5px var(--font-display); color: #ff6961; font-variant-numeric: tabular-nums; }
  .cnt { min-width: 18px; height: 18px; border-radius: 9px; background: var(--accent); color: white; font: 700 10.5px/18px var(--font-display); text-align: center; padding: 0 4px; }
  .toast { position: sticky; bottom: 0; align-self: center; padding: 6px 12px; border-radius: 999px; background: rgba(20, 20, 24, 0.92); color: white; font-size: 12px; }
</style>
