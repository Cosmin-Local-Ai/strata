<script lang="ts">
  // Full-screen overlay window (index.html#overlay). Currently hosts the power panel.
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import { onMount } from 'svelte'
  import { ipc } from '../lib/ipc'
  import PowerPanel from '../widgets/power/Panel.svelte'
  import Picker from './Picker.svelte'
  import RegionSelect from './RegionSelect.svelte'

  let kind = $state<string | null>(null)
  let gen = $state(0)
  const close = () => { ipc.overlayClose() }
  // Show the window only after the new content has actually painted (no empty/stale frame).
  const painted = () => setTimeout(() => requestAnimationFrame(() => invoke('overlay_ready')), 16)

  onMount(() => {
    invoke<string | null>('overlay_pending').then((k) => { if (k) { kind = k; gen++; painted() } })
    const subs = [
      listen<{ kind: string }>('overlay:open', (e) => { kind = e.payload.kind; gen++; painted() }),
      listen('overlay:close', () => (kind = null)),
    ]
    const esc = (e: KeyboardEvent) => { if (e.key === 'Escape') close() }
    window.addEventListener('keydown', esc)
    return () => { subs.forEach((p) => p.then((f) => f())); window.removeEventListener('keydown', esc) }
  })
</script>

{#key gen}
  {#if kind === 'power'}<PowerPanel {close} />{:else if kind === 'widgets'}<Picker {close} />{:else if kind?.startsWith('capture:')}<RegionSelect {close} text={kind === 'capture:text'} />{/if}
{/key}
