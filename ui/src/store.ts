import { create } from 'zustand'
import { TransformResult, HistoryEntry } from './types'

function loadHistory(): HistoryEntry[] {
  try {
    return JSON.parse(localStorage.getItem('cloudshift_history') || '[]')
  } catch {
    return []
  }
}

export type BatchFileItem = {
  id: string
  path: string
  source: string
  language: string
  sourceCloud: string
  result?: TransformResult
  error?: string
  status: 'pending' | 'running' | 'done' | 'error' | 'skipped'
}

interface AppState {
  /** home = menu + imports; workspace = editor / batch */
  screen: 'home' | 'workspace'
  workspaceMode: 'snippet' | 'batch'

  code: string
  language: string
  sourceCloud: string
  pathHint: string

  result: TransformResult | null
  transformedCode: string
  isTransforming: boolean
  error: string | null

  showSettings: boolean
  resultTab: 'diff' | 'code' | 'insights'

  apiKey: string
  authVerified: boolean | null
  history: HistoryEntry[]

  batchItems: BatchFileItem[]
  batchSelectedId: string | null
  isBatchRunning: boolean

  setCode: (code: string) => void
  setLanguage: (lang: string) => void
  setSourceCloud: (cloud: string) => void
  setPathHint: (hint: string) => void
  setResult: (result: TransformResult | null) => void
  setTransformedCode: (code: string) => void
  setIsTransforming: (v: boolean) => void
  setError: (error: string | null) => void
  setShowSettings: (v: boolean) => void
  setResultTab: (tab: 'diff' | 'code' | 'insights') => void
  setApiKey: (key: string) => void
  setAuthVerified: (v: boolean | null) => void
  addToHistory: (entry: Omit<HistoryEntry, 'id' | 'timestamp'>) => void
  clearHistory: () => void
  clearResult: () => void

  goHome: () => void
  enterSnippetWorkspace: () => void
  loadSnippet: (code: string, language: string, cloud: string, pathHint: string) => void
  loadBatch: (items: Omit<BatchFileItem, 'id' | 'status' | 'sourceCloud'>[], sourceCloud: string) => void
  setBatchSelectedId: (id: string | null) => void
  updateBatchItem: (id: string, patch: Partial<BatchFileItem>) => void
  clearBatch: () => void
  setIsBatchRunning: (v: boolean) => void
}

export const useStore = create<AppState>((set) => ({
  screen: 'home',
  workspaceMode: 'snippet',

  code: '',
  language: 'python',
  sourceCloud: 'aws',
  pathHint: '',

  result: null,
  transformedCode: '',
  isTransforming: false,
  error: null,

  showSettings: false,
  resultTab: 'diff',

  apiKey: localStorage.getItem('cloudshift_api_key') || '',
  authVerified: null,
  history: loadHistory(),

  batchItems: [],
  batchSelectedId: null,
  isBatchRunning: false,

  setCode: (code) => set({ code }),
  setLanguage: (language) => set({ language }),
  setSourceCloud: (sourceCloud) => set({ sourceCloud }),
  setPathHint: (pathHint) => set({ pathHint }),
  setResult: (result) => set({ result }),
  setTransformedCode: (transformedCode) => set({ transformedCode }),
  setIsTransforming: (isTransforming) => set({ isTransforming }),
  setError: (error) => set({ error }),
  setShowSettings: (showSettings) => set({ showSettings }),
  setResultTab: (resultTab) => set({ resultTab }),
  setApiKey: (apiKey) => {
    localStorage.setItem('cloudshift_api_key', apiKey)
    set({ apiKey, authVerified: null })
  },
  setAuthVerified: (authVerified) => set({ authVerified }),
  addToHistory: (entry) =>
    set((state) => {
      const newEntry: HistoryEntry = {
        ...entry,
        id: crypto.randomUUID(),
        timestamp: Date.now(),
      }
      const history = [newEntry, ...state.history].slice(0, 20)
      localStorage.setItem('cloudshift_history', JSON.stringify(history))
      return { history }
    }),
  clearHistory: () => {
    localStorage.removeItem('cloudshift_history')
    set({ history: [] })
  },
  clearResult: () => set({ result: null, transformedCode: '', error: null }),

  goHome: () =>
    set({
      screen: 'home',
      workspaceMode: 'snippet',
      result: null,
      transformedCode: '',
      error: null,
      code: '',
      pathHint: '',
      batchItems: [],
      batchSelectedId: null,
      isBatchRunning: false,
      resultTab: 'diff',
    }),

  enterSnippetWorkspace: () =>
    set({
      screen: 'workspace',
      workspaceMode: 'snippet',
      batchItems: [],
      batchSelectedId: null,
    }),

  loadSnippet: (code, language, sourceCloud, pathHint) =>
    set({
      screen: 'workspace',
      workspaceMode: 'snippet',
      code,
      language,
      sourceCloud,
      pathHint,
      result: null,
      transformedCode: '',
      error: null,
      batchItems: [],
      batchSelectedId: null,
      resultTab: 'diff',
    }),

  loadBatch: (items, sourceCloud) => {
    const batchItems: BatchFileItem[] = items.map((i) => ({
      ...i,
      id: crypto.randomUUID(),
      status: 'pending' as const,
      sourceCloud,
    }))
    const first = batchItems[0]
    set({
      screen: 'workspace',
      workspaceMode: 'batch',
      batchItems,
      batchSelectedId: first?.id ?? null,
      code: first?.source ?? '',
      language: first?.language ?? 'python',
      sourceCloud,
      pathHint: first?.path ?? '',
      result: null,
      transformedCode: '',
      error: null,
      resultTab: 'diff',
    })
  },

  setBatchSelectedId: (batchSelectedId) => set({ batchSelectedId }),

  updateBatchItem: (id, patch) =>
    set((s) => ({
      batchItems: s.batchItems.map((b) => (b.id === id ? { ...b, ...patch } : b)),
    })),

  clearBatch: () =>
    set({
      batchItems: [],
      batchSelectedId: null,
      isBatchRunning: false,
    }),

  setIsBatchRunning: (isBatchRunning) => set({ isBatchRunning }),
}))
