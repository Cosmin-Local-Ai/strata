// Copies the public part of this project into ../strata-public (the public GitHub repo).
// Private material (docs/, ref/, spikes/, personal/) never goes there.
//   node tools/export-public.mjs        then commit and push inside ../strata-public
import fs from 'node:fs'
import path from 'node:path'

const SRC = path.resolve(import.meta.dirname, '..')
const DST = path.resolve(SRC, '..', 'strata-public')
const INCLUDE = [
  '.github', '.gitignore', '.vscode/extensions.json', 'LICENSE', 'README.md', 'CHANGELOG.md', 'index.html', 'package.json', 'package-lock.json',
  'svelte.config.js', 'tsconfig.json', 'tsconfig.app.json', 'tsconfig.node.json', 'vite.config.ts',
  'public', 'src', 'guard', 'tray', 'src-tauri', 'tools/export-public.mjs',
]
const SKIP = /(^|[\\/])(node_modules|target|dist|gen|out)([\\/]|$)|\.key$/

function copy(rel) {
  const from = path.join(SRC, rel), to = path.join(DST, rel)
  if (SKIP.test(rel) || !fs.existsSync(from)) return
  if (fs.statSync(from).isDirectory()) { for (const f of fs.readdirSync(from)) copy(path.join(rel, f)); return }
  fs.mkdirSync(path.dirname(to), { recursive: true })
  fs.copyFileSync(from, to)
}

// Mirror: remove what's gone (keep the public repo's .git).
if (fs.existsSync(DST)) for (const f of fs.readdirSync(DST)) if (f !== '.git') fs.rmSync(path.join(DST, f), { recursive: true, force: true })
fs.mkdirSync(DST, { recursive: true })
for (const r of INCLUDE) copy(r)
// Built tray DLL is a build output, not source.
fs.rmSync(path.join(DST, 'tray', 'out'), { recursive: true, force: true })
console.log(`exported to ${DST}`)
