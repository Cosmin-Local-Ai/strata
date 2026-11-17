<script lang="ts">
  // Power menu: shut down, restart, sign out, hibernate, sleep
  // plus Lock. Destructive actions ask for a second click so a stray click can't end the session.
  import { POWER_ACTIONS as actions } from './actions'
  import { ipc } from '../../lib/ipc'
  import type { CardProps } from '../types'
  let { close }: CardProps = $props()

  let armed = $state<string | null>(null)
  let error = $state('')
  let up = $state(0)
  ipc.uptime().then((s) => (up = s))
  const uptime = $derived(`${Math.floor(up / 86400)}d ${Math.floor((up % 86400) / 3600)}h ${Math.floor((up % 3600) / 60)}m`)
  let disarm: ReturnType<typeof setTimeout> | undefined

  async function run(a: (typeof actions)[number]) {
    if (a.confirm && armed !== a.id) {
      armed = a.id
      clearTimeout(disarm)
      disarm = setTimeout(() => (armed = null), 3000)
      return
    }
    try {
      close()
      await ipc.power(a.id)
    } catch (e) {
      error = String(e)
    }
  }
</script>

<div class="pm">
  <div class="grid">
    {#each actions as a (a.id)}
      <button class:armed={armed === a.id} class:danger={a.confirm} onclick={() => run(a)}>
        <a.icon size={22} strokeWidth={1.8} />
        <span>{armed === a.id ? 'Click again' : a.label}</span>
      </button>
    {/each}
  </div>
  <footer>Up {uptime}{#if error} · <span class="err">{error}</span>{/if}</footer>
</div>

<style>
  .pm { width: 330px; padding: 14px; display: flex; flex-direction: column; gap: 10px; }
  .grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px; }
  button { display: flex; flex-direction: column; align-items: center; gap: 8px; padding: 14px 6px; border-radius: 12px;
           background: var(--surface); font-size: 12px; transition: background var(--t-fast) var(--ease-out), transform var(--t-fast) var(--ease-out); }
  button:hover { background: var(--surface-hover); }
  button:active { transform: scale(0.96); }
  button.armed { background: var(--danger); color: white; }
  footer { font-size: 11.5px; color: var(--fg-3); text-align: center; }
  .err { color: var(--danger); }
</style>
