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
} from 'lucide-react'
import clsx from 'clsx'
import { useStore } from '../store'
import { transform } from '../api'
import InsightsBar from './InsightsBar'

/* ── constants ─────────────────────────────────────────── */

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

const EXAMPLES = [
  {
    title: 'S3 to Cloud Storage',
    tag: 'AWS',
    tagColor: 'bg-orange-500/10 text-orange-400 border-orange-500/20',
    language: 'python',
    cloud: 'aws',
    code: `import boto3

s3 = boto3.client('s3')

def upload_file(bucket, key, data):
    s3.put_object(Bucket=bucket, Key=key, Body=data)

def download_file(bucket, key):
    response = s3.get_object(Bucket=bucket, Key=key)
    return response['Body'].read()

def list_files(bucket, prefix=''):
    response = s3.list_objects_v2(Bucket=bucket, Prefix=prefix)
    return [obj['Key'] for obj in response.get('Contents', [])]`,
  },
  {
    title: 'DynamoDB to Firestore',
    tag: 'AWS',
    tagColor: 'bg-orange-500/10 text-orange-400 border-orange-500/20',
    language: 'python',
    cloud: 'aws',
    code: `import boto3

dynamodb = boto3.resource('dynamodb')
table = dynamodb.Table('users')

def get_user(user_id):
    response = table.get_item(Key={'id': user_id})
    return response.get('Item')

def put_user(user):
    table.put_item(Item=user)

def query_users(status):
    response = table.query(
        KeyConditionExpression='status = :s',
        ExpressionAttributeValues={':s': status}
    )
    return response['Items']`,
  },
  {
    title: 'Lambda to Cloud Functions',
    tag: 'AWS',
    tagColor: 'bg-orange-500/10 text-orange-400 border-orange-500/20',
    language: 'python',
    cloud: 'aws',
    code: `import json
import boto3

def lambda_handler(event, context):
    sqs = boto3.client('sqs')
    sqs.send_message(
        QueueUrl='https://sqs.us-east-1.amazonaws.com/123/my-queue',
        MessageBody=json.dumps(event)
    )
    return {
        'statusCode': 200,
        'body': json.dumps({'message': 'Processed successfully'})
    }`,
  },
  {
    title: 'Blob to Cloud Storage',
    tag: 'Azure',
    tagColor: 'bg-sky-500/10 text-sky-400 border-sky-500/20',
    language: 'python',
    cloud: 'azure',
    code: `from azure.storage.blob import BlobServiceClient

connection_string = "DefaultEndpointsProtocol=https;AccountName=..."
blob_service = BlobServiceClient.from_connection_string(connection_string)
container = blob_service.get_container_client("my-container")

def upload_blob(name, data):
    blob = container.get_blob_client(name)
    blob.upload_blob(data, overwrite=True)

def download_blob(name):
    blob = container.get_blob_client(name)
    return blob.download_blob().readall()`,
  },
]

/* ── Monaco theme ──────────────────────────────────────── */

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

/* ── diff applier ──────────────────────────────────────── */

function applyDiff(original: string, diff: string): string {
  if (!diff) return original

  const origLines = original.split('\n')
  const diffLines = diff.split('\n')
  const result: string[] = []
  let origIdx = 0

  for (const line of diffLines) {
    if (line.startsWith('@@')) {
      const m = line.match(/@@ -(\d+)/)
      if (m) {
        const start = parseInt(m[1]) - 1
        while (origIdx < start) result.push(origLines[origIdx++])
      }
      continue
    }
    if (line.startsWith('---') || line.startsWith('+++') || line.startsWith('diff ')) continue
    if (line.startsWith('-')) origIdx++
    else if (line.startsWith('+')) result.push(line.slice(1))
    else if (line.startsWith(' ')) {
      result.push(line.slice(1))
      origIdx++
    }
  }

  while (origIdx < origLines.length) result.push(origLines[origIdx++])
  return result.join('\n')
}

/* ── editor options ────────────────────────────────────── */

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

/* ── component ─────────────────────────────────────────── */

