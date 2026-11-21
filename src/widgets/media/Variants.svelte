<script lang="ts">
  // Now Playing variants: compact (art + title), mini (round play button with a progress ring),
  // bar (title over a progress bar, with skip). All open the Now Playing card on click.
  import { Music, Pause, Play, SkipBack, SkipForward } from '@lucide/svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { iconUrl } from '../../lib/ipc'
  import { clock, media, startMediaAudio } from '../../lib/stores.svelte'
  import type { TileProps } from '../types'
  import { lastTrack, openAndResume } from './player.svelte'
  const lt = lastTrack()
  let { item }: TileProps = $props()
  startMediaAudio()
  const m = $derived(media.value)
  const c = clock(1000)
  const pos = $derived(m ? Math.min(m.duration, m.position + (m.playing ? (+c.now - m.sampledAt) / 1000 : 0)) : 0)
  const frac = $derived(m?.duration ? pos / m.duration : 0)
  const ctl = (action: string, e: MouseEvent) => { e.stopPropagation(); invoke('media_control', { action }) }
  let artFailed = $state(false)
  $effect(() => { m?.art; artFailed = false })
  const R = 14, C = 2 * Math.PI * R
</script>

{#if item.widget === 'now-playing-art' && (m?.active || lt.value)}
  <!-- Cover art blurred behind the title and the controls; the last track while the player is closed. -->
  {@const t = m?.active ? { title: m.title, artist: m.artist, art: m.art } : lt.value!}
  <span class="art" class:off={!m?.active}>
    {#if t.art}<img class="bg" src={t.art} alt="" />{/if}
    <span class="shade"></span>
    <span class="fg">
      <span class="tt">{t.title}{t.artist ? ` · ${t.artist}` : ''}</span>
      <span class="cs">
        {#if m?.active}
          <button onclick={(e) => ctl('prev', e)} aria-label="Previous"><SkipBack size={13} fill="currentColor" /></button>
          <button onclick={(e) => ctl('playpause', e)} aria-label={m.playing ? 'Pause' : 'Play'}>{#if m.playing}<Pause size={14} fill="currentColor" />{:else}<Play size={14} fill="currentColor" />{/if}</button>
          <button onclick={(e) => ctl('next', e)} aria-label="Next"><SkipForward size={13} fill="currentColor" /></button>
        {:else}
          <button onclick={(e) => { e.stopPropagation(); openAndResume() }} aria-label="Open the player and play"><Play size={14} fill="currentColor" /></button>
        {/if}
      </span>
    </span>
  </span>
{:else if !m?.active}
  <span class="idle"><Music size={16} />{#if item.widget !== 'now-playing-mini'}<small>Nothing playing</small>{/if}</span>
{:else if item.widget === 'now-playing-mini'}
  <button class="mini" onclick={(e) => ctl('playpause', e)} aria-label={m.playing ? 'Pause' : 'Play'} title={`${m.title} · ${m.artist}`}>
    <svg viewBox="0 0 34 34" width="34" height="34"><circle cx="17" cy="17" r={R} fill="none" stroke="var(--surface-press)" stroke-width="3" />
      <circle cx="17" cy="17" r={R} fill="none" stroke="var(--accent)" stroke-width="3" stroke-linecap="round" stroke-dasharray={C} stroke-dashoffset={C * (1 - frac)} transform="rotate(-90 17 17)" /></svg>
    <span class="pp">{#if m.playing}<Pause size={13} fill="currentColor" />{:else}<Play size={13} fill="currentColor" />{/if}</span>
  </button>
{:else if item.widget === 'now-playing-bar'}
  <span class="bar">
    <span class="bt"><span class="ti">{m.title}</span><span class="ar">{m.artist}</span></span>
    <span class="prog"><i style="width:{frac * 100}%"></i></span>
    <button class="sk" onclick={(e) => ctl('playpause', e)} aria-label="Play or pause">{#if m.playing}<Pause size={13} fill="currentColor" />{:else}<Play size={13} fill="currentColor" />{/if}</button>
    <button class="sk" onclick={(e) => ctl('next', e)} aria-label="Next"><SkipForward size={13} fill="currentColor" /></button>
  </span>
{:else}
  <span class="compact">
    {#if m.art && !artFailed}<img src={m.art} alt="" onerror={() => (artFailed = true)} />{:else}<img src={iconUrl(m.app, 64)} alt="" />{/if}
    <span class="ti">{m.title}</span>
  </span>
{/if}

<style>
  .art { position: relative; width: 190px; height: 100%; border-radius: var(--tile-r); overflow: hidden; display: block; }
  .art .bg { position: absolute; inset: -12px; width: calc(100% + 24px); height: calc(100% + 24px); object-fit: cover; filter: blur(9px) saturate(1.2); transform: scale(1.1); }
  .art .shade { position: absolute; inset: 0; background: linear-gradient(180deg, rgba(0,0,0,.18), rgba(0,0,0,.42)); }
  .art.off .bg { filter: blur(9px) saturate(0.6); }
  .art .fg { position: relative; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 1px; height: 100%; padding: 0 10px; color: white; text-shadow: 0 1px 2px rgba(0,0,0,.5); }
  .art .tt { max-width: 100%; font: 600 11.5px var(--font-display); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .art .cs { display: flex; gap: 10px; }
  .art .cs button { width: 22px; height: 20px; display: grid; place-items: center; border-radius: 6px; color: white; }
  .art .cs button:hover { background: rgba(255,255,255,.18); }
  .idle { display: flex; align-items: center; gap: 6px; padding: 0 10px; height: 100%; color: var(--fg-3); }
  .idle small { font-size: 11.5px; }
  .mini { position: relative; width: 100%; height: 100%; display: grid; place-items: center; }
  .mini .pp { position: absolute; inset: 0; display: grid; place-items: center; }
  .compact { display: flex; align-items: center; gap: 8px; padding: 0 12px 0 6px; height: 100%; max-width: 200px; }
  .compact img { width: calc(var(--tile) - 14px); height: calc(var(--tile) - 14px); border-radius: 7px; object-fit: cover; flex: none; }
  .ti { font-size: 12.5px; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .bar { display: grid; grid-template-columns: 1fr auto auto; grid-template-rows: auto 3px; column-gap: 4px; row-gap: 4px; align-items: center; padding: 0 8px 0 12px; height: 100%; width: 210px; }
  .bt { display: flex; gap: 6px; align-items: baseline; min-width: 0; grid-column: 1; }
  .ar { font-size: 11px; color: var(--fg-2); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .prog { grid-column: 1; grid-row: 2; height: 3px; border-radius: 2px; background: var(--surface-press); overflow: hidden; }
  .prog i { display: block; height: 100%; background: var(--accent); transition: width 1s linear; }
  .sk { grid-row: 1 / 3; width: 24px; height: 24px; border-radius: 12px; display: grid; place-items: center; }
  .sk:hover { background: var(--surface-hover); }
</style>
