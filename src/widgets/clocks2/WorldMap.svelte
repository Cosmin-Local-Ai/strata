<script lang="ts" module>
  // Built-in cities: [name, IANA zone, lat, lon, label side (1 right, -1 left), label dy].
  export const CITIES: [string, string, number, number, number, number][] = [
    ['Los Angeles', 'America/Los_Angeles', 34.05, -118.24, 1, 4],
    ['Denver', 'America/Denver', 39.74, -104.99, 1, 0],
    ['Chicago', 'America/Chicago', 41.88, -87.63, 1, 0],
    ['New York', 'America/New_York', 40.71, -74.0, 1, -4],
    ['Toronto', 'America/Toronto', 43.65, -79.38, -1, -10],
    ['Mexico City', 'America/Mexico_City', 19.43, -99.13, 1, 0],
    ['São Paulo', 'America/Sao_Paulo', -23.55, -46.63, 1, 0],
    ['Buenos Aires', 'America/Argentina/Buenos_Aires', -34.6, -58.38, 1, 0],
    ['London', 'Europe/London', 51.51, -0.13, 1, -6],
    ['Paris', 'Europe/Paris', 48.86, 2.35, 1, 6],
    ['Berlin', 'Europe/Berlin', 52.52, 13.4, 1, -10],
    ['Bucharest', 'Europe/Bucharest', 44.43, 26.1, 1, 4],
    ['Istanbul', 'Europe/Istanbul', 41.01, 28.98, 1, 10],
    ['Moscow', 'Europe/Moscow', 55.76, 37.62, 1, -4],
    ['Cairo', 'Africa/Cairo', 30.04, 31.24, 1, 4],
    ['Lagos', 'Africa/Lagos', 6.52, 3.38, 1, 0],
    ['Johannesburg', 'Africa/Johannesburg', -26.2, 28.05, 1, 0],
    ['Dubai', 'Asia/Dubai', 25.2, 55.27, -1, 8],
    ['Mumbai', 'Asia/Kolkata', 19.08, 72.88, 1, 12],
    ['Singapore', 'Asia/Singapore', 1.35, 103.82, 1, 0],
    ['Hong Kong', 'Asia/Hong_Kong', 22.32, 114.17, 1, 6],
    ['Shanghai', 'Asia/Shanghai', 31.23, 121.47, -1, 2],
    ['Seoul', 'Asia/Seoul', 37.57, 126.98, -1, -8],
    ['Tokyo', 'Asia/Tokyo', 35.68, 139.69, -1, -12],
    ['Sydney', 'Australia/Sydney', -33.87, 151.21, -1, 0],
    ['Auckland', 'Pacific/Auckland', -36.85, 174.76, -1, 10],
    ['Honolulu', 'Pacific/Honolulu', 21.31, -157.86, 1, 0],
  ]
  export const DEFAULT_CITIES = ['Los Angeles', 'New York', 'São Paulo', 'London', 'Paris', 'Dubai', 'Mumbai', 'Shanghai', 'Tokyo', 'Sydney']
</script>

