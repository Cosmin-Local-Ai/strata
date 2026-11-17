<script lang="ts">
  // Full-screen power panel: big cards over a blurred desktop,
  // in Strata's style. The blur is DWM acrylic on the overlay window; this draws the cards.
  import { ipc } from '../../lib/ipc'
  import { settings } from '../../lib/stores.svelte'
  const dim = $derived(settings.value?.powerBackdrop === 'dim')
  import { POWER_ACTIONS, formatUptime } from './actions'

  let { close }: { close: () => void } = $props()
  let armed = $state<string | null>(null)
  let error = $state('')
  let up = $state(0)
  let focusIdx = $state(0)
  ipc.uptime().then((s) => (up = s))
  let disarm: ReturnType<typeof setTimeout> | undefined
  const now = new Date()

  async function run(a: (typeof POWER_ACTIONS)[number]) {
    if (a.confirm && armed !== a.id) {
      armed = a.id
      clearTimeout(disarm)
      disarm = setTimeout(() => (armed = null), 3000)
      return
    }
    close()
    try { await ipc.power(a.id) } catch (e) { error = String(e) }
  }
  function key(e: KeyboardEvent) {
    if (e.key === 'ArrowRight' || e.key === 'Tab') { e.preventDefault(); focusIdx = (focusIdx + 1) % POWER_ACTIONS.length }
    else if (e.key === 'ArrowLeft') { focusIdx = (focusIdx + POWER_ACTIONS.length - 1) % POWER_ACTIONS.length }
    else if (e.key === 'Enter') run(POWER_ACTIONS[focusIdx])
  }
</script>

<svelte:window onkeydown={key} />

<!-- Clicking the backdrop closes; clicks on cards don't bubble to it. -->
<div class="backdrop" class:dim role="presentation" onclick={close}>
  <div class="head">
    <div class="time">{now.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}</div>
    <div class="sub">Up {formatUptime(up)}</div>
  </div>
  <div class="cards" role="presentation" onclick={(e) => e.stopPropagation()}>
    {#each POWER_ACTIONS as a, i (a.id)}
      <button class="card" class:armed={armed === a.id} class:focus={focusIdx === i} style="--i:{i}"
              onclick={() => run(a)} onmouseenter={() => (focusIdx = i)}>
        <a.icon size={54} strokeWidth={1.4} />
        <span>{armed === a.id ? 'Click again' : a.label}</span>
      </button>
    {/each}
  </div>
  <div class="hint">{error || 'Esc or click anywhere to cancel'}</div>
</div>

<style>
  .backdrop { position: fixed; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 40px;
              background: rgba(8, 8, 12, 0.22); animation: fade var(--t-slow) var(--ease-out); }
  .backdrop.dim { background: rgba(6, 6, 10, 0.86); }
  .head { text-align: center; }
  .time { font: 600 64px var(--font-display); letter-spacing: -1px; }
  .sub { color: var(--fg-2); font-size: 15px; }
  .cards { display: flex; gap: 18px; }
  .card { width: 170px; height: 210px; border-radius: 26px; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 22px;
          background: rgba(255, 255, 255, 0.08); border: 1px solid rgba(255, 255, 255, 0.12); font: 600 16px var(--font);
          box-shadow: 0 12px 40px rgba(0, 0, 0, 0.25);
          animation: rise var(--t-slow) var(--ease-spring) both; animation-delay: calc(var(--i) * 25ms);
          transition: transform var(--t-med) var(--ease-out), background var(--t-med) var(--ease-out), border-color var(--t-med) var(--ease-out); }
  .card.focus { background: rgba(255, 255, 255, 0.16); border-color: rgba(255, 255, 255, 0.28); transform: translateY(-4px); }
  .card:active { transform: scale(0.96); }
  .card.armed { background: var(--danger); border-color: transparent; color: white; }
  .hint { color: var(--fg-3); font-size: 13px; }
  @keyframes fade { from { opacity: 0; } }
  @keyframes rise { from { opacity: 0; transform: translateY(16px) scale(0.96); } }
</style>
