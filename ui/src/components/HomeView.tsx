import { useCallback, useEffect, useRef, useState } from 'react'
import {
  Zap,
  Upload,
  FolderOpen,
  FileArchive,
  FileCode,
  ArrowRight,
  Loader2,
  Github,
} from 'lucide-react'
import clsx from 'clsx'
import { useStore } from '../store'
import { AWS_EXAMPLES, AZURE_EXAMPLES, type CloudExample } from '../content/examples'
import { readFileEntries, readZipEntries, MAX_BATCH_FILES } from '../fileImport'
import { importGithubRepo } from '../api'
import { runHomeTour } from '../tour/cloudshiftTour'

const TOUR_BANNER_KEY = 'cloudshift_tour_banner_dismissed_v1'

const CLOUDS = [
  { value: 'aws', label: 'AWS' },
  { value: 'azure', label: 'Azure' },
  { value: 'any', label: 'Auto-detect' },
]

export default function HomeView() {
  const loadSnippet = useStore((s) => s.loadSnippet)
  const loadBatch = useStore((s) => s.loadBatch)
  const enterSnippetWorkspace = useStore((s) => s.enterSnippetWorkspace)
  const apiKey = useStore((s) => s.apiKey)

  const [paste, setPaste] = useState('')
  const [pasteLang, setPasteLang] = useState('python')
  const [pasteCloud, setPasteCloud] = useState('aws')
  const [importMsg, setImportMsg] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const fileRef = useRef<HTMLInputElement>(null)
  const folderRef = useRef<HTMLInputElement>(null)
  const zipRef = useRef<HTMLInputElement>(null)
  const [examplePicker, setExamplePicker] = useState('')
  const [githubUrl, setGithubUrl] = useState('')
  const [githubRef, setGithubRef] = useState('')
  const [showTourBanner, setShowTourBanner] = useState(false)

  useEffect(() => {
    try {
      setShowTourBanner(!localStorage.getItem(TOUR_BANNER_KEY))
    } catch {
      setShowTourBanner(false)
    }
  }, [])

  const dismissTourBanner = () => {
    try {
      localStorage.setItem(TOUR_BANNER_KEY, '1')
    } catch {
      /* ignore */
    }
    setShowTourBanner(false)
  }

  const openSnippet = useCallback(() => {
    enterSnippetWorkspace()
    loadSnippet('', 'python', 'aws', '')
  }, [enterSnippetWorkspace, loadSnippet])

  const openPaste = useCallback(() => {
    if (!paste.trim()) {
      setImportMsg('Paste some code first.')
      return
    }
    loadSnippet(paste, pasteLang, pasteCloud, 'snippet.py')
    setImportMsg(null)
  }, [paste, pasteLang, pasteCloud, loadSnippet])

  const loadExample = useCallback(
    (ex: CloudExample) => {
      loadSnippet(ex.code, ex.language, ex.cloud, `example_${ex.id}.py`)
    },
    [loadSnippet],
  )

  const loadGithubRepo = useCallback(async () => {
    const u = githubUrl.trim()
    if (!u) {
      setImportMsg('Enter a GitHub repository URL.')
      return
    }
    setBusy(true)
    setImportMsg(null)
    try {
      const data = await importGithubRepo(u, {
        ref: githubRef.trim() || undefined,
        apiKey: apiKey || undefined,
      })
      if (data.error && (!data.files || data.files.length === 0)) {
        setImportMsg(data.error)
        return
      }
      if (!data.files?.length) {
        setImportMsg(data.error || 'No supported files in this repository.')
        return
      }
      const refNote = data.resolved_ref ? `@${data.resolved_ref}` : ''
      const trunc = data.truncated ? ' (first 80 files only)' : ''
      if (data.files.length === 1) {
        const f = data.files[0]
        loadSnippet(f.source, f.language, 'any', f.path)
        setImportMsg(`Opened ${f.path} from GitHub${refNote}${trunc}`)
      } else {
        loadBatch(
          data.files.map((f) => ({ path: f.path, source: f.source, language: f.language })),
          'any',
        )
        setImportMsg(`Loaded ${data.files.length} files from GitHub${refNote}${trunc}`)
      }
      if (data.error && data.files.length > 0) {
        setImportMsg((m) => `${m} — ${data.error}`)
      }
    } catch (e) {
      setImportMsg(e instanceof Error ? e.message : 'GitHub import failed')
    } finally {
      setBusy(false)
    }
  }, [githubUrl, githubRef, apiKey, loadSnippet, loadBatch])

  const handleFiles = useCallback(
    async (list: FileList | null) => {
      if (!list?.length) return
      setBusy(true)
      setImportMsg(null)
      try {
        const entries = await readFileEntries(list)
        if (entries.length === 0) {
          setImportMsg('No supported code files found (Python, TS, JS, Java, Go, HCL, YAML, JSON, Dockerfile).')
          return
        }
        if (entries.length === 1) {
          const e = entries[0]
          loadSnippet(e.source, e.language, 'aws', e.path)
          setImportMsg(`Opened: ${e.path}`)
        } else {
          loadBatch(entries, 'aws')
          setImportMsg(`${entries.length} files (max ${MAX_BATCH_FILES}). Transform each or run batch.`)
        }
      } finally {
        setBusy(false)
      }
    },
    [loadSnippet, loadBatch],
  )

  const handleZip = useCallback(
    async (f: File | null) => {
      if (!f) return
      setBusy(true)
      setImportMsg(null)
      try {
        const entries = await readZipEntries(f)
        if (entries.length === 0) {
          setImportMsg('ZIP had no supported code files.')
          return
        }
        if (entries.length === 1) {
          const e = entries[0]
          loadSnippet(e.source, e.language, 'aws', e.path)
        } else {
          loadBatch(entries, 'aws')
          setImportMsg(`${entries.length} files from archive.`)
        }
      } catch {
        setImportMsg('Could not read ZIP.')
      } finally {
        setBusy(false)
      }
    },
    [loadSnippet, loadBatch],
  )

  const onDrop = useCallback(
    async (e: React.DragEvent) => {
      e.preventDefault()
      const dt = e.dataTransfer
      if (!dt.files?.length) return
      const files = Array.from(dt.files)
      const zip = files.find((x) => x.name.toLowerCase().endsWith('.zip'))
      if (files.length === 1 && zip) {
        await handleZip(zip)
        return
      }
      await handleFiles(dt.files)
    },
    [handleFiles, handleZip],
  )

  return (
    <div
      className="flex-1 overflow-y-auto"
      onDragOver={(e) => e.preventDefault()}
      onDrop={onDrop}
    >
      <div className="max-w-3xl mx-auto px-4 py-10 space-y-10">
        {showTourBanner && (
          <div
            id="tour-home-banner"
            className="flex flex-col sm:flex-row sm:items-center gap-3 rounded-xl border border-blue-500/25 bg-blue-500/5 px-4 py-3"
          >
            <p className="text-sm text-zinc-300 flex-1">
              <span className="font-medium text-blue-400">New here?</span> Take a quick guided tour of the home
              menu and how to load code.
            </p>
            <div className="flex items-center gap-2 shrink-0">
              <button
                type="button"
                onClick={() => {
                  dismissTourBanner()
                  runHomeTour()
                }}
                className="h-8 px-3 text-xs font-semibold rounded-md bg-blue-600 hover:bg-blue-500 text-white"
              >
                Start tour
              </button>
              <button
                type="button"
                onClick={dismissTourBanner}
                className="h-8 px-3 text-xs text-zinc-500 hover:text-zinc-300"
              >
                Dismiss
              </button>
            </div>
          </div>
        )}

        <div id="tour-home-intro" className="text-center space-y-3">
          <div className="w-14 h-14 rounded-2xl bg-gradient-to-br from-blue-500/20 to-violet-500/20 border border-blue-500/15 flex items-center justify-center mx-auto">
            <Zap className="w-7 h-7 text-blue-400" />
          </div>
          <h1 className="text-xl font-semibold text-zinc-100">CloudShift</h1>
          <p className="text-sm text-zinc-500 max-w-md mx-auto">
            Transform AWS/Azure code to GCP. Paste code, upload files, open a GitHub repo, or use a ZIP.
          </p>
        </div>

        {importMsg && (
          <p
            className={clsx(
              'text-center text-sm px-4 py-2 rounded-lg border',
              importMsg.startsWith('Opened') ||
              importMsg.startsWith('Loaded') ||
              importMsg.includes('files from GitHub')
                ? 'bg-emerald-500/10 border-emerald-500/20 text-emerald-400/90'
                : 'bg-amber-500/10 border-amber-500/20 text-amber-400/90',
            )}
          >
            {importMsg}
          </p>
        )}

        <div id="tour-home-imports" className="space-y-2">
        {/* Quick actions */}
        <div className="grid sm:grid-cols-2 gap-3">
          <button
            type="button"
            onClick={openSnippet}
            disabled={busy}
            className="flex items-center gap-3 p-4 rounded-xl border border-[#222228] bg-[#111114] hover:bg-[#141418] hover:border-[#333340] text-left transition-all"
          >
            <FileCode className="w-8 h-8 text-blue-400 shrink-0" />
            <div>
              <div className="font-medium text-zinc-200">Empty editor</div>
              <div className="text-xs text-zinc-600 mt-0.5">Type or paste in the workspace</div>
            </div>
            <ArrowRight className="w-4 h-4 text-zinc-600 ml-auto shrink-0" />
          </button>

          <label className="flex items-center gap-3 p-4 rounded-xl border border-[#222228] bg-[#111114] hover:bg-[#141418] hover:border-[#333340] cursor-pointer transition-all">
            <Upload className="w-8 h-8 text-violet-400 shrink-0" />
            <div>
              <div className="font-medium text-zinc-200">Upload file(s)</div>
              <div className="text-xs text-zinc-600 mt-0.5">One file opens editor; several = batch</div>
            </div>
            <input
              ref={fileRef}
              type="file"
              multiple
              className="hidden"
              onChange={(e) => {
                handleFiles(e.target.files)
                e.target.value = ''
              }}
            />
            {busy ? <Loader2 className="w-4 h-4 animate-spin text-zinc-500" /> : null}
          </label>

          <label className="flex items-center gap-3 p-4 rounded-xl border border-[#222228] bg-[#111114] hover:bg-[#141418] hover:border-[#333340] cursor-pointer transition-all">
            <FolderOpen className="w-8 h-8 text-amber-400 shrink-0" />
            <div>
              <div className="font-medium text-zinc-200">Upload folder</div>
              <div className="text-xs text-zinc-600 mt-0.5">Whole repo tree (supported extensions)</div>
            </div>
            <input
              ref={folderRef}
              type="file"
              multiple
              {...({ webkitdirectory: '', directory: '' } as React.InputHTMLAttributes<HTMLInputElement>)}
              className="hidden"
              onChange={(e) => {
                handleFiles(e.target.files)
                e.target.value = ''
              }}
            />
          </label>

          <label className="flex items-center gap-3 p-4 rounded-xl border border-[#222228] bg-[#111114] hover:bg-[#141418] hover:border-[#333340] cursor-pointer transition-all">
            <FileArchive className="w-8 h-8 text-emerald-400 shrink-0" />
            <div>
              <div className="font-medium text-zinc-200">Upload ZIP</div>
              <div className="text-xs text-zinc-600 mt-0.5">Exported repo archive</div>
            </div>
            <input
              ref={zipRef}
              type="file"
              accept=".zip,application/zip"
              className="hidden"
              onChange={(e) => {
                handleZip(e.target.files?.[0] ?? null)
                e.target.value = ''
              }}
            />
          </label>
        </div>

        <p className="text-center text-[11px] text-zinc-600">
          Drag and drop files or a .zip here · Up to {MAX_BATCH_FILES} files · ~900KB per file
        </p>
        </div>

        {/* GitHub repo */}
        <div id="tour-home-github" className="rounded-xl border border-[#222228] bg-[#0c0c0f] p-4 space-y-3">
          <div className="flex items-center gap-2">
            <Github className="w-4 h-4 text-zinc-500" />
            <h2 className="text-[11px] font-medium text-zinc-500 uppercase tracking-wider">
              GitHub repository
            </h2>
          </div>
          <p className="text-xs text-zinc-600 leading-relaxed">
            Public repos download on the server (max ~25&nbsp;MB archive). Optional branch or tag. Private
            repos need <code className="text-zinc-500">GITHUB_TOKEN</code> on the server.
          </p>
          <input
            type="url"
            value={githubUrl}
            onChange={(e) => setGithubUrl(e.target.value)}
            placeholder="https://github.com/owner/repo"
            className="w-full h-10 px-3 text-sm bg-[#111114] border border-[#27272a] rounded-lg text-zinc-200 placeholder:text-zinc-700 outline-none focus:border-blue-500/40"
          />
          <div className="flex flex-wrap items-center gap-2">
            <input
              type="text"
              value={githubRef}
              onChange={(e) => setGithubRef(e.target.value)}
              placeholder="Branch or tag (optional — default branch if empty)"
              className="flex-1 min-w-[200px] h-9 px-3 text-xs bg-[#111114] border border-[#27272a] rounded-lg text-zinc-300 placeholder:text-zinc-700 outline-none focus:border-blue-500/40"
            />
            <button
              type="button"
              onClick={() => void loadGithubRepo()}
              disabled={busy || !githubUrl.trim()}
              className="h-9 px-4 text-xs font-semibold rounded-lg bg-zinc-700 hover:bg-zinc-600 text-white disabled:opacity-40 flex items-center gap-2 shrink-0"
            >
              {busy ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <Github className="w-3.5 h-3.5" />}
              Load repo
            </button>
          </div>
        </div>

        {/* Paste snippet */}
        <div id="tour-home-paste" className="rounded-xl border border-[#222228] bg-[#0c0c0f] p-4 space-y-3">
          <h2 className="text-[11px] font-medium text-zinc-500 uppercase tracking-wider">Paste code</h2>
          <textarea
            value={paste}
            onChange={(e) => setPaste(e.target.value)}
            placeholder="Paste a code snippet…"
            rows={6}
            className="w-full rounded-lg bg-[#111114] border border-[#27272a] px-3 py-2 text-sm text-zinc-300 placeholder:text-zinc-700 font-mono resize-y min-h-[120px] outline-none focus:border-blue-500/40"
          />
          <div className="flex flex-wrap items-center gap-2">
            <select
              value={pasteLang}
              onChange={(e) => setPasteLang(e.target.value)}
              className="h-8 px-2 text-xs bg-[#111114] border border-[#27272a] rounded text-zinc-300"
            >
              <option value="python">Python</option>
              <option value="typescript">TypeScript</option>
              <option value="javascript">JavaScript</option>
              <option value="java">Java</option>
              <option value="go">Go</option>
              <option value="hcl">HCL</option>
              <option value="yaml">YAML</option>
              <option value="json">JSON</option>
              <option value="dockerfile">Dockerfile</option>
            </select>
            <select
              value={pasteCloud}
              onChange={(e) => setPasteCloud(e.target.value)}
              className="h-8 px-2 text-xs bg-[#111114] border border-[#27272a] rounded text-zinc-300"
            >
              {CLOUDS.map((c) => (
                <option key={c.value} value={c.value}>
                  {c.label}
                </option>
              ))}
            </select>
            <button
              type="button"
              onClick={openPaste}
              disabled={busy || !paste.trim()}
              className="h-8 px-4 text-xs font-semibold rounded-md bg-blue-600 hover:bg-blue-500 text-white disabled:opacity-40"
            >
              Open in editor
            </button>
          </div>
        </div>

        {/* Service examples — AWS & Azure */}
        <div id="tour-home-examples" className="rounded-xl border border-[#222228] bg-[#0c0c0f] p-4 space-y-3">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <h2 className="text-[11px] font-medium text-zinc-500 uppercase tracking-wider">
              Service examples
            </h2>
            <span className="text-[10px] text-zinc-500 tabular-nums">
              <span className="text-orange-400/90">{AWS_EXAMPLES.length} AWS</span>
              <span className="text-zinc-600 mx-1">·</span>
              <span className="text-sky-400/90">{AZURE_EXAMPLES.length} Azure</span>
            </span>
          </div>
          <p className="text-xs text-zinc-600 leading-relaxed">
            Samples are <strong className="text-zinc-400">AWS or Azure source code</strong> (what you migrate{' '}
            <em>from</em>) — not GCP. Run <strong className="text-zinc-400">Transform</strong> to generate GCP
            equivalents (e.g. Azure Blob → <code className="text-zinc-500">google.cloud.storage</code>, S3 →
            Cloud Storage).
          </p>
          <select
            value={examplePicker}
            onChange={(e) => {
              const id = e.target.value
              if (!id) return
              const ex =
                AWS_EXAMPLES.find((x) => x.id === id) ?? AZURE_EXAMPLES.find((x) => x.id === id)
              if (ex) {
                loadExample(ex)
                setImportMsg(`Opened: ${ex.title}`)
              }
              setExamplePicker('')
            }}
            className="w-full h-11 px-3 text-sm bg-[#111114] border border-[#27272a] rounded-lg text-zinc-200 outline-none focus:border-blue-500/40 focus:ring-1 focus:ring-blue-500/20 cursor-pointer"
          >
            <option value="">
              Choose AWS or Azure service ({AWS_EXAMPLES.length + AZURE_EXAMPLES.length} examples)…
            </option>
            <optgroup label={`Amazon Web Services — ${AWS_EXAMPLES.length} services`}>
              {AWS_EXAMPLES.map((ex) => (
                <option key={ex.id} value={ex.id}>
                  {ex.title}
                </option>
              ))}
            </optgroup>
            <optgroup label={`Microsoft Azure — ${AZURE_EXAMPLES.length} services`}>
              {AZURE_EXAMPLES.map((ex) => (
                <option key={ex.id} value={ex.id}>
                  {ex.title}
                </option>
              ))}
            </optgroup>
          </select>
        </div>
      </div>
    </div>
  )
}
