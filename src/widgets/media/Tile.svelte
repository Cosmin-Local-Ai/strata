<script lang="ts">
  // Now Playing: cover art (or the app's icon), title and artist, play/pause and skip. Any app that
  // reports to Windows' media controls (Spotify, browsers, ...).
  // Lyrics mode (item option `lyrics`, opt-in): the current synced lyric line replaces title/artist.
  import { Music, Pause, Play, SkipBack, SkipForward } from '@lucide/svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { iconUrl } from '../../lib/ipc'
  import { media, startMediaAudio } from '../../lib/stores.svelte'
  import type { TileProps } from '../types'
  import { useLyrics } from './lyrics.svelte'
  import { idlePlayer, lastTrack, maybeResume, openAndResume, rememberTrack } from './player.svelte'
  let { item }: TileProps = $props()
  startMediaAudio()
  const m = $derived(media.value)
  let artFailed = $state(false)
  const ctl = (action: string, e: MouseEvent) => { e.stopPropagation(); invoke('media_control', { action }) }


  // The "playing" bars are drawn natively in their own slot next to the title (never over the art) by the Rust side (glass/eq.rs): smooth,
  // reacting to the real audio level, and no Chromium repaints. We only report where the art is.
  let artEl: HTMLElement | undefined = $state()
  $effect(() => {
    const playing = !!m?.playing
    const el = artEl
    if (!el || !playing) { invoke('eq_rect', { rect: null }); return }
    const send = () => { const r = el.getBoundingClientRect(); invoke('eq_rect', { rect: [r.left, r.top, r.width, r.height] }) }
    send()
    // Follow the slot's size *and* position: other tiles changing width move it without resizing it,
    // and every such change resizes the dock body, so observe that too.
    const ro = new ResizeObserver(send)
    ro.observe(el)
    const body = el.closest('.body')
    if (body) ro.observe(body)
    body?.addEventListener('scroll', send)
    return () => { ro.disconnect(); body?.removeEventListener('scroll', send); invoke('eq_rect', { rect: null }) }
  })

  // Remember the last player so the idle tile can open it again. Before anything has played, offer
  // Spotify only when it is installed; otherwise the tile just says nothing is playing.
  $effect(() => { if (m?.active && m.app) localStorage.setItem('np:last', m.app) })
  $effect(() => { if (m?.active && m.title) rememberTrack({ app: m.app, title: m.title, artist: m.artist, art: m.art }) })
  $effect(() => maybeResume(m))
  const lt = lastTrack()
  const p = idlePlayer()
  // Re-read when the media state changes (that is when np:last can change).
  const idle = $derived.by(() => { void m?.app; void m?.active; return p.value })
  const playerName = $derived(idle?.name ?? '')
  const playerIcon = $derived(idle?.icon ?? '')
  const ly = useLyrics(() => !!item.options.lyrics)
  const line = $derived(ly.lyrics?.synced[ly.index]?.text ?? '')
</script>

{#if m?.active && m.title}
  <span class="np">
    <span class="art">
      {#if m.art}<img src={m.art} alt="" />{:else if !artFailed}<img src={iconUrl(`shell:AppsFolder\\${m.app}`, 64)} alt="" onerror={() => (artFailed = true)} />{:else}<Music size={16} />{/if}
      
    </span>
    {#if item.options.lyrics && ly.lyrics?.synced.length}
      <span class="text lyr">
        {#key ly.index}<span class="line">{line || '♪'}</span>{/key}
        <span class="a">{m.title}</span>
      </span>
    {:else}
      <span class="text"><span class="t">{m.title}</span><span class="a">{m.artist}</span></span>
    {/if}
    <span class="eqslot" class:on={m.playing} bind:this={artEl} aria-hidden="true"></span>
    <span class="ctl">
      <button disabled={!m.canPrev} onclick={(e) => ctl('prev', e)} aria-label="Previous"><SkipBack size={14} /></button>
      <button onclick={(e) => ctl('toggle', e)} aria-label={m.playing ? 'Pause' : 'Play'}>{#if m.playing}<Pause size={16} />{:else}<Play size={16} />{/if}</button>
      <button disabled={!m.canNext} onclick={(e) => ctl('next', e)} aria-label="Next"><SkipForward size={14} /></button>
    </span>
  </span>
{:else}
  <!-- Nothing playing: stay on the dock as a launcher for the last player (Spotify if installed). -->
  {#if lt.value && playerName}
    <!-- Remembered track: the player is closed (or the PC restarted); play reopens it and resumes. -->
    <span class="np idle-track" title="Open {playerName}">
      <span class="art">{#if lt.value.art}<img src={lt.value.art} alt="" />{:else if !artFailed}<img src={iconUrl(playerIcon, 64)} alt="" onerror={() => (artFailed = true)} />{:else}<Music size={16} />{/if}</span>
      <span class="text"><span class="t">{lt.value.title}</span><span class="a">{lt.value.artist || playerName}</span></span>
      <span class="ctl"><button onclick={(e) => { e.stopPropagation(); openAndResume() }} aria-label="Open {playerName} and play"><Play size={16} /></button></span>
    </span>
  {:else if playerName}
    <span class="idle" title="Click to open {playerName}">
      <span class="art">{#if !artFailed}<img src={iconUrl(playerIcon, 64)} alt="" onerror={() => (artFailed = true)} />{:else}<Music size={16} />{/if}</span>
      <span class="text"><span class="t">{playerName}</span><span class="a">Click to open</span></span>
    </span>
  {:else}
    <span class="idle">
      <span class="art"><Music size={16} /></span>
      <span class="text"><span class="t">Nothing playing</span><span class="a">Play something to control it here</span></span>
    </span>
  {/if}
{/if}

<style>
  .np { display: flex; align-items: center; gap: 10px; padding: 0 8px 0 6px; height: 100%; }
  /* Text keeps its own width budget; the bars' slot is added on top of it, never taken from it. */
  .text { max-width: 220px; }
  .art { position: relative; flex: none; width: calc(var(--tile) - 12px); height: calc(var(--tile) - 12px); border-radius: 8px; overflow: hidden;
         display: grid; place-items: center; background: var(--surface); }
  .art img { width: 100%; height: 100%; object-fit: cover; }
  .text { display: flex; flex-direction: column; min-width: 0; line-height: 1.2; }
  .t { font-weight: 600; font-size: 12.5px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .a { font-size: 11px; color: var(--fg-2); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .lyr { width: 200px; }
  .line { font-weight: 600; font-size: 12.5px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; animation: in var(--t-slow) var(--ease-out); }
  @keyframes in { from { opacity: 0; transform: translateY(4px); } }
  .eqslot { flex: none; width: 0; height: 13px; margin: 0 -5px; align-self: center; transition: width var(--t-med) var(--ease-out), margin var(--t-med) var(--ease-out); }
  .eqslot.on { width: 13px; margin: 0 2px 0 4px; }
  .ctl { display: flex; align-items: center; gap: 1px; flex: none; }
  .ctl button { width: 26px; height: 26px; border-radius: 13px; display: grid; place-items: center; }
  .ctl button:hover:not(:disabled) { background: var(--surface-hover); }
  .ctl button:active:not(:disabled) { transform: scale(0.9); }
  .ctl button:disabled { opacity: 0.35; }
  .idle-track { opacity: 0.85; }
  .idle-track .art img { filter: saturate(0.85); }
  .idle { display: flex; align-items: center; gap: 10px; padding: 0 12px 0 6px; height: 100%; opacity: 0.85; }
  .idle-old { display: grid; place-items: center; width: var(--tile); height: 100%; color: var(--fg-3); }
</style>
