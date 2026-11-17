// Weather via Open-Meteo (no key). Refresh every 15 min.
import { Cloud, CloudDrizzle, CloudFog, CloudLightning, CloudMoon, CloudRain, CloudSnow, CloudSun, Moon, Sun } from '@lucide/svelte'
import { poll } from '../../lib/poll'

export type Place = { name: string; lat: number; lon: number }
/** Until a city is picked in the weather card. */
export const DEFAULT_PLACE: Place = { name: 'London', lat: 51.5072, lon: -0.1276 }

export type Weather = {
  current: { temp: number; feels: number; code: number; day: boolean; humidity: number; wind: number }
  hourly: { time: string; temp: number; code: number; pop: number }[]
  daily: { date: string; max: number; min: number; code: number; sunrise: string; sunset: string }[]
}

async function fetchWeather(p: Place): Promise<Weather> {
  const u = new URL('https://api.open-meteo.com/v1/forecast')
  u.search = new URLSearchParams({
    latitude: String(p.lat), longitude: String(p.lon), timezone: 'auto', forecast_days: '7',
    current: 'temperature_2m,apparent_temperature,weather_code,is_day,relative_humidity_2m,wind_speed_10m',
    hourly: 'temperature_2m,weather_code,precipitation_probability',
    daily: 'weather_code,temperature_2m_max,temperature_2m_min,sunrise,sunset',
  }).toString()
  const r = await fetch(u)
  if (!r.ok) throw new Error(`Weather service answered ${r.status}`)
  const j = await r.json()
  const now = Date.now()
  const hourly = (j.hourly.time as string[])
    .map((t, i) => ({ time: t, temp: j.hourly.temperature_2m[i], code: j.hourly.weather_code[i], pop: j.hourly.precipitation_probability[i] ?? 0 }))
    .filter((h) => Date.parse(h.time) >= now - 3_600_000)
    .slice(0, 12)
  return {
    current: {
      temp: j.current.temperature_2m, feels: j.current.apparent_temperature, code: j.current.weather_code,
      day: j.current.is_day === 1, humidity: j.current.relative_humidity_2m, wind: j.current.wind_speed_10m,
    },
    hourly,
    daily: (j.daily.time as string[]).map((d, i) => ({
      date: d, max: j.daily.temperature_2m_max[i], min: j.daily.temperature_2m_min[i], code: j.daily.weather_code[i],
      sunrise: j.daily.sunrise[i], sunset: j.daily.sunset[i],
    })),
  }
}

export function watchWeather(p: Place, cb: (w: Weather | null, err: string | null) => void) {
  return poll(`weather:${p.lat},${p.lon}`, 15 * 60_000, () => fetchWeather(p), cb)
}

/** WMO weather code → words + icon. */
export function describe(code: number, day = true): { text: string; icon: any } {
  if (code === 0) return { text: 'Clear', icon: day ? Sun : Moon }
  if (code <= 2) return { text: 'Partly cloudy', icon: day ? CloudSun : CloudMoon }
  if (code === 3) return { text: 'Overcast', icon: Cloud }
  if (code === 45 || code === 48) return { text: 'Fog', icon: CloudFog }
  if (code >= 51 && code <= 57) return { text: 'Drizzle', icon: CloudDrizzle }
  if ((code >= 61 && code <= 67) || (code >= 80 && code <= 82)) return { text: 'Rain', icon: CloudRain }
  if ((code >= 71 && code <= 77) || code === 85 || code === 86) return { text: 'Snow', icon: CloudSnow }
  if (code >= 95) return { text: 'Thunderstorm', icon: CloudLightning }
  return { text: 'Cloudy', icon: Cloud }
}

export async function searchPlaces(q: string): Promise<Place[]> {
  const u = `https://geocoding-api.open-meteo.com/v1/search?count=6&language=en&name=${encodeURIComponent(q)}`
  const j = await (await fetch(u)).json()
  return (j.results ?? []).map((r: any) => ({ name: [r.name, r.admin1, r.country].filter(Boolean).join(', '), lat: r.latitude, lon: r.longitude }))
}