<script lang="ts">
  // Dotted world map with the live day/night terminator and city times (redrawn once a minute).
  import { clock } from '../../lib/stores.svelte'
  import { MAP } from './worldmap'
  let { width, cities = DEFAULT_CITIES, labels = true, hour12 = false }:
    { width: number; cities?: string[]; labels?: boolean; hour12?: boolean } = $props()
  const c = clock(60_000)
  const height = $derived(Math.round(width * MAP.h / MAP.w))
  let canvas = $state<HTMLCanvasElement>()

  const bits = MAP.rows.map((r) => [...r].flatMap((h) => parseInt(h, 16).toString(2).padStart(4, '0').split('').map(Number)))
  const rad = Math.PI / 180

  function sun(d: Date) {
    const start = Date.UTC(d.getUTCFullYear(), 0, 0)
    const doy = (d.getTime() - start) / 864e5
    const decl = -23.44 * Math.cos((2 * Math.PI / 365) * (doy + 10))
    const hours = d.getUTCHours() + d.getUTCMinutes() / 60
    return { decl, lon: -15 * (hours - 12) }
  }
  const altitude = (lat: number, lon: number, s: { decl: number; lon: number }) =>
    Math.sin(lat * rad) * Math.sin(s.decl * rad) + Math.cos(lat * rad) * Math.cos(s.decl * rad) * Math.cos((lon - s.lon) * rad)

  $effect(() => {
    if (!canvas) return
    const now = c.now
    const dpr = window.devicePixelRatio || 1
    canvas.width = Math.round(width * dpr); canvas.height = Math.round(height * dpr)
    const g = canvas.getContext('2d')!
    g.scale(dpr, dpr)
    const cw = width / MAP.w, ch = height / MAP.h, dot = Math.max(1, Math.min(cw, ch) * 0.72)
    const s = sun(now)
    const css = getComputedStyle(canvas)
    const accent = css.getPropertyValue('--accent').trim() || '#0a84ff'
    for (let y = 0; y < MAP.h; y++) {
      const lat = MAP.lat0 - (y + 0.5) * (MAP.lat0 - MAP.lat1) / MAP.h
      for (let x = 0; x < MAP.w; x++) {
        if (!bits[y][x]) continue
        const lon = -180 + (x + 0.5) * 360 / MAP.w
        const a = altitude(lat, lon, s) // > 0 day; soft twilight band
        const light = Math.max(0, Math.min(1, (a + 0.1) / 0.2))
        g.fillStyle = `rgba(255,255,255,${0.16 + 0.28 * light})`
        g.fillRect(x * cw + (cw - dot) / 2, y * ch + (ch - dot) / 2, dot, dot)
      }
    }
    // Terminator line.
    g.beginPath()
    g.strokeStyle = 'rgba(255,255,255,0.22)'; g.lineWidth = 1
    const td = Math.tan(s.decl * rad) || 1e-6
    for (let px = 0; px <= width; px += 2) {
      const lon = -180 + px / width * 360
      const lat = Math.atan(-Math.cos((lon - s.lon) * rad) / td) / rad
      const py = (MAP.lat0 - lat) / (MAP.lat0 - MAP.lat1) * height
      px === 0 ? g.moveTo(px, py) : g.lineTo(px, py)
    }
    g.stroke()
    // Cities.
    const big = width > 400
    g.textBaseline = 'middle'
    for (const name of cities) {
      const ct = CITIES.find((x) => x[0] === name)
      if (!ct) continue
      const [, tz, lat, lon, side, dy] = ct
      const px = (lon + 180) / 360 * width, py = (MAP.lat0 - lat) / (MAP.lat0 - MAP.lat1) * height
      g.fillStyle = accent
      g.fillRect(px - 2, py - 2, big ? 4 : 3, big ? 4 : 3)
      if (!labels) continue
      const t = now.toLocaleTimeString('en-GB', { timeZone: tz, hour: '2-digit', minute: '2-digit', hour12 })
      g.font = `600 ${big ? 13 : 10}px ${css.getPropertyValue('--font-display') || 'sans-serif'}`
      const tw = g.measureText(t + ' ').width
      g.font = `700 ${big ? 13 : 10}px ${css.getPropertyValue('--font-display') || 'sans-serif'}`
      const nw = g.measureText(name).width
      const right = side > 0 && px + 6 + tw + nw < width
      const lx = right ? px + 6 : px - 6 - tw - nw
      const ly = py + (big ? dy : dy / 2)
      g.font = `600 ${big ? 13 : 10}px ${css.getPropertyValue('--font-display') || 'sans-serif'}`
      g.fillStyle = 'rgba(255,255,255,0.55)'; g.fillText(t, lx, ly)
      g.font = `700 ${big ? 13 : 10}px ${css.getPropertyValue('--font-display') || 'sans-serif'}`
      g.fillStyle = 'rgba(255,255,255,0.95)'; g.fillText(name, lx + tw, ly)
    }
  })
</script>

<canvas bind:this={canvas} style="width:{width}px; height:{height}px"></canvas>

<style>
  canvas { display: block; }
</style>
