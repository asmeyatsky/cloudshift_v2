import { useCallback, useRef, useState } from 'react'
import {
  Zap,
  Upload,
  FolderOpen,
  FileArchive,
  FileCode,
  ArrowRight,
  Loader2,
} from 'lucide-react'
import clsx from 'clsx'
import { useStore } from '../store'
import { EXAMPLES } from '../content/examples'
import { readFileEntries, readZipEntries, MAX_BATCH_FILES } from '../fileImport'

const CLOUDS = [
  { value: 'aws', label: 'AWS' },
  { value: 'azure', label: 'Azure' },
  { value: 'any', label: 'Auto-detect' },
]

export default function HomeView() {
  const loadSnippet = useStore((s) => s.loadSnippet)
  const loadBatch = useStore((s) => s.loadBatch)
  const enterSnippetWorkspace = useStore((s) => s.enterSnippetWorkspace)

  const [paste, setPaste] = useState('')
  const [pasteLang, setPasteLang] = useState('python')
  const [pasteCloud, setPasteCloud] = useState('aws')
  const [importMsg, setImportMsg] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const fileRef = useRef<HTMLInputElement>(null)
  const folderRef = useRef<HTMLInputElement>(null)
  const zipRef = useRef<HTMLInputElement>(null)

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
    (ex: (typeof EXAMPLES)[number]) => {
      loadSnippet(ex.code, ex.language, ex.cloud, `example_${ex.title.replace(/\s+/g, '_')}.py`)
    },
    [loadSnippet],
  )

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
        <div className="text-center space-y-3">
          <div className="w-14 h-14 rounded-2xl bg-gradient-to-br from-blue-500/20 to-violet-500/20 border border-blue-500/15 flex items-center justify-center mx-auto">
            <Zap className="w-7 h-7 text-blue-400" />
          </div>
          <h1 className="text-xl font-semibold text-zinc-100">CloudShift</h1>
          <p className="text-sm text-zinc-500 max-w-md mx-auto">
            Transform AWS/Azure code to GCP. Paste a snippet, upload files, a folder, or a repo ZIP.
          </p>
        </div>

        {importMsg && (
          <p
            className={clsx(
              'text-center text-sm px-4 py-2 rounded-lg border',
              importMsg.startsWith('Opened') || importMsg.includes('files')
                ? 'bg-emerald-500/10 border-emerald-500/20 text-emerald-400/90'
                : 'bg-amber-500/10 border-amber-500/20 text-amber-400/90',
            )}
          >
            {importMsg}
          </p>
        )}

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

        {/* Paste snippet */}
        <div className="rounded-xl border border-[#222228] bg-[#0c0c0f] p-4 space-y-3">
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

        {/* Examples */}
        <div>
          <h2 className="text-[11px] font-medium text-zinc-500 uppercase tracking-wider mb-3 text-center">
            Examples
          </h2>
          <div className="grid sm:grid-cols-2 gap-2">
            {EXAMPLES.map((ex) => (
              <button
                key={ex.title}
                type="button"
                onClick={() => loadExample(ex)}
                className="group p-3 rounded-lg border border-[#222228] hover:border-[#333340] bg-[#111114] hover:bg-[#141418] text-left transition-all"
              >
                <div className="flex items-center gap-2 mb-1">
                  <span
                    className={clsx(
                      'text-[9px] font-bold px-1.5 py-0.5 rounded border uppercase tracking-wider',
                      ex.tagColor,
                    )}
                  >
                    {ex.tag}
                  </span>
                  <span className="text-xs font-medium text-zinc-300 truncate">{ex.title}</span>
                </div>
                <span className="text-[10px] text-zinc-600 capitalize">{ex.language}</span>
              </button>
            ))}
          </div>
        </div>
      </div>
    </div>
  )
}
