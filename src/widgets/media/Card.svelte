<script lang="ts">
  import { MicVocal, Music, Pause, Play, SkipBack, SkipForward } from '@lucide/svelte'
  import { useLyrics } from './lyrics.svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { media, startMediaAudio } from '../../lib/stores.svelte'
  import type { CardProps } from '../types'
  let { item, act }: CardProps = $props()
  let showLyrics = $state(!!item.options.lyrics)
  const ly = useLyrics(() => showLyrics)
  let list: HTMLDivElement | undefined = $state()
  $effect(() => { const i = ly.index; const el = list?.children[i] as HTMLElement | undefined; el?.scrollIntoView({ block: 'center', behavior: 'smooth' }) })
  function toggleLyrics() { showLyrics = !showLyrics; act('set-options', { lyrics: showLyrics }) }
  startMediaAudio()
  const m = $derived(media.value)
  // Extrapolate position while playing (Windows only reports it on changes).
  let now = $state(Date.now())
  $effect(() => { const t = setInterval(() => (now = Date.now()), 500); return () => clearInterval(t) })
  const pos = $derived(m ? Math.min(m.duration, m.position + (m.playing ? (now - m.sampledAt) / 1000 : 0)) : 0)
  const fmt = (s: number) => `${Math.floor(s / 60)}:${String(Math.floor(s % 60)).padStart(2, '0')}`
  const ctl = (action: string) => invoke('media_control', { action })
  function seek(e: MouseEvent) {
    if (!m?.duration) return
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect()
    invoke('media_seek', { seconds: ((e.clientX - r.left) / r.width) * m.duration })
  }
</script>

<div class="mc">
  {#if m?.active && m.title}
    {#if showLyrics}
      <div class="lyrics" bind:this={list}>
        {#if ly.loading}<p>Looking for lyrics…</p>
        {:else if ly.lyrics?.synced.length}
          {#each ly.lyrics.synced as l, i}<div class="l" class:cur={i === ly.index} class:past={i < ly.index}>{l.text || '♪'}</div>{/each}
        {:else if ly.lyrics?.plain}<pre>{ly.lyrics.plain}</pre>
        {:else if ly.lyrics?.instrumental}<p>Instrumental</p>
        {:else}<p>No lyrics found for this track.</p>{/if}
      </div>
    {:else}
      <div class="cover">{#if m.art}<img src={m.art} alt="" />{:else}<Music size={48} />{/if}</div>
    {/if}
    <div class="t">{m.title}</div>
    <div class="a">{m.artist}{m.album ? ` · ${m.album}` : ''}</div>
    {#if m.duration > 0}
      <div class="bar" role="slider" tabindex="0" aria-valuenow={pos} onclick={seek} onkeydown={() => {}}>
        <span style="width:{(pos / m.duration) * 100}%"></span>
      </div>
      <div class="times"><span>{fmt(pos)}</span><span>-{fmt(m.duration - pos)}</span></div>
    {/if}
    <div class="ctl">
      <button disabled={!m.canPrev} onclick={() => ctl('prev')}><SkipBack size={22} /></button>
      <button class="main" onclick={() => ctl('toggle')}>{#if m.playing}<Pause size={26} />{:else}<Play size={26} />{/if}</button>
      <button disabled={!m.canNext} onclick={() => ctl('next')}><SkipForward size={22} /></button>
    </div>
    <button class="lyrbtn" class:on={showLyrics} onclick={toggleLyrics} title="Synced lyrics from lrclib.net (sends the track name there)"><MicVocal size={14} /> Lyrics</button>
  {:else}
    <p>Nothing is playing. Media from any app that shows in Windows' media controls appears here.</p>
  {/if}
</div>

<style>
  .mc { width: 300px; padding: 16px; display: flex; flex-direction: column; align-items: center; gap: 6px; }
  .cover { width: 220px; height: 220px; border-radius: 14px; overflow: hidden; display: grid; place-items: center; background: var(--surface); margin-bottom: 8px; box-shadow: 0 10px 30px rgba(0,0,0,.35); }
  .cover img { width: 100%; height: 100%; object-fit: cover; }
  .t { font: 600 15px var(--font-display); text-align: center; }
  .a { font-size: 12px; color: var(--fg-2); text-align: center; }
  .bar { width: 100%; height: 5px; border-radius: 3px; background: var(--surface-press); margin-top: 10px; overflow: hidden; }
  .bar span { display: block; height: 100%; background: var(--fg); border-radius: 3px; }
  .times { width: 100%; display: flex; justify-content: space-between; font-size: 11px; color: var(--fg-3); font-variant-numeric: tabular-nums; }
  .ctl { display: flex; align-items: center; gap: 18px; margin-top: 4px; }
  .ctl button { width: 40px; height: 40px; border-radius: 20px; display: grid; place-items: center; }
  .ctl button.main { width: 52px; height: 52px; border-radius: 26px; background: var(--surface-hover); }
  .ctl button:hover:not(:disabled) { background: var(--surface-press); }
  .ctl button:disabled { opacity: 0.35; }
  p { color: var(--fg-2); margin: 0; text-align: center; }
  .lyrics { width: 268px; height: 236px; overflow-y: auto; scrollbar-width: none; display: flex; flex-direction: column; gap: 6px; padding: 90px 0; mask-image: linear-gradient(transparent, #000 25%, #000 75%, transparent); margin-bottom: 8px; }
  .l { font: 600 15px var(--font-display); color: var(--fg-3); transition: color var(--t-med) var(--ease-out), transform var(--t-med) var(--ease-out); transform-origin: left; }
  .l.past { color: var(--fg-2); }
  .l.cur { color: var(--fg); transform: scale(1.04); }
  pre { font: 13px/1.5 var(--font); color: var(--fg-2); white-space: pre-wrap; margin: 0; }
  .lyrbtn { display: flex; align-items: center; gap: 6px; font-size: 12px; color: var(--fg-2); padding: 4px 10px; border-radius: 12px; }
  .lyrbtn.on { background: var(--accent); color: white; }
</style>
