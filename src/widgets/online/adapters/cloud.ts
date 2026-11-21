// Cloud storage: Google Drive (the Google sign-in shared with GA4 / Gmail, read-only file list) and
// Digi Storage (RCS & RDS, built on Koofr: email + an app password, HTTP Basic through Strata).
import { Cloud, HardDrive } from '@lucide/svelte'
import { invoke } from '@tauri-apps/api/core'
import { ago, api, NotConnected, type Adapter, type Row } from '../../../lib/services.svelte'

const size = (b: number) => b >= 1e12 ? `${(b / 1e12).toFixed(1)} TB` : b >= 1e9 ? `${(b / 1e9).toFixed(1)} GB` : b >= 1e6 ? `${(b / 1e6).toFixed(0)} MB` : `${Math.max(1, Math.round(b / 1e3))} KB`

// ---- Google Drive
const DRIVE_TABS = [{ id: 'recent', label: 'Recent' }, { id: 'starred', label: 'Starred' }, { id: 'shared', label: 'Shared with me' }]
const kindOf = (mime: string) => /folder/.test(mime) ? 'Folder' : /document/.test(mime) ? 'Doc' : /spreadsheet/.test(mime) ? 'Sheet' : /presentation/.test(mime) ? 'Slides' : /form/.test(mime) ? 'Form' : /pdf/.test(mime) ? 'PDF' : /image/.test(mime) ? 'Image' : /video/.test(mime) ? 'Video' : 'File'

const drive: Adapter = {
  title: 'Google Drive', icon: HardDrive, signIn: 'google', every: 5 * 60_000, tabs: DRIVE_TABS, search: true,
  link: 'https://console.cloud.google.com/apis/library/drive.googleapis.com',
  help: 'Uses the same Google sign-in as Gmail and Analytics. In your Google Cloud project, also enable the "Google Drive API". If you signed in before Drive was added, sign in again so Strata may read your file list (read-only: names and dates, never file contents).',
  async fetch(_o, c) {
    if (!await invoke<boolean>('google_status')) throw new NotConnected()
    const D = 'https://www.googleapis.com/drive/v3'
    const q = ['trashed = false', c.tab === 'starred' ? 'starred = true' : c.tab === 'shared' ? 'sharedWithMe = true' : '', c.query ? `name contains '${c.query.replace(/['\\]/g, '\\$&')}'` : ''].filter(Boolean).join(' and ')
    const order = c.tab === 'recent' || !c.tab ? 'viewedByMeTime desc' : 'modifiedTime desc'
    let about: any, files: any
    try {
      ;[about, files] = await Promise.all([
        api<any>('google', `${D}/about?fields=storageQuota,user(emailAddress)`),
        api<any>('google', `${D}/files?pageSize=15&orderBy=${encodeURIComponent(order)}&q=${encodeURIComponent(q)}&fields=${encodeURIComponent('files(id,name,mimeType,modifiedTime,viewedByMeTime,webViewLink,size)')}`),
      ])
    } catch (e) {
      const m = String((e as Error).message)
      if (/403|insufficient|scope/i.test(m)) throw new Error('Google hasn\'t allowed Drive yet: enable the Google Drive API in your Cloud project, then disconnect and sign in again here.')
      throw e
    }
    const used = +(about.storageQuota?.usage ?? 0), limit = +(about.storageQuota?.limit ?? 0)
    const rows: Row[] = (files.files ?? []).map((f: any) => ({
      title: f.name, sub: `${kindOf(f.mimeType)} · ${ago(Date.parse(f.viewedByMeTime ?? f.modifiedTime))}${f.size ? ` · ${size(+f.size)}` : ''}`, href: f.webViewLink,
    }))
    return {
      value: limit ? `${Math.round((100 * used) / limit)}%` : size(used), label: limit ? `${size(used)} of ${size(limit)}` : 'used',
      ring: limit ? used / limit : null, rows, rowsTitle: DRIVE_TABS.find((t) => t.id === (c.tab ?? 'recent'))?.label,
      stats: [{ label: 'Drive', value: size(+(about.storageQuota?.usageInDrive ?? 0)) }, { label: 'Gmail & Photos', value: size(Math.max(0, used - +(about.storageQuota?.usageInDrive ?? 0) - +(about.storageQuota?.usageInDriveTrash ?? 0))) }, { label: 'Trash', value: size(+(about.storageQuota?.usageInDriveTrash ?? 0)) }],
      open: 'https://drive.google.com/drive/home', note: about.user?.emailAddress,
    }
  },
}

// ---- Digi Storage (Koofr API)
const DIGI = 'https://storage.rcs-rds.ro'
const digi: Adapter = {
  title: 'Digi Storage', icon: Cloud, secret: 'digistorage', every: 10 * 60_000, keyLabel: 'App password',
  fields: [{ key: 'email', label: 'Email you sign in with', placeholder: 'you@example.com' }],
  tabs: [{ id: 'recent', label: 'Recently changed' }, { id: 'files', label: 'Files' }],
  link: `${DIGI}/login?redirect=%2Fapp%2F`,
  help: 'Digi Storage → your name (top right) → Preferences → Password → App passwords: generate one named "Strata" and paste it here with your email. App passwords can be revoked any time and never replace your real password.',
  async fetch(o, c) {
    if (!o.email) throw new NotConnected()
    const h = { 'x-strata-user': String(o.email).trim() }
    const call = (path: string) => api<any>('digistorage', `${DIGI}/api/v2/${path}`, { headers: h }).catch((e) => {
      if (/40[13]/.test(String((e as Error).message))) throw new Error('Digi Storage refused the email / app password. Generate a new app password and paste it again.')
      throw e
    })
    const mounts: any[] = (await call('mounts')).mounts ?? []
    const m = mounts.find((x) => x.isPrimary) ?? mounts[0]
    if (!m) throw new Error('No storage found on this account')
    const MiB = 1024 * 1024
    const used = +(m.spaceUsed ?? 0) * MiB, total = +(m.spaceTotal ?? 0) * MiB
    const list: any[] = (await call(`mounts/${m.id}/files/list?path=${encodeURIComponent('/')}`)).files ?? []
    const shown = c.tab === 'files'
      ? [...list].sort((a, b) => (a.type === 'dir' ? 0 : 1) - (b.type === 'dir' ? 0 : 1) || a.name.localeCompare(b.name))
      : [...list].filter((f) => f.type !== 'dir').sort((a, b) => b.modified - a.modified)
    const rows: Row[] = shown.slice(0, 15).map((f) => ({
      title: f.name, sub: f.type === 'dir' ? 'Folder' : `${ago(+f.modified)} · ${size(+f.size)}`,
      href: `${DIGI}/app/storage/${m.id}?path=${encodeURIComponent(f.type === 'dir' ? `/${f.name}` : '/')}`,
    }))
    return {
      value: total ? `${Math.round((100 * used) / total)}%` : size(used), label: total ? `${size(used)} of ${size(total)}` : 'used',
      ring: total ? used / total : null, rows, rowsTitle: c.tab === 'files' ? 'Top folder' : 'Recently changed (top folder)',
      stats: [{ label: 'Used', value: size(used) }, { label: 'Free', value: total ? size(Math.max(0, total - used)) : '–' }, { label: 'Places', value: String(mounts.length) }],
      open: `${DIGI}/app/storage/${m.id}`, note: String(o.email),
    }
  },
}

export const CLOUD: Record<string, Adapter> = { 'google-drive': drive, 'digi-storage': digi }
