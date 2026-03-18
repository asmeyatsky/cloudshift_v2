import { useCallback, useEffect, useState } from 'react'
import MonacoEditor, { DiffEditor, type Monaco } from '@monaco-editor/react'
import {
  Zap,
  ArrowRightLeft,
  FileCode,
  Copy,
  Check,
  Loader2,
  Code2,
  Sparkles,
  CloudOff,
  Home,
  AlertTriangle,
  AlertCircle,
  Info,
  ShieldCheck,
  ListTree,
  PlayCircle,
} from 'lucide-react'
import clsx from 'clsx'
import { useStore } from '../store'
import { transform } from '../api'
import InsightsBar from './InsightsBar'
import { applyDiff } from '../applyDiff'
import type { Warning } from '../types'

const LANGUAGES = [
  { value: 'python', label: 'Python' },
  { value: 'typescript', label: 'TypeScript' },
  { value: 'javascript', label: 'JavaScript' },
  { value: 'java', label: 'Java' },
  { value: 'go', label: 'Go' },
  { value: 'hcl', label: 'HCL' },
  { value: 'yaml', label: 'YAML' },
  { value: 'dockerfile', label: 'Dockerfile' },
  { value: 'json', label: 'JSON' },
]

const CLOUDS = [
  { value: 'aws', label: 'AWS' },
  { value: 'azure', label: 'Azure' },
  { value: 'any', label: 'Auto-detect' },
]

const MONACO_LANG: Record<string, string> = {
  python: 'python',
  typescript: 'typescript',
  javascript: 'javascript',
  java: 'java',
  go: 'go',
  hcl: 'plaintext',
  yaml: 'yaml',
  dockerfile: 'dockerfile',
  json: 'json',
}

const SEVERITY: Record<
  Warning['severity'],
  { icon: typeof AlertCircle; color: string; bg: string; border: string }
> = {
  Error: {
    icon: AlertCircle,
    color: 'text-red-400',
    bg: 'bg-red-500/10',
    border: 'border-red-500/20',
  },
  Warning: {
    icon: AlertTriangle,
    color: 'text-amber-400',
    bg: 'bg-amber-500/10',
    border: 'border-amber-500/20',
  },
  Info: {
    icon: Info,
    color: 'text-blue-400',
    bg: 'bg-blue-500/10',
    border: 'border-blue-500/20',
  },
}

function defineTheme(monaco: Monaco) {
  monaco.editor.defineTheme('cloudshift-dark', {
    base: 'vs-dark',
    inherit: true,
    rules: [],
    colors: {
      'editor.background': '#111114',
      'editor.foreground': '#e4e4e7',
      'editorLineNumber.foreground': '#3f3f46',
      'editorLineNumber.activeForeground': '#71717a',
      'editor.selectionBackground': '#3b82f640',
      'editor.lineHighlightBackground': '#ffffff05',
      'editorWidget.background': '#18181c',
      'editorWidget.border': '#27272a',
      'scrollbarSlider.background': '#3f3f4680',
      'scrollbarSlider.hoverBackground': '#52525b80',
      'diffEditor.insertedTextBackground': '#22c55e15',
      'diffEditor.removedTextBackground': '#ef444415',
      'diffEditor.insertedLineBackground': '#22c55e08',
      'diffEditor.removedLineBackground': '#ef444408',
    },
  })
}

const EDITOR_OPTS = {
  minimap: { enabled: false },
  fontSize: 13,
  lineHeight: 20,
  padding: { top: 12, bottom: 12 },
  scrollBeyondLastLine: false,
  renderLineHighlight: 'none' as const,
  overviewRulerLanes: 0,
  hideCursorInOverviewRuler: true,
  overviewRulerBorder: false,
  scrollbar: { verticalScrollbarSize: 6, horizontalScrollbarSize: 6 },
  fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace",
  fontLigatures: true,
}

const BATCH_DELAY_MS = 700

