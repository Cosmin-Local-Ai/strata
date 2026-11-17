<script lang="ts">
  // Audio output: icon follows level/mute; scroll over it to change volume, click for the card.
  import { Volume, Volume1, Volume2, VolumeX } from '@lucide/svelte'
  import { audio, startMediaAudio } from '../../lib/stores.svelte'
  import type { TileProps } from '../types'
  let { }: TileProps = $props()
  startMediaAudio()
  const a = $derived(audio.value)
  const Icon = $derived(!a || a.muted || a.volume === 0 ? VolumeX : a.volume < 34 ? Volume : a.volume < 67 ? Volume1 : Volume2)
</script>

<span class="au" class:off={!a?.available}>
  <Icon size={18} strokeWidth={1.9} />
  {#if a?.available}<span class="v">{a.muted ? 'Muted' : `${a.volume}%`}</span>{/if}
</span>

<style>
  .au { display: flex; align-items: center; gap: 6px; padding: 0 12px; height: 100%; }
  .v { font-size: 12.5px; font-weight: 600; font-variant-numeric: tabular-nums; min-width: 34px; }
  .off { opacity: 0.4; }
</style>
