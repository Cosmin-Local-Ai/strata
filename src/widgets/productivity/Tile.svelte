<script lang="ts">
  // Todos (next task + progress ring, tick it off from the dock), Pomodoro (start/skip from the bar),
  // Timer (one-click start), Stopwatch (start/pause/lap from the bar).
  import { Check, Pause, Play, SkipForward, Timer as TimerIcon } from '@lucide/svelte'
  import { dataFile } from '../../lib/stores.svelte'
  import type { TileProps } from '../types'
  import Ring from '../system/Ring.svelte'
  import { chime, fmt, persisted, pomoNext, POMO, type PomoState, type SwState, type TimerState } from './timers.svelte'
  let { item }: TileProps = $props()
  const id = $derived(item.widget)

  let now = $state(Date.now())

  type Todo = { id: string; text: string; done: boolean }
  const todos = dataFile<{ todos: Todo[] }>('todos', { todos: [] })
  const next = $derived(todos.value.todos.find((t) => !t.done))
  const doneRatio = $derived(todos.value.todos.length ? todos.value.todos.filter((t) => t.done).length / todos.value.todos.length : 0)
  function tick(e: MouseEvent) {
    e.stopPropagation()
    if (next) todos.save({ todos: todos.value.todos.map((t) => (t.id === next.id ? { ...t, done: true } : t)) })
  }

  const timer = persisted<TimerState>('timer:' + item.uid, { duration: (item.options.minutes ?? 5) * 60_000, endsAt: null, remaining: (item.options.minutes ?? 5) * 60_000 })
  const timerLeft = $derived(timer.value.endsAt ? timer.value.endsAt - now : timer.value.remaining)
  function timerToggle(e: MouseEvent) {
    e.stopPropagation()
    const t = timer.value
    timer.value = t.endsAt ? { ...t, endsAt: null, remaining: Math.max(0, t.endsAt - Date.now()) } : { ...t, endsAt: Date.now() + (t.remaining > 0 ? t.remaining : t.duration) }
    now = Date.now()
  }

  const pomo = persisted<PomoState>('pomo:' + item.uid, { phase: 'work', round: 0, endsAt: null, remaining: POMO.work })
  const pomoLeft = $derived(pomo.value.endsAt ? pomo.value.endsAt - now : pomo.value.remaining)
  function pomoToggle(e: MouseEvent) {
    e.stopPropagation()
    const p = pomo.value
    pomo.value = p.endsAt ? { ...p, endsAt: null, remaining: Math.max(0, p.endsAt - Date.now()) } : { ...p, endsAt: Date.now() + p.remaining }
    now = Date.now()
  }
  function pomoSkip(e: MouseEvent) { e.stopPropagation(); pomo.value = pomoNext(pomo.value); now = Date.now() }

  const sw = persisted<SwState>('sw:' + item.uid, { startedAt: null, before: 0, laps: [] })
  const swElapsed = $derived(sw.value.before + (sw.value.startedAt ? now - sw.value.startedAt : 0))
  function swToggle(e: MouseEvent) {
    e.stopPropagation()
    const s = sw.value
    sw.value = s.startedAt ? { ...s, startedAt: null, before: s.before + Date.now() - s.startedAt } : { ...s, startedAt: Date.now() }
    now = Date.now()
  }

  // Shared 1 s ticker, only while something runs and the dock is visible.
  const running = $derived(
    (id === 'timer' && !!timer.value.endsAt) || (id === 'pomodoro' && !!pomo.value.endsAt) || (id === 'stopwatch' && !!sw.value.startedAt)
  )
  $effect(() => { if (!running) return; const t = setInterval(() => { if (document.visibilityState === 'visible') now = Date.now() }, 500); return () => clearInterval(t) })

  // Completion: chime and stop (timer) / move to the next phase (pomodoro).
  $effect(() => {
    if (id === 'timer' && timer.value.endsAt && timerLeft <= 0) { chime(); timer.value = { ...timer.value, endsAt: null, remaining: 0 } }
    if (id === 'pomodoro' && pomo.value.endsAt && pomoLeft <= 0) { chime(); pomo.value = pomoNext(pomo.value) }
  })
</script>

{#if id === 'todos'}
  <span class="todo">
    <Ring value={doneRatio} size={24} stroke={3} />
    {#if next}
      <span class="tt">{next.text}</span>
      <button class="tick" title="Done" onclick={tick}><Check size={13} /></button>
    {:else}<span class="tt muted">{todos.value.todos.length ? 'All done' : 'No to-dos'}</span>{/if}
  </span>
{:else if id === 'pomodoro'}
  <span class="tm">
    <Ring value={1 - pomoLeft / POMO[pomo.value.phase]} size={24} stroke={3} color={pomo.value.phase === 'work' ? '#ff453a' : '#30d158'} />
    <span class="t">{fmt(pomoLeft)}</span>
    <button class="b" onclick={pomoToggle} title={pomo.value.endsAt ? 'Pause' : 'Start'}>{#if pomo.value.endsAt}<Pause size={13} />{:else}<Play size={13} />{/if}</button>
    <button class="b" onclick={pomoSkip} title="Skip to next phase"><SkipForward size={13} /></button>
  </span>
{:else if id === 'timer'}
  <span class="tm">
    <TimerIcon size={16} />
    <span class="t" class:done={timerLeft <= 0}>{fmt(timerLeft)}</span>
    <button class="b" onclick={timerToggle} title={timer.value.endsAt ? 'Pause' : 'Start'}>{#if timer.value.endsAt}<Pause size={13} />{:else}<Play size={13} />{/if}</button>
  </span>
{:else if id === 'stopwatch'}
  <span class="tm">
    <span class="t">{fmt(swElapsed)}</span>
    <button class="b" onclick={swToggle} title={sw.value.startedAt ? 'Pause' : 'Start'}>{#if sw.value.startedAt}<Pause size={13} />{:else}<Play size={13} />{/if}</button>
  </span>
{/if}

<style>
  .todo, .tm { display: flex; align-items: center; gap: 7px; padding: 0 8px 0 10px; height: 100%; max-width: 240px; }
  .tt { font-size: 12.5px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .muted { color: var(--fg-2); }
  .tick { width: 22px; height: 22px; border-radius: 11px; display: grid; place-items: center; border: 1.5px solid var(--fg-3); flex: none; color: transparent; }
  .tick:hover { border-color: var(--good); background: var(--good); color: white; }
  .t { font: 600 13.5px var(--font-display); font-variant-numeric: tabular-nums; min-width: 38px; }
  .t.done { color: var(--danger); }
  .b { width: 24px; height: 24px; border-radius: 12px; display: grid; place-items: center; }
  .b:hover { background: var(--surface-hover); }
</style>
