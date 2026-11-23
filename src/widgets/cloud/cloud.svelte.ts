// Cloud Storage: one summary per account (Google Drive, Digi Storage) shared by the tile sizes and
// the card. Read-only, through Strata's service gateway; cached in localStorage for the other window.
import { invoke } from '@tauri-apps/api/core'
import { api, NotConnected } from '../../lib/services.svelte'

export type Recent = { name: string; when: number; href: string }
export type Account = {
  id: 'drive' | 'digi'
  name: string
  state: 'ok' | 'connect' | 'error'
  error?: string
  /** A page that fixes the error (e.g. where to turn the Drive API on). */
  fix?: { label: string; href: string }
  email?: string
  used: number
  total: number
  breakdown?: { label: string; bytes: number }[]
  recent: Recent[]
  open: string
}

export const DIGI = 'https://storage.rcs-rds.ro'
export const size = (b: number) => b >= 1e12 ? `${(b / 1e12).toFixed(1)} TB` : b >= 1e9 ? `${(b / 1e9).toFixed(1)} GB` : b >= 1e6 ? `${Math.round(b / 1e6)} MB` : `${Math.max(0, Math.round(b / 1e3))} KB`

async function drive(): Promise<Account> {
  const base: Account = { id: 'drive', name: 'Google Drive', state: 'connect', used: 0, total: 0, recent: [], open: 'https://drive.google.com/drive/home' }
  if (!await invoke<boolean>('google_status')) return base
  const D = 'https://www.googleapis.com/drive/v3'
  try {
    const [about, files] = await Promise.all([
      api<any>('google', `${D}/about?fields=storageQuota,user(emailAddress)`),
      api<any>('google', `${D}/files?pageSize=5&orderBy=${encodeURIComponent('viewedByMeTime desc')}&q=${encodeURIComponent('trashed = false')}&fields=${encodeURIComponent('files(name,viewedByMeTime,modifiedTime,webViewLink)')}`),
    ])
    const q = about.storageQuota ?? {}
    const used = +(q.usage ?? 0), inDrive = +(q.usageInDrive ?? 0), trash = +(q.usageInDriveTrash ?? 0)
    return {
      ...base, state: 'ok', email: about.user?.emailAddress, used, total: +(q.limit ?? 0),
      breakdown: [{ label: 'Drive', bytes: inDrive }, { label: 'Gmail & Photos', bytes: Math.max(0, used - inDrive - trash) }, { label: 'Trash', bytes: trash }],
      recent: (files.files ?? []).map((f: any) => ({ name: f.name, when: Date.parse(f.viewedByMeTime ?? f.modifiedTime), href: f.webViewLink })),
    }
  } catch (e) {
    const m = String((e as Error).message ?? e)
    // Signed in, but the Drive API is off in your Google Cloud project: signing in again won't help.
    if (/has not been used|is disabled|SERVICE_DISABLED|accessNotConfigured/i.test(m))
      return { ...base, state: 'error', error: 'Signed in. Turn on the Google Drive API in your Google Cloud project, then refresh.',
               fix: { label: 'Turn on the Drive API', href: 'https://console.cloud.google.com/apis/library/drive.googleapis.com' } }
    const again = /\(401\)|insufficient|scope/i.test(m)
    return { ...base, state: again ? 'connect' : 'error', error: again ? 'Sign in to Google again to allow Drive (read-only).' : m }
  }
}

async function digi(email: string): Promise<Account> {
  const base: Account = { id: 'digi', name: 'Digi Storage', state: 'connect', used: 0, total: 0, recent: [], open: `${DIGI}/app/` }
  if (!email || !await invoke<boolean>('secret_has', { name: 'digistorage' })) return base
  const h = { 'x-strata-user': email }
  try {
    const mounts: any[] = (await api<any>('digistorage', `${DIGI}/api/v2/mounts`, { headers: h })).mounts ?? []
    const m = mounts.find((x) => x.isPrimary) ?? mounts[0]
    if (!m) return { ...base, state: 'error', error: 'No storage found on this account' }
    const MiB = 1024 * 1024
    const list: any[] = (await api<any>('digistorage', `${DIGI}/api/v2/mounts/${m.id}/files/list?path=${encodeURIComponent('/')}`, { headers: h })).files ?? []
    return {
      ...base, state: 'ok', email, used: +(m.spaceUsed ?? 0) * MiB, total: +(m.spaceTotal ?? 0) * MiB, open: `${DIGI}/app/storage/${m.id}`,
      recent: list.filter((f) => f.type !== 'dir').sort((a, b) => b.modified - a.modified).slice(0, 5)
        .map((f) => ({ name: f.name, when: +f.modified, href: `${DIGI}/app/storage/${m.id}?path=%2F` })),
    }
  } catch (e) {
    if (e instanceof NotConnected) return base
    const m = String((e as Error).message ?? e)
    return { ...base, state: /40[13]/.test(m) ? 'connect' : 'error', error: /40[13]/.test(m) ? 'Digi Storage refused the email / app password.' : m }
  }
}

const KEY = 'cloud:summary'
type Cache = { at: number; accounts: Account[] }
const box = $state<{ accounts: Account[]; loading: boolean; at: number }>((() => {
  try { const c: Cache = JSON.parse(localStorage.getItem(KEY) ?? 'null'); if (c) return { accounts: c.accounts, loading: false, at: c.at } } catch { /* fresh */ }
  return { accounts: [], loading: false, at: 0 }
})())
window.addEventListener('storage', (e) => { if (e.key === KEY && e.newValue) { const c: Cache = JSON.parse(e.newValue); box.accounts = c.accounts; box.at = c.at } })

export async function refresh(ids: string[], digiEmail: string) {
  box.loading = true
  try {
    const out = await Promise.all(ids.map((id) => id === 'drive' ? drive() : digi(digiEmail)))
    box.accounts = out; box.at = Date.now()
    localStorage.setItem(KEY, JSON.stringify({ at: box.at, accounts: out }))
  } finally { box.loading = false }
}
/** Refreshes when the cached summary is older than 10 minutes (or `force`). */
export function cloud(ids: () => string[], digiEmail: () => string) {
  $effect(() => {
    const i = ids(), e = digiEmail()
    // Not connected yet (or failing): check again on every open, so a sign-in elsewhere shows up at once.
    const stale = Date.now() - box.at > 10 * 60_000 || box.accounts.map((a) => a.id).join() !== i.join() || box.accounts.some((a) => a.state !== 'ok')
    if (stale) refresh(i, e)
    const t = setInterval(() => refresh(i, e), 10 * 60_000)
    return () => clearInterval(t)
  })
  return box
}
export const pct = (a: Account) => a.total ? Math.min(1, a.used / a.total) : 0
