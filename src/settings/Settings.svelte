<script lang="ts">
  // Settings window (index.html#settings), styled after macOS System Settings (brief §4.2):
  // sidebar of sections, grouped rows, an info tip on every dock setting, changes apply live.
  import { invoke } from '@tauri-apps/api/core'
  import { AppWindow, Gamepad2, Info, Monitor, Palette, PanelBottom, PanelBottomClose as Taskbar, Sparkles } from '@lucide/svelte'
  import { settings, startStores } from '../lib/stores.svelte'
  import { notesFor, renderNotes } from '../lib/changelog'
  import Row from './Row.svelte'

  startStores()
  document.documentElement.classList.add('auto-theme')
  type Display = { id: string; name: string; gdiName: string; primary: boolean; width: number; height: number; enabled: boolean }
  type TaskbarStatus = { state: 'managed' | 'left' | 'blocked'; hidden?: boolean; by?: string }

  const sections = [
    { id: 'dock', label: 'Dock', icon: PanelBottom },
    { id: 'appearance', label: 'Appearance', icon: Palette },
    { id: 'displays', label: 'Displays', icon: Monitor },
    { id: 'taskbar', label: 'Windows taskbar', icon: Taskbar },
    { id: 'glass', label: 'Glass', icon: Sparkles },
    { id: 'game', label: 'Game & AI mode', icon: Gamepad2 },
    { id: 'about', label: 'About', icon: Info },
  ]
  let section = $state('dock')
  let displays = $state<Display[]>([])
  let tb = $state<TaskbarStatus | null>(null)
  let error = $state('')
  const s = $derived(settings.value)

  const loadDisplays = () => invoke<Display[]>('displays_list').then((d) => (displays = d))
  loadDisplays()
  invoke<TaskbarStatus>('taskbar_state').then((t) => (tb = t))

  async function resetAll() {
    error = ''
    try { settings.value = await invoke('settings_reset') } catch (e) { error = String(e) }
  }
  let newApp = $state('')
  function addYield() {
    const v = newApp.trim()
    if (v && s && !s.yieldApps.includes(v)) set({ yieldApps: [...s.yieldApps, v] })
    newApp = ''
  }

  let autostart = $state(false)
  invoke<boolean>('autostart_get').then((v) => (autostart = v))
  let version = $state('')
  import('@tauri-apps/api/app').then((m) => m.getVersion()).then((v) => (version = v))
  // Updates: show what changed first, install on request.
  let updateMsg = $state(''), updating = $state(false)
  let pending = $state<{ version: string; notes: string; install: () => Promise<void> } | null>(null)
  async function checkUpdate() {
    updating = true; updateMsg = 'Checking…'
    try {
      const { check } = await import('@tauri-apps/plugin-updater')
      const u = await check()
      if (!u) { updateMsg = 'You have the latest version'; return }
      updateMsg = ''
      pending = { version: u.version, notes: u.body ?? '', install: async () => {
        updating = true; updateMsg = `Installing ${u.version}…`
        try {
          await u.downloadAndInstall()
          const { relaunch } = await import('@tauri-apps/plugin-process')
          await relaunch()
        } catch (e) { updateMsg = `Couldn't install: ${String(e).slice(0, 80)}` } finally { updating = false }
      } }
    } catch (e) { updateMsg = `Couldn't check: ${String(e).slice(0, 80)}` } finally { updating = false }
  }
  const currentNotes = $derived(version ? notesFor(version) : '')
  async function set(patch: Record<string, unknown>) {
    error = ''
    try { settings.value = await invoke('settings_update', { patch }) } catch (e) { error = String(e) }
  }
  const SIZES = [{ id: 'S', h: 48 }, { id: 'M', h: 56 }, { id: 'L', h: 64 }]
  // Minimal: a slim classic status bar (28 px, full width, flush to the edge).
  const MINIMAL = { dockHeight: 28, edgeDistance: 0, fullWidth: true, padding: 4, spacing: 4 }
  const isMinimal = $derived(s?.dockHeight === 28 && s?.fullWidth && s?.edgeDistance === 0)
  const STYLES = [
    { id: 'liquidGlass', label: 'Liquid Glass' }, { id: 'iosGlass', label: 'iOS Glass' }, { id: 'clear', label: 'Clear' },
    { id: 'black', label: 'Black' }, { id: 'white', label: 'White' }, { id: 'midnight', label: 'Midnight' },
  ]
  const ACCENTS = ['#0a84ff', '#5e5ce6', '#bf5af2', '#ff375f', '#ff9f0a', '#ffd60a', '#30d158', '#64d2ff', '#8e8e93']
  async function toggleDisplay(d: Display) {
    const displaysPref = { ...(s?.displays ?? {}) }
    displaysPref[d.id] = { enabled: !d.enabled, name: d.name }
    await set({ displays: displaysPref })
    loadDisplays()
  }
  function isAux(d: Display) { return !d.primary && (d.height > d.width || d.width < 1000) }