export default function TransformView() {
  const {
    code,
    language,
    sourceCloud,
    pathHint,
    result,
    transformedCode,
    isTransforming,
    error,
    resultTab,
    setCode,
    setLanguage,
    setSourceCloud,
    setPathHint,
    setResultTab,
    clearResult,
  } = useStore()

  const [copied, setCopied] = useState(false)

  /* transform action — reads latest state via getState() */
  const handleTransform = useCallback(async () => {
    const s = useStore.getState()
    if (!s.code.trim() || s.isTransforming) return

    s.setIsTransforming(true)
    s.setError(null)

    try {
      const res = await transform(
        {
          source: s.code,
          language: s.language,
          source_cloud: s.sourceCloud,
          path_hint: s.pathHint || undefined,
        },
        s.apiKey || undefined,
      )

      s.setResult(res)
      s.setTransformedCode(applyDiff(s.code, res.diff))
      s.setResultTab('diff')
      s.addToHistory({
        code: s.code,
        language: s.language,
        sourceCloud: s.sourceCloud,
        result: res,
      })
    } catch (err) {
      s.setError(err instanceof Error ? err.message : 'Transform failed')
      s.setResult(null)
      s.setTransformedCode('')
    } finally {
      s.setIsTransforming(false)
    }
  }, [])

  /* keyboard shortcut */
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
        e.preventDefault()
        handleTransform()
      }
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [handleTransform])

  const handleCopy = () => {
    navigator.clipboard.writeText(transformedCode)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  const loadExample = (ex: (typeof EXAMPLES)[number]) => {
    setCode(ex.code)
    setLanguage(ex.language)
    setSourceCloud(ex.cloud)
    clearResult()
  }

  const lang = MONACO_LANG[language] || 'plaintext'
  const hasResult = !!result

  return (
    <div className="flex-1 flex flex-col min-h-0">
      <div className="flex-1 flex min-h-0">
        {/* ── Source panel ── */}
        <div className="flex-1 flex flex-col min-w-0 border-r border-[#1e1e22]">
          {/* Toolbar */}
          <div className="flex items-center gap-2 px-3 h-10 border-b border-[#1e1e22] bg-[#0c0c0f] shrink-0">
            <Code2 className="w-3.5 h-3.5 text-zinc-600" />
            <span className="text-[11px] font-medium text-zinc-500 uppercase tracking-wider mr-1">
              Source
            </span>
            <div className="w-px h-4 bg-[#1e1e22]" />

            <select
              value={language}
              onChange={(e) => setLanguage(e.target.value)}
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
              placeholder="file path hint"
              value={pathHint}
              onChange={(e) => setPathHint(e.target.value)}
              className="h-6 px-2 text-[11px] bg-transparent border border-[#27272a] rounded text-zinc-400 outline-none focus:border-blue-500/50 placeholder:text-zinc-700 w-28"
            />

            <div className="flex-1" />

            <button
              onClick={handleTransform}
              disabled={isTransforming || !code.trim()}
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
              Transform
              <kbd className="hidden lg:inline text-[9px] opacity-50 ml-0.5 font-mono">
                {'\u2318\u21B5'}
              </kbd>
            </button>
          </div>

          {/* Editor */}
          <div className="flex-1 min-h-0">
            <MonacoEditor
              language={lang}
              value={code}
              onChange={(v) => setCode(v || '')}
              theme="cloudshift-dark"
              beforeMount={defineTheme}
              options={EDITOR_OPTS}
            />
          </div>
        </div>

        {/* ── Result panel ── */}
        <div className="flex-1 flex flex-col min-w-0">
          {hasResult ? (
            <>
              {/* Result toolbar */}
              <div className="flex items-center gap-2 px-3 h-10 border-b border-[#1e1e22] bg-[#0c0c0f] shrink-0">
                <Sparkles className="w-3.5 h-3.5 text-zinc-600" />
                <span className="text-[11px] font-medium text-zinc-500 uppercase tracking-wider mr-1">
                  Result
                </span>
                <div className="w-px h-4 bg-[#1e1e22]" />

                <div className="flex bg-[#18181c] rounded-md p-0.5 border border-[#222228]">
                  <button
                    onClick={() => setResultTab('diff')}
                    className={clsx(
                      'px-2.5 py-1 text-[11px] rounded flex items-center gap-1 transition-colors',
                      resultTab === 'diff'
                        ? 'bg-[#27272a] text-zinc-200 shadow-sm'
                        : 'text-zinc-500 hover:text-zinc-300',
                    )}
                  >
                    <ArrowRightLeft className="w-3 h-3" />
                    Diff
                  </button>
                  <button
                    onClick={() => setResultTab('code')}
                    className={clsx(
                      'px-2.5 py-1 text-[11px] rounded flex items-center gap-1 transition-colors',
                      resultTab === 'code'
                        ? 'bg-[#27272a] text-zinc-200 shadow-sm'
                        : 'text-zinc-500 hover:text-zinc-300',
                    )}
                  >
                    <FileCode className="w-3 h-3" />
                    Code
                  </button>
                </div>

                <div className="flex-1" />

                <button
                  onClick={handleCopy}
                  className="h-6 px-2 text-[11px] rounded border border-[#27272a] text-zinc-500 hover:text-zinc-300 hover:bg-white/5 transition-colors flex items-center gap-1"
                  title="Copy transformed code"
                >
                  {copied ? (
                    <Check className="w-3 h-3 text-emerald-400" />
                  ) : (
                    <Copy className="w-3 h-3" />
                  )}
                  {copied ? 'Copied' : 'Copy'}
                </button>
              </div>

              {/* Diff or code view */}
              <div className="flex-1 min-h-0">
                {resultTab === 'diff' ? (
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
            /* Loading */
            <div className="flex-1 flex items-center justify-center">
              <div className="text-center space-y-4">
                <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-blue-500/20 to-violet-500/20 flex items-center justify-center mx-auto">
                  <Loader2 className="w-6 h-6 text-blue-400 animate-spin" />
                </div>
                <div>
                  <p className="text-sm text-zinc-300">Transforming...</p>
                  <p className="text-xs text-zinc-600 mt-1">
                    Matching patterns and generating GCP equivalents
                  </p>
                </div>
              </div>
            </div>
          ) : error ? (
            /* Error */
            <div className="flex-1 flex items-center justify-center p-8">
              <div className="text-center max-w-sm space-y-3">
                <div className="w-10 h-10 rounded-xl bg-red-500/10 flex items-center justify-center mx-auto">
                  <CloudOff className="w-5 h-5 text-red-400" />
                </div>
                <p className="text-sm text-red-400 font-medium">Transform failed</p>
                <p className="text-xs text-zinc-500 break-words">{error}</p>
                {(error.includes('401') || error.toLowerCase().includes('auth')) && (
                  <p className="text-xs text-zinc-600">
                    Check your API key in Settings
                  </p>
                )}
              </div>
            </div>
          ) : (
            /* Empty state */
            <div className="flex-1 flex items-center justify-center p-8 overflow-y-auto">
              <div className="max-w-lg w-full space-y-8">
                <div className="text-center space-y-3">
                  <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-blue-500/10 to-violet-500/10 border border-blue-500/10 flex items-center justify-center mx-auto">
                    <Zap className="w-5 h-5 text-blue-400" />
                  </div>
                  <div>
                    <h2 className="text-base font-semibold text-zinc-200">
                      Transform to GCP
                    </h2>
                    <p className="text-xs text-zinc-500 mt-1.5 leading-relaxed max-w-xs mx-auto">
                      Paste AWS or Azure code on the left and hit Transform.
                      CloudShift rewrites it to GCP equivalents using 134+ patterns.
                    </p>
                  </div>
                </div>

                <div>
                  <h3 className="text-[11px] font-medium text-zinc-600 uppercase tracking-wider mb-3 text-center">
                    Try an example
                  </h3>
                  <div className="grid grid-cols-2 gap-2">
                    {EXAMPLES.map((ex) => (
                      <button
                        key={ex.title}
                        onClick={() => loadExample(ex)}
                        className="group p-3 rounded-lg border border-[#222228] hover:border-[#333340] bg-[#111114] hover:bg-[#141418] transition-all text-left"
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
                          <span className="text-xs font-medium text-zinc-300 group-hover:text-zinc-100 transition-colors truncate">
                            {ex.title}
                          </span>
                        </div>
                        <span className="text-[10px] text-zinc-600">Python</span>
                      </button>
                    ))}
                  </div>
                </div>

                <div className="text-center">
                  <p className="text-[10px] text-zinc-700">
                    Press{' '}
                    <kbd className="px-1 py-0.5 rounded bg-[#1e1e22] text-zinc-500 font-mono text-[9px]">
                      {'\u2318'}
                    </kbd>{' '}
                    +{' '}
                    <kbd className="px-1 py-0.5 rounded bg-[#1e1e22] text-zinc-500 font-mono text-[9px]">
                      Enter
                    </kbd>{' '}
                    to transform
                  </p>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Insights bar */}
      {hasResult && <InsightsBar />}
    </div>
  )
}
