import { mount } from 'svelte'
import './app.css'
import CardHost from './card/CardHost.svelte'
import Dock from './dock/Dock.svelte'
import OverlayHost from './card/OverlayHost.svelte'
import Settings from './settings/Settings.svelte'
import Toasts from './notices/Toasts.svelte'
import { invoke } from '@tauri-apps/api/core'

// Uncaught errors go to Strata.log, so a broken widget can be found from the log alone.
const report = (m: string) => { try { invoke('ui_error', { message: m }) } catch { /* not in Tauri */ } }
window.addEventListener('error', (e) => report(`${e.message} at ${e.filename}:${e.lineno}:${e.colno}${e.error?.stack ? `
${e.error.stack}` : ''}`))
window.addEventListener('unhandledrejection', (e) => { const r: any = e.reason; if (r?.stack || r instanceof Error) report(`unhandled: ${r?.stack ?? r}`) })

// One bundle, three surfaces: dock windows, the shared card/menu window (#card), the full-screen panel (#overlay).
const target = document.getElementById('app')!
const surfaces: Record<string, any> = { '#card': CardHost, '#overlay': OverlayHost, '#settings': Settings, '#toast': Toasts }
let app: any
try { app = mount(surfaces[location.hash] ?? Dock, { target }) } catch (e: any) { report(`mount failed: ${e?.stack ?? e}`); throw e }

export default app
