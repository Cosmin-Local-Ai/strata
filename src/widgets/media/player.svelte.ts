// The player the idle Now Playing tile offers: the last app that played, else Spotify when it is
// installed (asked once from Rust), else nothing.
import { invoke } from '@tauri-apps/api/core'

let spotify = $state<[string, string] | null | undefined>(undefined)
function ask() {
  if (spotify !== undefined) return
  spotify = null
  invoke<[string, string] | null>('default_player').then((v) => (spotify = v ?? null)).catch(() => {})
}

/** SMTC app ids are an exe name ("Spotify.exe"), an AUMID ("Pkg_hash!App") or a browser id. */
function describe(app: string): { name: string; icon: string; target: string } {
  if (/spotify/i.test(app)) return { name: 'Spotify', icon: spotify?.[0] ?? String.raw`%APPDATA%\Spotify\Spotify.exe`, target: 'spotify:' }
  if (app.includes('!')) {
    const pkg = app.split('!')[0].split('_')[0]
    return { name: pkg.split('.').pop() || pkg, icon: 'shell:AppsFolder\\' + app, target: 'shell:AppsFolder\\' + app }
  }
  const name = (app.split(/[\/]/).pop() || app).replace(/\.exe$/i, '')
  return { name, icon: app, target: app }
}

export function idlePlayer() {
  ask()
  return {
    get value() {
      const last = localStorage.getItem('np:last')
      if (last) return describe(last)
      if (spotify) return { name: 'Spotify', icon: spotify[0], target: spotify[1] }
      return null
    },
  }
}

export function idleTarget(): string | null {
  const last = localStorage.getItem('np:last')
  if (last) return describe(last).target
  return spotify ? spotify[1] : null
}

// The last track, kept across closing the player and restarting the PC, so the idle tile still
// shows what you were listening to (cover, title, artist) and can pick it up again.
export type LastTrack = { app: string; title: string; artist: string; art: string }
const TRACK_KEY = 'np:track'
let track = $state<LastTrack | null>(read())
function read(): LastTrack | null {
  try { return JSON.parse(localStorage.getItem(TRACK_KEY) || 'null') } catch { return null }
}
export function rememberTrack(t: LastTrack) {
  if (!t.title) return
  if (track && track.title === t.title && track.artist === t.artist && track.app === t.app && (track.art === t.art || !t.art)) return
  const next = { ...t, art: t.art || (track?.title === t.title ? track.art : '') }
  track = next
  try { localStorage.setItem(TRACK_KEY, JSON.stringify(next)) } catch { localStorage.setItem(TRACK_KEY, JSON.stringify({ ...next, art: '' })) }
}
export function lastTrack() { return { get value() { return track } } }
// Other dock windows (one per screen) write the same key: stay in sync.
window.addEventListener('storage', (e) => { if (e.key === TRACK_KEY) track = read() })

/** Open the player and, once its media session shows up, resume playback (Spotify resumes the
 *  last track on Play). Gives up quietly after 20 s. */
let resumeUntil = 0
export function openAndResume() {
  const t = idleTarget()
  if (!t) return
  invoke('launch', { target: t })
  resumeUntil = Date.now() + 20000
}
export function maybeResume(m: { active?: boolean; playing?: boolean; app?: string } | null | undefined) {
  if (!resumeUntil || !m?.active) return
  if (Date.now() > resumeUntil) { resumeUntil = 0; return }
  if (m.playing) { resumeUntil = 0; return }
  resumeUntil = 0
  // A short delay: a session that just appeared often ignores the first command.
  setTimeout(() => invoke('media_control', { action: 'play' }), 900)
}