export default function TransformView() {
  const workspaceMode = useStore((s) => s.workspaceMode)
  const goHome = useStore((s) => s.goHome)
  const batchItems = useStore((s) => s.batchItems)
  const batchSelectedId = useStore((s) => s.batchSelectedId)
  const isBatchRunning = useStore((s) => s.isBatchRunning)
  const setIsBatchRunning = useStore((s) => s.setIsBatchRunning)
  const updateBatchItem = useStore((s) => s.updateBatchItem)

  const code = useStore((s) => s.code)
  const language = useStore((s) => s.language)
  const sourceCloud = useStore((s) => s.sourceCloud)
  const pathHint = useStore((s) => s.pathHint)
  const result = useStore((s) => s.result)
  const transformedCode = useStore((s) => s.transformedCode)
  const isTransforming = useStore((s) => s.isTransforming)
  const error = useStore((s) => s.error)
  const resultTab = useStore((s) => s.resultTab)
  const setCode = useStore((s) => s.setCode)
  const setLanguage = useStore((s) => s.setLanguage)
  const setSourceCloud = useStore((s) => s.setSourceCloud)
  const setPathHint = useStore((s) => s.setPathHint)
  const setResultTab = useStore((s) => s.setResultTab)
  const setResult = useStore((s) => s.setResult)
  const setTransformedCode = useStore((s) => s.setTransformedCode)
  const setIsTransforming = useStore((s) => s.setIsTransforming)

  const [copied, setCopied] = useState(false)

  const flushBatchEditorToItem = useCallback(() => {
    const s = useStore.getState()
    if (s.workspaceMode === 'batch' && s.batchSelectedId) {
      s.updateBatchItem(s.batchSelectedId, { source: s.code })
    }
  }, [])

  const selectBatchRow = useCallback(
    (id: string) => {
      const s = useStore.getState()
      if (s.workspaceMode !== 'batch') return
      const prev = s.batchSelectedId
      if (prev) s.updateBatchItem(prev, { source: s.code })
      const item = s.batchItems.find((b) => b.id === id)
      if (!item) return
      s.setBatchSelectedId(id)
      s.setCode(item.source)
      s.setLanguage(item.language)
      s.setPathHint(item.path)
      s.setError(null)
      if (item.result) {
        s.setResult(item.result)
        const out =
          item.result.transformed_source && item.result.transformed_source.length > 0
            ? item.result.transformed_source
            : applyDiff(item.source, item.result.diff)
        s.setTransformedCode(out)
        s.setResultTab('diff')
      } else {
        s.setResult(null)
        s.setTransformedCode('')
      }
    },
    [],
  )

  const handleTransform = useCallback(async () => {
    const s = useStore.getState()
    if (!s.code.trim() || s.isTransforming) return
    const batchItemId = s.workspaceMode === 'batch' ? s.batchSelectedId : null
    if (batchItemId) s.updateBatchItem(batchItemId, { source: s.code })

    const lang = s.language
    const cloud = s.sourceCloud
    const path = s.pathHint || undefined
    const src = s.code
    const key = s.apiKey || undefined

    s.setIsTransforming(true)
    s.setError(null)

    try {
      const res = await transform(
        {
          source: src,
          language: lang,
          source_cloud: cloud,
          path_hint: path,
        },
        key,
      )

      const st = useStore.getState()
      st.setResult(res)
      const out =
        res.transformed_source && res.transformed_source.length > 0
          ? res.transformed_source
          : applyDiff(src, res.diff)
      st.setTransformedCode(out)
      st.setResultTab('diff')

      if (batchItemId) {
        st.updateBatchItem(batchItemId, {
          result: res,
          status: 'done',
          error: undefined,
          source: src,
        })
      }

      st.addToHistory({
        code: src,
        language: lang,
        sourceCloud: cloud,
        result: res,
      })
    } catch (err) {
      const st = useStore.getState()
      const msg = err instanceof Error ? err.message : 'Transform failed'
      st.setError(msg)
      if (st.batchSelectedId === batchItemId) {
        st.setResult(null)
        st.setTransformedCode('')
      }
      if (batchItemId) {
        st.updateBatchItem(batchItemId, {
          status: 'error',
          error: msg,
        })
      }
    } finally {
      useStore.getState().setIsTransforming(false)
    }
  }, [])

  const runBatchAll = useCallback(async () => {
    flushBatchEditorToItem()
    const s0 = useStore.getState()
    setIsBatchRunning(true)
    const items = [...s0.batchItems]
    const key = s0.apiKey || undefined
    const cloud = s0.sourceCloud

    for (let i = 0; i < items.length; i++) {
      const item = items[i]
      const fresh = useStore.getState().batchItems.find((b) => b.id === item.id)
      const source = fresh?.source ?? item.source
      const lang = fresh?.language ?? item.language
      const path = fresh?.path ?? item.path

      useStore.getState().updateBatchItem(item.id, { status: 'running', error: undefined })
      try {
        const res = await transform(
          {
            source,
            language: lang,
            source_cloud: cloud,
            path_hint: path || undefined,
          },
          key,
        )
        useStore.getState().updateBatchItem(item.id, {
          result: res,
          status: 'done',
          source,
        })
        const sel = useStore.getState().batchSelectedId
        if (sel === item.id) {
          useStore.getState().setResult(res)
          const out =
            res.transformed_source && res.transformed_source.length > 0
              ? res.transformed_source
              : applyDiff(source, res.diff)
          useStore.getState().setTransformedCode(out)
          useStore.getState().setResultTab('diff')
        }
      } catch (err) {
        useStore.getState().updateBatchItem(item.id, {
          status: 'error',
          error: err instanceof Error ? err.message : 'Failed',
        })
      }
      if (i < items.length - 1) await new Promise((r) => setTimeout(r, BATCH_DELAY_MS))
    }
    setIsBatchRunning(false)
  }, [flushBatchEditorToItem, setIsBatchRunning])

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
        e.preventDefault()
        if (!useStore.getState().isBatchRunning) handleTransform()
      }
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [handleTransform])

  const handleCopy = () => {
    if (!transformedCode.trim()) return
    navigator.clipboard.writeText(transformedCode)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  const lang = MONACO_LANG[language] || 'plaintext'
  const hasResult = !!result
  const isBatch = workspaceMode === 'batch'

  const batchDone = batchItems.filter((b) => b.status === 'done').length
  const batchErr = batchItems.filter((b) => b.status === 'error').length

  return (
    <div className="flex-1 flex flex-col min-h-0">
      <div className="flex-1 flex min-h-0">
        {isBatch && (
          <aside className="w-52 shrink-0 border-r border-[#1e1e22] bg-[#0c0c0f] flex flex-col min-h-0">
            <div className="p-2 border-b border-[#1e1e22] space-y-2">
              <div className="flex items-center gap-1.5 text-[10px] text-zinc-500 uppercase tracking-wider px-1">
                <ListTree className="w-3 h-3" />
                Files ({batchItems.length})
              </div>
              <button
                type="button"
                onClick={runBatchAll}
                disabled={isBatchRunning || batchItems.length === 0}
                className="w-full h-8 flex items-center justify-center gap-1.5 text-[11px] font-semibold rounded-md bg-violet-600 hover:bg-violet-500 text-white disabled:opacity-40"
              >
                {isBatchRunning ? (
                  <Loader2 className="w-3.5 h-3.5 animate-spin" />
                ) : (
                  <PlayCircle className="w-3.5 h-3.5" />
                )}
                Transform all
              </button>
              <p className="text-[9px] text-zinc-600 px-1 leading-snug">
                {batchDone} done
                {batchErr ? ` · ${batchErr} errors` : ''}
              </p>
            </div>
            <ul className="flex-1 overflow-y-auto p-1 space-y-0.5">
              {batchItems.map((b) => (
                <li key={b.id}>
                  <button
                    type="button"
                    onClick={() => selectBatchRow(b.id)}
                    className={clsx(
                      'w-full text-left px-2 py-1.5 rounded-md text-[11px] truncate transition-colors',
                      batchSelectedId === b.id
                        ? 'bg-blue-500/15 text-blue-300 border border-blue-500/25'
                        : 'text-zinc-400 hover:bg-white/5 border border-transparent',
                    )}
                  >
                    <span className="block truncate font-mono text-[10px]">{b.path}</span>
                    <span
                      className={clsx(
                        'text-[9px] mt-0.5',
                        b.status === 'done' && 'text-emerald-500/80',
                        b.status === 'error' && 'text-red-400/80',
                        b.status === 'running' && 'text-amber-400',
                        b.status === 'pending' && 'text-zinc-600',
                      )}
                    >
                      {b.status === 'done' && '✓ transformed'}
                      {b.status === 'error' && (b.error ? b.error.slice(0, 24) : 'Error')}
                      {b.status === 'running' && '…'}
                      {b.status === 'pending' && 'pending'}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          </aside>
        )}

        {/* Source */}
        <div className="flex-1 flex flex-col min-w-0 border-r border-[#1e1e22]">
          <div className="flex items-center gap-2 px-3 h-10 border-b border-[#1e1e22] bg-[#0c0c0f] shrink-0 flex-wrap">
            <button
              type="button"
              onClick={() => goHome()}
              className="h-7 px-2 flex items-center gap-1 text-[11px] text-zinc-500 hover:text-zinc-300 rounded border border-[#27272a] hover:bg-white/5"
              title="Home"
            >
              <Home className="w-3 h-3" />
              Menu
            </button>
            <div className="w-px h-4 bg-[#1e1e22]" />
            <Code2 className="w-3.5 h-3.5 text-zinc-600" />
            <span className="text-[11px] font-medium text-zinc-500 uppercase tracking-wider">Source</span>
            <div className="w-px h-4 bg-[#1e1e22]" />

            <select
              value={language}
              onChange={(e) => {
                setLanguage(e.target.value)
                if (isBatch && batchSelectedId) updateBatchItem(batchSelectedId, { language: e.target.value })
              }}
              className="h-6 px-2 text-[11px] bg-transparent border border-[#27272a] rounded text-zinc-300 outline-none focus:border-blue-500/50 cursor-pointer"
            >
              {LANGUAGES.map((l) => (
                <option key={l.value} value={l.value}>
                  {l.label}
                </option>
              ))}
            </select>

            <select
              value={sourceCloud}
              onChange={(e) => setSourceCloud(e.target.value)}
              className="h-6 px-2 text-[11px] bg-transparent border border-[#27272a] rounded text-zinc-300 outline-none focus:border-blue-500/50 cursor-pointer"
            >
              {CLOUDS.map((c) => (
                <option key={c.value} value={c.value}>
                  {c.label}
                </option>
              ))}
            </select>

            <input
              type="text"
              placeholder="path hint"
              value={pathHint}
              onChange={(e) => {
                setPathHint(e.target.value)
                if (isBatch && batchSelectedId) updateBatchItem(batchSelectedId, { path: e.target.value })
              }}
              className="h-6 px-2 text-[11px] bg-transparent border border-[#27272a] rounded text-zinc-400 outline-none focus:border-blue-500/50 placeholder:text-zinc-700 w-24 sm:w-32"
            />

            <div className="flex-1 min-w-2" />

            <button
              onClick={handleTransform}
              disabled={isTransforming || !code.trim() || isBatchRunning}
              className={clsx(
                'h-7 px-3.5 text-[11px] font-semibold rounded-md flex items-center gap-1.5 transition-all',
                'bg-blue-600 hover:bg-blue-500 text-white',
                'disabled:opacity-30 disabled:cursor-not-allowed disabled:hover:bg-blue-600',
              )}
            >
              {isTransforming ? (
                <Loader2 className="w-3.5 h-3.5 animate-spin" />
              ) : (
                <Zap className="w-3 h-3" />
              )}
              {isBatch ? 'This file' : 'Transform'}
              <kbd className="hidden lg:inline text-[9px] opacity-50 ml-0.5 font-mono">
                {typeof navigator !== 'undefined' && /Mac|iPhone|iPod|iPad/i.test(navigator.platform)
                  ? '\u2318\u21B5'
                  : 'Ctrl+Enter'}
              </kbd>
            </button>
          </div>

          <div className="flex-1 min-h-0">
            <MonacoEditor
              language={lang}
              value={code}
              onChange={(v) => {
                const next = v || ''
                setCode(next)
                if (isBatch && batchSelectedId) updateBatchItem(batchSelectedId, { source: next })
              }}
              theme="cloudshift-dark"
              beforeMount={defineTheme}
              options={EDITOR_OPTS}
            />
          </div>
        </div>

        {/* Result */}
        <div className="flex-1 flex flex-col min-w-0">
          {hasResult ? (
            <>
              <div className="flex items-center gap-2 px-3 h-10 border-b border-[#1e1e22] bg-[#0c0c0f] shrink-0 flex-wrap">
                <Sparkles className="w-3.5 h-3.5 text-zinc-600" />
                <span className="text-[11px] font-medium text-zinc-500 uppercase tracking-wider">Result</span>
                <div className="w-px h-4 bg-[#1e1e22]" />

                <div className="flex bg-[#18181c] rounded-md p-0.5 border border-[#222228]">
                  {(['diff', 'code', 'insights'] as const).map((tab) => (
                    <button
                      key={tab}
                      onClick={() => setResultTab(tab)}
                      className={clsx(
                        'px-2.5 py-1 text-[11px] rounded flex items-center gap-1 transition-colors capitalize',
                        resultTab === tab
                          ? 'bg-[#27272a] text-zinc-200 shadow-sm'
                          : 'text-zinc-500 hover:text-zinc-300',
                      )}
                    >
                      {tab === 'diff' && <ArrowRightLeft className="w-3 h-3" />}
                      {tab === 'code' && <FileCode className="w-3 h-3" />}
                      {tab === 'insights' && <Sparkles className="w-3 h-3" />}
                      {tab === 'insights' ? 'Patterns & warnings' : tab}
                    </button>
                  ))}
                </div>

                <div className="flex-1" />

                {resultTab !== 'insights' && (
                  <button
                    onClick={handleCopy}
                    className="h-6 px-2 text-[11px] rounded border border-[#27272a] text-zinc-500 hover:text-zinc-300 hover:bg-white/5 flex items-center gap-1"
                  >
                    {copied ? <Check className="w-3 h-3 text-emerald-400" /> : <Copy className="w-3 h-3" />}
                    {copied ? 'Copied' : 'Copy'}
                  </button>
                )}
              </div>

              <div className="flex-1 min-h-0 overflow-hidden">
                {resultTab === 'insights' && result ? (
                  <div className="h-full overflow-y-auto p-4 space-y-6 bg-[#111114]">
                    <div>
                      <h3 className="text-[11px] font-medium text-zinc-500 uppercase tracking-wider mb-3">
                        Patterns matched ({result.patterns.length})
                      </h3>
                      <div className="space-y-2">
                        {result.patterns.map((p, i) => (
                          <div key={i} className="p-3 rounded-lg bg-[#141417] border border-[#222228]">
                            <div className="flex items-center justify-between mb-2">
                              <span className="text-[11px] font-mono text-violet-400 break-all">
                                {p.pattern_id.join(', ')}
                              </span>
                              <span
                                className={clsx(
                                  'text-[10px] font-medium px-1.5 py-0.5 rounded-full shrink-0 ml-2',
                                  p.confidence >= 0.8
                                    ? 'bg-emerald-500/10 text-emerald-400'
                                    : p.confidence >= 0.5
                                      ? 'bg-amber-500/10 text-amber-400'
                                      : 'bg-red-500/10 text-red-400',
                                )}
                              >
                                {Math.round(p.confidence * 100)}%
                              </span>
                            </div>
                            <div className="text-[11px] font-mono space-y-1">
                              <div className="text-red-400/80 whitespace-pre-wrap break-all">− {p.source_text}</div>
                              <div className="text-emerald-400/80 whitespace-pre-wrap break-all">+ {p.replacement_text}</div>
                            </div>
                            {p.span && (
                              <p className="text-[10px] text-zinc-600 mt-2">
                                Lines {p.span.start_line}–{p.span.end_line}
                              </p>
                            )}
                          </div>
                        ))}
                        {result.patterns.length === 0 && (
                          <p className="text-sm text-zinc-600 italic">No patterns matched.</p>
                        )}
                      </div>
                    </div>
                    <div>
                      <h3 className="text-[11px] font-medium text-zinc-500 uppercase tracking-wider mb-3">
                        Warnings ({result.warnings.length})
                      </h3>
                      <div className="space-y-2">
                        {result.warnings.map((w, i) => {
                          const cfg = SEVERITY[w.severity]
                          const Icon = cfg.icon
                          return (
                            <div
                              key={i}
                              className={clsx('p-3 rounded-lg border flex items-start gap-2', cfg.bg, cfg.border)}
                            >
                              <Icon className={clsx('w-4 h-4 shrink-0 mt-0.5', cfg.color)} />
                              <div>
                                <p className="text-sm text-zinc-300">{w.message}</p>
                                {w.span && (
                                  <p className="text-[10px] text-zinc-600 mt-1">
                                    Lines {w.span.start_line}–{w.span.end_line}
                                  </p>
                                )}
                              </div>
                            </div>
                          )
                        })}
                        {result.warnings.length === 0 && (
                          <div className="flex items-center gap-2 text-sm text-emerald-400/70">
                            <ShieldCheck className="w-4 h-4" />
                            No warnings
                          </div>
                        )}
                      </div>
                    </div>
                    <div className="pt-2 border-t border-[#222228]">
                      <p className="text-[11px] text-zinc-500">
                        Confidence:{' '}
                        <span
                          className={clsx(
                            'font-semibold',
                            result.confidence >= 0.8
                              ? 'text-emerald-400'
                              : result.confidence >= 0.5
                                ? 'text-amber-400'
                                : 'text-red-400',
                          )}
                        >
                          {Math.round(result.confidence * 100)}%
                        </span>
                      </p>
                    </div>
                  </div>
                ) : resultTab === 'diff' ? (
                  <DiffEditor
                    original={code}
                    modified={transformedCode}
                    language={lang}
                    theme="cloudshift-dark"
                    beforeMount={defineTheme}
                    options={{
                      ...EDITOR_OPTS,
                      readOnly: true,
                      renderSideBySide: true,
                    }}
                  />
                ) : (
                  <MonacoEditor
                    language={lang}
                    value={transformedCode}
                    theme="cloudshift-dark"
                    beforeMount={defineTheme}
                    options={{ ...EDITOR_OPTS, readOnly: true }}
                  />
                )}
              </div>
            </>
          ) : isTransforming ? (
            <div className="flex-1 flex items-center justify-center">
              <div className="text-center space-y-4">
                <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-blue-500/20 to-violet-500/20 flex items-center justify-center mx-auto">
                  <Loader2 className="w-6 h-6 text-blue-400 animate-spin" />
                </div>
                <div>
                  <p className="text-sm text-zinc-300">Transforming…</p>
                  <p className="text-xs text-zinc-600 mt-1">Matching patterns and generating GCP code</p>
                </div>
              </div>
            </div>
          ) : error ? (
            <div className="flex-1 flex items-center justify-center p-8">
              <div className="text-center max-w-sm space-y-3">
                <div className="w-10 h-10 rounded-xl bg-red-500/10 flex items-center justify-center mx-auto">
                  <CloudOff className="w-5 h-5 text-red-400" />
                </div>
                <p className="text-sm text-red-400 font-medium">Transform failed</p>
                <p className="text-xs text-zinc-500 break-words">{error}</p>
                {(error.includes('401') || error.toLowerCase().includes('auth')) && (
                  <p className="text-xs text-zinc-600">Check API key in Settings</p>
                )}
              </div>
            </div>
          ) : (
            <div className="flex-1 flex items-center justify-center p-8">
              <div className="text-center max-w-md space-y-4">
                <Zap className="w-10 h-10 text-blue-400/40 mx-auto" />
                <div>
                  <h2 className="text-base font-semibold text-zinc-300">No result yet</h2>
                  <p className="text-xs text-zinc-600 mt-2">
                    {isBatch
                      ? 'Select a file, edit if needed, then Transform this file or use Transform all.'
                      : 'Run Transform on the left, or open Menu to try examples and imports.'}
                  </p>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>

      {hasResult && resultTab !== 'insights' && <InsightsBar />}
    </div>
  )
}
