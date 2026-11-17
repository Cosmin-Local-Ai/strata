// Temperatures from LibreHardwareMonitor's local web server (http://localhost:8085/data.json).
// Strata never loads a GPU or sensor driver itself (no conflicts with GPU overlays or vendor hardware tools).
import { invoke } from '@tauri-apps/api/core'

export type Sensor = { id: string; name: string; hardware: string; value: number }

function walk(n: any, hw: string, out: Sensor[]) {
  const text: string = n.Text ?? ''
  const val: string = n.Value ?? ''
  if (/°C$/.test(val) && (n.Type === undefined || n.Type === 'Temperature')) {
    const v = parseFloat(val.replace(',', '.'))
    if (isFinite(v)) out.push({ id: n.SensorId ?? `${hw}/${text}`, name: text, hardware: hw, value: v })
  }
  const isHw = n.ImageURL && !/transparent/.test(n.ImageURL) && n.Children?.some((c: any) => c.Children?.length)
  for (const c of n.Children ?? []) walk(c, isHw ? text : hw, out)
}

export async function readSensors(): Promise<Sensor[]> {
  const r = await invoke<{ status: number; body: any }>('api', { service: 'public', url: 'http://localhost:8085/data.json', method: 'GET', body: null, headers: null })
    .catch(() => { throw new Error('lhm') })
  if (r.status !== 200 || typeof r.body !== 'object') throw new Error('lhm')
  const out: Sensor[] = []
  walk(r.body, '', out)
  return out
}

/** Default sensor: CPU package / Tctl, else the first CPU sensor, else the first. */
export function pickDefault(s: Sensor[]): Sensor | undefined {
  return s.find((x) => /package|tctl|tdie/i.test(x.name) && /cpu|ryzen|intel|core/i.test(x.hardware)) ?? s.find((x) => /cpu|ryzen|intel/i.test(x.hardware)) ?? s[0]
}
export const toUnit = (c: number, f: boolean) => (f ? c * 9 / 5 + 32 : c)
