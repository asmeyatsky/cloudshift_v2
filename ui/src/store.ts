import { create } from 'zustand'
import { TransformResult, HistoryEntry } from './types'

function loadHistory(): HistoryEntry[] {
  try {
    return JSON.parse(localStorage.getItem('cloudshift_history') || '[]')
  } catch {
    return []
  }
}

interface AppState {
  code: string
  language: string
  sourceCloud: string
  pathHint: string

  result: TransformResult | null
  transformedCode: string
  isTransforming: boolean
  error: string | null

  showSettings: boolean
  resultTab: 'diff' | 'code'

  apiKey: string
  /** null = not yet checked; IAP may succeed without apiKey */
  authVerified: boolean | null
  history: HistoryEntry[]

  setCode: (code: string) => void
  setLanguage: (lang: string) => void
  setSourceCloud: (cloud: string) => void
  setPathHint: (hint: string) => void
  setResult: (result: TransformResult | null) => void
  setTransformedCode: (code: string) => void
  setIsTransforming: (v: boolean) => void
  setError: (error: string | null) => void
  setShowSettings: (v: boolean) => void
  setResultTab: (tab: 'diff' | 'code') => void
  setApiKey: (key: string) => void
  setAuthVerified: (v: boolean | null) => void
  addToHistory: (entry: Omit<HistoryEntry, 'id' | 'timestamp'>) => void
  clearHistory: () => void
  clearResult: () => void
}

export const useStore = create<AppState>((set) => ({
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
}))
