import { mount } from 'svelte'
import './app.css'
import CardHost from './card/CardHost.svelte'
import Dock from './dock/Dock.svelte'
import OverlayHost from './card/OverlayHost.svelte'
import Settings from './settings/Settings.svelte'

// One bundle, three surfaces: dock windows, the shared card/menu window (#card), the full-screen panel (#overlay).
const target = document.getElementById('app')!
const surfaces: Record<string, any> = { '#card': CardHost, '#overlay': OverlayHost, '#settings': Settings }
const app = mount(surfaces[location.hash] ?? Dock, { target })

export default app
