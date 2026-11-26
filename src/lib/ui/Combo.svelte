<script lang="ts">
  // Searchable picker that opens *inside* the card (the card grows), so it never leaves the screen,
  // whether the dock is at the top or the bottom. Native <select> popups ignore the dock position.
  import { Check, ChevronDown, Search } from '@lucide/svelte'
  let { value, options, onpick, open = $bindable(false) }:
    { value: string; options: { value: string; label: string; hint?: string }[]; onpick: (v: string) => void; open?: boolean } = $props()
  let q = $state('')
  let input = $state<HTMLInputElement>()
  $effect(() => { if (open) { q = ''; queueMicrotask(() => input?.focus()) } })
  const shown = $derived(options.filter((o) => !q || `${o.value} ${o.label} ${o.hint ?? ''}`.toLowerCase().includes(q.trim().toLowerCase())))
  const cur = $derived(options.find((o) => o.value === value))
  function pick(v: string) { onpick(v); open = false }
</script>

<button class="field" class:open onclick={() => (open = !open)}>
  <span class="v">{cur?.value ?? value}</span><span class="l">{cur?.label ?? ''}</span><ChevronDown size={13} />
</button>
{#if open}
  <div class="pop">
    <label class="sr"><Search size={13} />
      <input bind:this={input} bind:value={q} placeholder="Search by code or name" spellcheck="false" autocomplete="off"
             onkeydown={(e) => { if (e.key === 'Enter' && shown[0]) pick(shown[0].value); if (e.key === 'Escape') { e.stopPropagation(); open = false } }} />
    </label>
    <ul>
      {#each shown as o (o.value)}
        <li><button class:sel={o.value === value} onclick={() => pick(o.value)}><span class="v">{o.value}</span><span class="l">{o.label}</span>{#if o.value === value}<Check size={13} />{/if}</button></li>
      {:else}<li class="none">No match</li>{/each}
    </ul>
  </div>
{/if}

<style>
  .field { width: 100%; display: flex; align-items: center; gap: 6px; padding: 7px 9px; border-radius: 9px; border: 1px solid var(--border); background: var(--surface); color: var(--fg-2); min-width: 0; }
  .field.open { border-color: var(--accent); }
  .field .v, li .v { font-weight: 600; color: var(--fg); font-size: 13px; }
  .field .l { flex: 1; min-width: 0; text-align: left; font-size: 11.5px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .field:hover { border-color: color-mix(in srgb, var(--fg) 22%, transparent); }
  .pop { grid-column: 1 / -1; order: 10; flex-basis: 100%; width: 100%; margin-top: 2px; border-radius: 11px; border: 1px solid var(--border); background: var(--surface); padding: 6px; box-sizing: border-box; }
  /* Same look as the fields above: a bordered box with a search icon, accent border while typing. */
  .sr { display: flex; align-items: center; gap: 7px; padding: 0 9px; height: 32px; border-radius: 8px; border: 1px solid var(--border); background: rgba(0, 0, 0, 0.2); color: var(--fg-3); }
  .sr:focus-within { border-color: var(--accent); }
  .sr input { flex: 1; min-width: 0; background: none; border: 0; outline: none; color: var(--fg); font-size: 12.5px; padding: 0; }
  .sr input::placeholder { color: var(--fg-3); }
  ul { list-style: none; margin: 6px 0 0; padding: 0 2px 0 0; max-height: 216px; overflow-y: auto; scrollbar-width: thin; }
  li button { width: 100%; display: flex; gap: 8px; align-items: center; padding: 6px 8px; border-radius: 7px; text-align: left; }
  li button:hover { background: var(--surface-hover); }
  li button.sel { background: color-mix(in srgb, var(--accent) 18%, transparent); color: var(--accent); }
  li .v { flex: none; min-width: 34px; }
  li .l { flex: 1; min-width: 0; font-size: 12px; color: var(--fg-2); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .none { color: var(--fg-3); font-size: 12px; padding: 6px 8px; }
</style>
