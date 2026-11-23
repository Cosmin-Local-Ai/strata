<script lang="ts">
  import { notice } from '../../lib/notice'
  // Search: web search, or type a folder/file path to open it.
  import { Folder, Search } from '@lucide/svelte'
  import { ipc } from '../../lib/ipc'
  import type { CardProps } from '../types'
  let { item, close }: CardProps = $props()
  let q = $state('')
  let input: HTMLInputElement
  $effect(() => { input?.focus() })
  const engine: string = item.options.engine ?? 'https://www.google.com/search?q='
  const isPath = $derived(/^([a-z]:\\|\\\\|%[a-z]+%|~[\\/]|shell:)/i.test(q.trim()))
  async function go() {
    const t = q.trim()
    if (!t) return
    const target = isPath ? t.replace(/^~[\\/]/, '%USERPROFILE%\\') : engine + encodeURIComponent(t)
    try { await ipc.launch(target); close() } catch (e) { notice(e) }
  }
</script>

<form class="sc" onsubmit={(e) => { e.preventDefault(); go() }}>
  {#if isPath}<Folder size={16} />{:else}<Search size={16} />{/if}
  <input bind:this={input} bind:value={q} placeholder="Search the web or type a folder path" spellcheck="false" />
</form>

<style>
  .sc { width: 420px; display: flex; align-items: center; gap: 10px; padding: 12px 16px; color: var(--fg-2); }
  input { flex: 1; background: none; border: 0; outline: none; font-size: 15px; color: var(--fg); }
</style>
