/** Max source bytes per file (under API 1 MiB limit). */
export const MAX_FILE_BYTES = 900_000
export const MAX_BATCH_FILES = 80

const SKIP_PATH_RE =
  /node_modules|[/\\]\.git[/\\]|__pycache__|\.venv|venv[/\\]|[/\\]dist[/\\]|[/\\]target[/\\]|[/\\]\.next[/\\]|[/\\]vendor[/\\]|[/\\]build[/\\]/i

const EXT_TO_LANG: Record<string, string> = {
  '.py': 'python',
  '.ts': 'typescript',
  '.tsx': 'typescript',
  '.js': 'javascript',
  '.jsx': 'javascript',
  '.mjs': 'javascript',
  '.cjs': 'javascript',
  '.java': 'java',
  '.go': 'go',
  '.tf': 'hcl',
  '.hcl': 'hcl',
  '.yaml': 'yaml',
  '.yml': 'yaml',
  '.dockerfile': 'dockerfile',
  '.json': 'json',
}

export function guessLanguageFromPath(path: string): string | null {
  const lower = path.toLowerCase()
  if (lower.endsWith('dockerfile') || lower.split(/[/\\]/).pop() === 'Dockerfile')
    return 'dockerfile'
  const dot = path.lastIndexOf('.')
  if (dot < 0) return null
  const ext = path.slice(dot).toLowerCase()
  return EXT_TO_LANG[ext] ?? null
}

export function shouldSkipPath(path: string): boolean {
  return SKIP_PATH_RE.test(path) || path.split(/[/\\]/).some((p) => p.startsWith('.'))
}

export async function readFileEntries(
  files: FileList | File[],
): Promise<{ path: string; source: string; language: string }[]> {
  const out: { path: string; source: string; language: string }[] = []
  const arr = Array.from(files)
  for (const f of arr.slice(0, MAX_BATCH_FILES)) {
    const path = (f as File & { webkitRelativePath?: string }).webkitRelativePath || f.name
    if (shouldSkipPath(path)) continue
    const lang = guessLanguageFromPath(path)
    if (!lang) continue
    if (f.size > MAX_FILE_BYTES) continue
    const text = await f.text()
    if (!text.trim()) continue
    out.push({ path, source: text, language: lang })
  }
  return out
}

export async function readZipEntries(file: File): Promise<{ path: string; source: string; language: string }[]> {
  const JSZip = (await import('jszip')).default
  const zip = await JSZip.loadAsync(await file.arrayBuffer())
  const out: { path: string; source: string; language: string }[] = []
  for (const name of Object.keys(zip.files)) {
    if (out.length >= MAX_BATCH_FILES) break
    const entry = zip.files[name]
    if (!entry || entry.dir) continue
    const path = name.replace(/^\/+/, '')
    if (shouldSkipPath(path)) continue
    const lang = guessLanguageFromPath(path)
    if (!lang) continue
    const u8 = await entry.async('uint8array')
    if (u8.length > MAX_FILE_BYTES) continue
    if (u8.slice(0, Math.min(4096, u8.length)).includes(0)) continue
    const source = new TextDecoder('utf-8', { fatal: false }).decode(u8)
    if (!source.trim()) continue
    out.push({ path, source, language: lang })
  }
  return out
}