</script>

{#if s}
  <div class="win">
    <nav>
      <div class="brand"><AppWindow size={16} /> Strata</div>
      {#each sections as x}
        <button class:sel={section === x.id} onclick={() => (section = x.id)}><x.icon size={16} /> {x.label}</button>
      {/each}
    </nav>
    <main>
      <h1>{sections.find((x) => x.id === section)?.label}</h1>
      {#if error}<p class="error">{error}</p>{/if}

      {#if section === 'dock'}
        <div class="actions"><button class="reset" onclick={resetAll} title="Restore every setting to its default. Your dock widgets and display choices are kept.">Reset to defaults</button></div>
        <div class="group">
          <Row label="Position" tip="Which screen edge the dock sits on. Cards open toward the middle of the screen.">
            <div class="seg">
              <button class:on={s.edge === 'bottom'} onclick={() => set({ edge: 'bottom' })}>Bottom</button>
              <button class:on={s.edge === 'top'} onclick={() => set({ edge: 'top' })}>Top</button>
            </div>
          </Row>
          <Row label="Size" tip="Minimal (a slim 28 px full-width bar), S, M and L presets, or drag for any height between 24 and 120 px.">
            <div class="seg">
              <button class:on={isMinimal} onclick={() => set(MINIMAL)} title="A slim full-width bar">Minimal</button>
              {#each SIZES as z}<button class:on={s.dockHeight === z.h && !isMinimal} onclick={() => set({ dockHeight: z.h, ...(isMinimal ? { edgeDistance: 5, fullWidth: false, padding: 8, spacing: 8 } : {}) })}>{z.id}</button>{/each}
            </div>
            <input type="range" min="24" max="120" value={s.dockHeight} onchange={(e) => set({ dockHeight: +e.currentTarget.value })} />
            <span class="val">{s.dockHeight}px</span>
          </Row>
          <Row label="Distance from edge" tip="Gap between the dock and the screen edge (0–24 px).">
            <input type="range" min="0" max="24" value={s.edgeDistance} onchange={(e) => set({ edgeDistance: +e.currentTarget.value })} />
            <span class="val">{s.edgeDistance}px</span>
          </Row>
          <Row label="Padding" tip="Space inside the dock around the widgets (0–24 px).">
            <input type="range" min="0" max="24" value={s.padding} onchange={(e) => set({ padding: +e.currentTarget.value })} />
            <span class="val">{s.padding}px</span>
          </Row>
          <Row label="Spacing" tip="Space between widgets (0–24 px).">
            <input type="range" min="0" max="24" value={s.spacing} onchange={(e) => set({ spacing: +e.currentTarget.value })} />
            <span class="val">{s.spacing}px</span>
          </Row>
        </div>
        <div class="group">
          <Row label="Full width" tip="Stretch the dock across the whole screen instead of a floating pill. On a 32:9 screen a pill with a width limit is usually nicer.">
            <input type="checkbox" class="sw" checked={s.fullWidth} onchange={(e) => set({ fullWidth: e.currentTarget.checked })} />
          </Row>
          {#if !s.fullWidth}
            <Row label="Maximum width" tip="Widest the pill may grow, as a share of the screen; beyond it the dock scrolls.">
              <input type="range" min="20" max="100" value={Math.round(s.maxWidthFraction * 100)} onchange={(e) => set({ maxWidthFraction: +e.currentTarget.value / 100 })} />
              <span class="val">{Math.round(s.maxWidthFraction * 100)}%</span>
            </Row>
          {/if}
          <Row label="Names on hover" tip="Show a widget's name above it when you hover it.">
            <input type="checkbox" class="sw" checked={s.hoverNames} onchange={(e) => set({ hoverNames: e.currentTarget.checked })} />
          </Row>
          <Row label="Window previews" tip="Hovering an app icon shows live previews of its windows, with title and close button.">
            <input type="checkbox" class="sw" checked={s.windowPreviews} onchange={(e) => set({ windowPreviews: e.currentTarget.checked })} />
          </Row>
          <Row label="Magnification" tip="Mac-style: icons grow as the pointer passes over them.">
            <input type="checkbox" class="sw" checked={s.magnification} onchange={(e) => set({ magnification: e.currentTarget.checked })} />
          </Row>
          {#if s.magnification}
            <Row label="Magnification size" tip="How large icons grow under the pointer.">
              <input type="range" min="110" max="200" value={Math.round(s.magnificationScale * 100)} onchange={(e) => set({ magnificationScale: +e.currentTarget.value / 100 })} />
              <span class="val">{Math.round(s.magnificationScale * 100)}%</span>
            </Row>
          {/if}
          <Row label="Scroll moves the dock" tip="For smaller screens: the mouse wheel slides the dock left and right when it has more widgets than fit. Hold Ctrl to use a widget's own scroll (volume, brightness).">
            <input type="checkbox" class="sw" checked={!!s.wheelScrollsDock} onchange={(e) => set({ wheelScrollsDock: e.currentTarget.checked })} />
          </Row>
          <Row label="Auto-hide" tip="The dock slides away and returns when you touch the screen edge. While hidden it reserves no space. (Slide animation: in progress.)">
            <input type="checkbox" class="sw" checked={s.autoHide} onchange={(e) => set({ autoHide: e.currentTarget.checked })} />
          </Row>
        </div>
      {:else if section === 'appearance'}
        <div class="group">
          <Row label="Style" tip="Dock look. Glass styles bend and frost what's behind the dock.">
            <div class="styles">
              {#each STYLES as st}<button class="style {st.id}" class:on={s.style === st.id} onclick={() => set({ style: st.id })}><span></span>{st.label}</button>{/each}
            </div>
          </Row>
          <Row label="Accent colour" tip="Used for switches, sliders, highlights and the focused workspace.">
            <div class="accents">
              {#each ACCENTS as a}<button class="acc" class:on={s.accent === a} style="background:{a}" onclick={() => set({ accent: a })} aria-label={a}></button>{/each}
            </div>
          </Row>
          <Row label="Power menu background" tip="Blurred desktop: your screen, frosted, behind the power cards . Dim: a plain dark backdrop.">
            <div class="seg">
              <button class:on={s.powerBackdrop !== "dim"} onclick={() => set({ powerBackdrop: "blur" })}>Blurred desktop</button>
              <button class:on={s.powerBackdrop === "dim"} onclick={() => set({ powerBackdrop: "dim" })}>Dim</button>
            </div>
          </Row>
        </div>
      {:else if section === 'displays'}
        <p class="note">Docks are remembered per screen. Auxiliary screens (like a case display) are off unless you turn them on.</p>
        <div class="group">
          {#each displays as d}
            <Row label={`${d.primary ? 'Main display' : d.name} (${d.gdiName.replace('\\\\.\\', '')})`} tip={`${d.width}×${d.height}${isAux(d) ? ' · looks like an auxiliary screen' : ''}`}>
              <input type="checkbox" class="sw" checked={d.enabled} onchange={() => toggleDisplay(d)} />
            </Row>
          {/each}
        </div>
      {:else if section === 'taskbar'}
        {#if tb?.state === 'blocked'}
          <p class="warn">{tb.by} is managing the Windows taskbar, so Strata leaves it alone. Remove Taskbar Hide from your Startup folder and restart Strata to let Strata manage it (it restores the taskbar on quit, crash or kill).</p>
        {/if}
        <div class="group">
          <Row label="While Strata runs" tip="No Windows files are changed. Quitting Strata, a crash or a kill always brings the taskbar back.">
            <div class="seg">
              <button class:on={s.taskbarMode === 'autoHide'} onclick={() => set({ taskbarMode: 'autoHide' })}>Auto-hide</button>
              <button class:on={s.taskbarMode === 'hidden'} onclick={() => set({ taskbarMode: 'hidden' })}>Hidden</button>
              <button class:on={s.taskbarMode === 'leave'} onclick={() => set({ taskbarMode: 'leave' })}>Leave alone</button>
            </div>
          </Row>
        </div>
      {:else if section === 'glass'}
        <div class="group">
          <Row label="Live glass" tip="Off: glass shows your wallpaper as a still image: no screen capture, no GPU use. On: glass follows animated wallpapers live by capturing your main screen, but then the dock does NOT appear in screenshots or OBS display capture, and the GPU driver is loaded (game overlays may notice).">
            <input type="checkbox" class="sw" checked={s.liveGlass} onchange={(e) => set({ liveGlass: e.currentTarget.checked })} />
          </Row>
          {#if s.liveGlass}
            <Row label="Live glass frame rate" tip="Upper limit for refreshing glass over moving content.">
              <input type="range" min="10" max="60" step="5" value={s.liveGlassFps} onchange={(e) => set({ liveGlassFps: +e.currentTarget.value })} />
              <span class="val">{s.liveGlassFps} fps</span>
            </Row>
          {/if}
        </div>
      {:else if section === 'game'}
        <p class="note">While a game or local AI is running, Strata stops all its GPU work, frees memory and freezes animations. Under a fullscreen game the dock also hides.</p>
        <div class="group">
          <Row label="Steam games" tip="Any game running from your Steam libraries (tools like Wallpaper Engine are ignored).">
            <input type="checkbox" class="sw" checked={s.yieldForSteamGames} onchange={(e) => set({ yieldForSteamGames: e.currentTarget.checked })} />
          </Row>
        </div>
        <h2 class="sub">Also yield to these programs</h2>
        <p class="note">While any of these runs, Strata stays out of the way. Use <code>name.exe:word</code> to match only when the program's command line contains that word (Ollama's model runner is <code>ollama.exe:runner</code>).</p>
        <div class="group list">
          {#each s.yieldApps as a (a)}
            <div class="li"><span>{a}</span><button class="x" onclick={() => set({ yieldApps: s.yieldApps.filter((x: string) => x !== a) })} aria-label="Remove">×</button></div>
          {/each}
          <div class="li add"><input bind:value={newApp} placeholder="Add a program, e.g. comfyui.exe" onkeydown={(e) => e.key === 'Enter' && addYield()} /><button onclick={addYield}>Add</button></div>
        </div>
      {:else if section === 'about'}
        <div class="group">
          <Row label="Version" tip="">{version}</Row>
          <Row label="Start with Windows" tip="Listed in Task Manager → Startup apps. Strata starts minimised into the dock when you sign in.">
            <input type="checkbox" class="sw" checked={autostart} onchange={async (e) => { await invoke('autostart_set', { on: e.currentTarget.checked }); autostart = await invoke<boolean>('autostart_get') }} />
          </Row>
          <Row label="Updates" tip="Checks the Strata releases on GitHub. Updates are signed; Strata verifies them before installing.">
            <button class="btn" onclick={checkUpdate} disabled={updating}>{updateMsg || 'Check for updates'}</button>
          </Row>
          {#if pending}
            <div class="notes">
              <div class="nh"><strong>Strata {pending.version} is available</strong><span class="sp"></span>
                <button class="btn" onclick={() => (pending = null)} disabled={updating}>Later</button>
                <button class="btn primary" onclick={() => pending?.install()} disabled={updating}>{updating ? updateMsg : 'Install and restart'}</button>
              </div>
              <div class="md">{@html renderNotes(pending.notes || 'No notes were published for this version.')}</div>
            </div>
          {/if}
          <Row label="Your data" tip="Settings, notes and launchpad live in %APPDATA%\Strata. Nothing is sent anywhere.">%APPDATA%\Strata</Row>
        </div>
        {#if currentNotes}
          <div class="notes"><div class="nh"><strong>What's new in {version}</strong></div><div class="md">{@html renderNotes(currentNotes)}</div></div>
        {/if}
        <button class="quit" onclick={() => invoke('quit')}>Quit Strata</button>
      {/if}
    </main>
  </div>
{/if}

<style>
  .win { position: fixed; inset: 0; display: flex; color: var(--fg); font-size: 13px; }
  nav { width: 210px; padding: 14px 10px; display: flex; flex-direction: column; gap: 2px; border-right: 1px solid var(--border); }
  .brand { display: flex; align-items: center; gap: 8px; font-weight: 700; padding: 6px 10px 14px; }
  nav button { display: flex; align-items: center; gap: 10px; padding: 7px 10px; border-radius: 8px; text-align: left; color: var(--fg-2); }
  nav button:hover { background: var(--surface-hover); color: var(--fg); }
  nav button.sel { background: var(--accent); color: white; }
  main { flex: 1; overflow-y: auto; padding: 18px 26px 30px; user-select: text; }
  h1 { font: 700 20px var(--font-display); margin: 0 0 14px; }
  .group { background: var(--surface); border: 1px solid var(--border); border-radius: 12px; margin-bottom: 16px; overflow: hidden; }
  .seg { display: inline-flex; background: var(--surface); border-radius: 8px; padding: 2px; gap: 2px; }
  .seg button { padding: 4px 12px; border-radius: 6px; color: var(--fg-2); }
  .seg button.on { background: var(--accent); color: white; }
  input[type='range'] { width: 180px; accent-color: var(--accent); }
  .val { width: 48px; text-align: right; color: var(--fg-2); font-variant-numeric: tabular-nums; }
  .styles { display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px; }
  .style { display: flex; flex-direction: column; align-items: center; gap: 6px; padding: 8px; border-radius: 10px; border: 2px solid transparent; color: var(--fg-2); font-size: 12px; }
  .style.on { border-color: var(--accent); color: var(--fg); }
  .style span { width: 92px; height: 22px; border-radius: 11px; border: 1px solid var(--border); }
  .liquidGlass span { background: linear-gradient(135deg, rgba(255,255,255,.35), rgba(255,255,255,.05)); }
  .iosGlass span { background: rgba(255,255,255,.28); backdrop-filter: blur(4px); }
  .clear span { background: rgba(255,255,255,.06); }
  .black span { background: #111113; }
  .white span { background: #f4f4f6; }
  .midnight span { background: #0e1430; }
  .accents { display: flex; gap: 8px; }
  .acc { width: 22px; height: 22px; border-radius: 11px; border: 2px solid transparent; }
  .acc.on { border-color: var(--fg); }
  textarea { width: 280px; background: var(--surface); border: 1px solid var(--border); border-radius: 8px; padding: 6px 8px; color: var(--fg); resize: vertical; }
  .note { color: var(--fg-2); margin: 0 0 12px; max-width: 620px; line-height: 1.45; }
  .warn { color: var(--warn); margin: 0 0 12px; max-width: 620px; line-height: 1.45; }
  .error { color: var(--danger); }
  .actions { display: flex; justify-content: flex-end; margin: -6px 0 10px; }
  .reset { padding: 6px 12px; border-radius: 8px; background: var(--surface); border: 1px solid var(--border); font-size: 12px; }
  .reset:hover { border-color: var(--accent); }
  h2.sub { font: 600 13px var(--font); margin: 18px 0 6px; }
  .list .li { display: flex; align-items: center; justify-content: space-between; padding: 8px 14px; }
  .list .li + .li { border-top: 1px solid var(--border); }
  .list .x { width: 22px; height: 22px; border-radius: 11px; color: var(--fg-3); }
  .list .x:hover { background: var(--danger); color: white; }
  .list .add input { flex: 1; background: none; border: 0; outline: none; color: var(--fg); }
  .list .add button { padding: 4px 12px; border-radius: 7px; background: var(--accent); color: white; }
  code { font-family: Consolas, monospace; font-size: 12px; background: var(--surface); padding: 1px 5px; border-radius: 4px; }
  .quit { padding: 7px 14px; border-radius: 8px; background: var(--surface); border: 1px solid var(--border); }
  .quit:hover { color: var(--danger); }
  .notes { margin-top: 12px; padding: 14px 16px; border-radius: 12px; background: var(--surface); border: 1px solid var(--border); }
  .nh { display: flex; align-items: center; gap: 8px; margin-bottom: 6px; }
  .nh .sp { flex: 1; }
  .nh .btn.primary { background: var(--accent); color: white; }
  .md { font-size: 12.5px; line-height: 1.5; color: var(--fg-2); max-height: 360px; overflow-y: auto; }
  .md :global(h4), .md :global(h5) { margin: 10px 0 4px; font-size: 12.5px; color: var(--fg); }
  .md :global(ul) { margin: 0; padding-left: 18px; }
  .md :global(li) { margin: 2px 0; }
  .md :global(b) { color: var(--fg); }
  .md :global(p) { margin: 4px 0; }
</style>
