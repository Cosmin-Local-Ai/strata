<script lang="ts">
  import { Check, Flag, Plus, RotateCcw, Trash2 } from '@lucide/svelte'
  import { dataFile } from '../../lib/stores.svelte'
  import type { CardProps } from '../types'
  import { fmt, persisted, POMO, type PomoState, type SwState, type TimerState } from './timers.svelte'
  let { item }: CardProps = $props()
  const id = item.widget
  let now = $state(Date.now())
  $effect(() => { const t = setInterval(() => (now = Date.now()), 250); return () => clearInterval(t) })

  // Todos
  type Todo = { id: string; text: string; done: boolean }
  const todos = dataFile<{ todos: Todo[] }>('todos', { todos: [] })
  let text = $state('')
  function add() {
    const t = text.trim()
    if (!t) return
    todos.save({ todos: [...todos.value.todos, { id: 't' + Date.now(), text: t, done: false }] })
    text = ''
  }
  const setTodos = (f: (l: Todo[]) => Todo[]) => todos.save({ todos: f(todos.value.todos) })

  // Timer
  const timer = persisted<TimerState>('timer:' + item.uid, { duration: 5 * 60_000, endsAt: null, remaining: 5 * 60_000 })
  const saved = persisted<number[]>('timer-presets', [60_000, 5 * 60_000, 10 * 60_000, 25 * 60_000, 60 * 60_000])
  let hh = $state(0), mm = $state(5), ss = $state(0)
  const startTimer = (ms: number) => (timer.value = { duration: ms, endsAt: Date.now() + ms, remaining: ms })
  const left = $derived(timer.value.endsAt ? timer.value.endsAt - now : timer.value.remaining)

  // Pomodoro
  const pomo = persisted<PomoState>('pomo:' + item.uid, { phase: 'work', round: 0, endsAt: null, remaining: POMO.work })
  const pLeft = $derived(pomo.value.endsAt ? pomo.value.endsAt - now : pomo.value.remaining)

  // Stopwatch
  const sw = persisted<SwState>('sw:' + item.uid, { startedAt: null, before: 0, laps: [] })
  const el = $derived(sw.value.before + (sw.value.startedAt ? now - sw.value.startedAt : 0))
</script>

<div class="pc">
  {#if id === 'todos'}
    <form class="add" onsubmit={(e) => { e.preventDefault(); add() }}>
      <input bind:value={text} placeholder="Add a to-do" />
      <button type="submit" aria-label="Add"><Plus size={15} /></button>
    </form>
    <ul class="todos">
      {#each todos.value.todos as t (t.id)}
        <li class:done={t.done}>
          <button class="chk" onclick={() => setTodos((l) => l.map((x) => (x.id === t.id ? { ...x, done: !x.done } : x)))} aria-label="Toggle">{#if t.done}<Check size={12} />{/if}</button>
          <span>{t.text}</span>
          <button class="del" onclick={() => setTodos((l) => l.filter((x) => x.id !== t.id))} aria-label="Delete"><Trash2 size={13} /></button>
        </li>
      {:else}<li class="muted">Nothing to do.</li>{/each}
    </ul>
    {#if todos.value.todos.some((t) => t.done)}<button class="link" onclick={() => setTodos((l) => l.filter((x) => !x.done))}>Clear completed</button>{/if}
  {:else if id === 'timer'}
    <div class="big">{fmt(left, left >= 3_600_000)}</div>
    <div class="presets">{#each saved.value as p}<button onclick={() => startTimer(p)}>{p >= 3_600_000 ? `${p / 3_600_000} h` : `${p / 60_000} min`}</button>{/each}</div>
    <div class="custom">
      <input type="number" min="0" max="23" bind:value={hh} /> h <input type="number" min="0" max="59" bind:value={mm} /> m <input type="number" min="0" max="59" bind:value={ss} /> s
      <button class="go" onclick={() => startTimer((hh * 3600 + mm * 60 + ss) * 1000)}>Start</button>
      <button class="link" onclick={() => { const ms = (hh * 3600 + mm * 60 + ss) * 1000; if (ms > 0 && !saved.value.includes(ms)) saved.value = [...saved.value, ms].sort((a, b) => a - b) }}>Save</button>
    </div>
    <button class="link" onclick={() => (timer.value = { ...timer.value, endsAt: null, remaining: timer.value.duration })}><RotateCcw size={12} /> Reset</button>
  {:else if id === 'pomodoro'}
    <div class="phase {pomo.value.phase}">{pomo.value.phase === 'work' ? 'Focus' : pomo.value.phase === 'long' ? 'Long break' : 'Break'} · round {pomo.value.round + 1}</div>
    <div class="big">{fmt(pLeft)}</div>
    <p class="muted">25 min focus, 5 min break, a 15 min break every 4 rounds. A chime plays at each change.</p>
    <button class="link" onclick={() => (pomo.value = { phase: 'work', round: 0, endsAt: null, remaining: POMO.work })}><RotateCcw size={12} /> Reset</button>
  {:else if id === 'stopwatch'}
    <div class="big">{fmt(el, el >= 3_600_000)}<small>.{String(Math.floor((el % 1000) / 100))}</small></div>
    <div class="row">
      <button class="go" onclick={() => (sw.value = sw.value.startedAt ? { ...sw.value, startedAt: null, before: sw.value.before + Date.now() - sw.value.startedAt } : { ...sw.value, startedAt: Date.now() })}>{sw.value.startedAt ? 'Pause' : 'Start'}</button>
      <button disabled={!sw.value.startedAt} onclick={() => (sw.value = { ...sw.value, laps: [el, ...sw.value.laps] })}><Flag size={13} /> Lap</button>
      <button onclick={() => (sw.value = { startedAt: null, before: 0, laps: [] })}><RotateCcw size={13} /> Reset</button>
    </div>
    <ol class="laps">{#each sw.value.laps as l, i}<li><span>Lap {sw.value.laps.length - i}</span><span>{fmt(l, l >= 3_600_000)}</span></li>{/each}</ol>
  {/if}
</div>

<style>
  .pc { width: 300px; padding: 14px; display: flex; flex-direction: column; gap: 10px; }
  .add { display: flex; gap: 6px; }
  .add input { flex: 1; padding: 7px 10px; border-radius: 9px; border: 1px solid var(--border); background: var(--surface); outline: none; }
  .add button, .row button, .presets button { padding: 6px 10px; border-radius: 9px; background: var(--surface-press); display: flex; align-items: center; gap: 5px; font-size: 12px; }
  .row button:disabled { opacity: 0.4; }
  .todos { list-style: none; margin: 0; padding: 0; max-height: 300px; overflow-y: auto; }
  .todos li { display: flex; align-items: center; gap: 8px; padding: 6px 4px; font-size: 13px; }
  .todos li span { flex: 1; min-width: 0; overflow-wrap: anywhere; }
  .todos li.done span { text-decoration: line-through; color: var(--fg-3); }
  .chk { width: 20px; height: 20px; border-radius: 10px; border: 1.5px solid var(--fg-3); display: grid; place-items: center; flex: none; }
  .done .chk { background: var(--good); border-color: var(--good); color: white; }
  .del { color: var(--fg-3); padding: 3px; border-radius: 6px; }
  .del:hover { color: var(--danger); }
  .big { font: 600 40px var(--font-display); font-variant-numeric: tabular-nums; text-align: center; }
  .big small { font-size: 20px; color: var(--fg-2); }
  .presets { display: flex; flex-wrap: wrap; gap: 6px; }
  .custom { display: flex; align-items: center; gap: 4px; font-size: 12px; color: var(--fg-2); flex-wrap: wrap; }
  .custom input { width: 44px; padding: 4px 6px; border-radius: 7px; border: 1px solid var(--border); background: var(--surface); color: var(--fg); }
  .go { padding: 6px 14px !important; border-radius: 9px; background: var(--accent) !important; color: white; font-weight: 600; }
  .row { display: flex; gap: 6px; justify-content: center; }
  .phase { text-align: center; font-weight: 600; font-size: 12px; text-transform: uppercase; letter-spacing: .05em; }
  .phase.work { color: #ff453a; } .phase.break, .phase.long { color: var(--good); }
  .laps { list-style: none; margin: 0; padding: 0; max-height: 180px; overflow-y: auto; font-variant-numeric: tabular-nums; }
  .laps li { display: flex; justify-content: space-between; padding: 4px 6px; font-size: 12.5px; border-bottom: 1px solid var(--border); }
  .muted { color: var(--fg-2); font-size: 12px; margin: 0; }
  .link { display: flex; align-items: center; gap: 5px; color: var(--fg-3); font-size: 12px; align-self: flex-start; }
  .link:hover { color: var(--fg); }
</style>
