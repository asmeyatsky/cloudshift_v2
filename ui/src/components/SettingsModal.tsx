import { useState } from 'react'
import { X, Key, CheckCircle2, XCircle, Loader2, Trash2 } from 'lucide-react'
import { useStore } from '../store'
import { checkAuth } from '../api'

export default function SettingsModal() {
  const showSettings = useStore((s) => s.showSettings)
  const setShowSettings = useStore((s) => s.setShowSettings)
  const apiKey = useStore((s) => s.apiKey)
  const setApiKey = useStore((s) => s.setApiKey)
  const history = useStore((s) => s.history)
  const clearHistory = useStore((s) => s.clearHistory)

  const [draft, setDraft] = useState(apiKey)
  const [testing, setTesting] = useState(false)
  const [testResult, setTestResult] = useState<boolean | null>(null)

  if (!showSettings) return null

  const handleTest = async () => {
    setTesting(true)
    setTestResult(null)
    const ok = await checkAuth(draft)
    setTestResult(ok)
    setTesting(false)
  }

  const handleSave = () => {
    setApiKey(draft.trim())
    setShowSettings(false)
    setTestResult(null)
  }

  const handleClose = () => {
    setDraft(apiKey)
    setShowSettings(false)
    setTestResult(null)
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={handleClose}
    >
      <div
        className="w-full max-w-md bg-[#141417] border border-[#27272a] rounded-xl shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-[#27272a]">
          <h2 className="text-sm font-semibold text-zinc-100">Settings</h2>
          <button
            onClick={handleClose}
            className="p-1 rounded-md hover:bg-white/5 text-zinc-500 hover:text-zinc-300"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Body */}
        <div className="p-5 space-y-5">
          {/* API Key */}
          <div className="space-y-2">
            <label className="flex items-center gap-1.5 text-xs font-medium text-zinc-400">
              <Key className="w-3 h-3" />
              API Key
            </label>
            <div className="flex gap-2">
              <input
                type="password"
                value={draft}
                onChange={(e) => {
                  setDraft(e.target.value)
                  setTestResult(null)
                }}
                placeholder="Enter your API key"
                className="flex-1 h-9 px-3 text-sm bg-[#0e0e11] border border-[#27272a] rounded-lg text-zinc-200 outline-none focus:border-blue-500/50 placeholder:text-zinc-600"
              />
              <button
                onClick={handleTest}
                disabled={!draft.trim() || testing}
                className="h-9 px-3 text-xs font-medium border border-[#27272a] rounded-lg text-zinc-400 hover:text-zinc-200 hover:bg-white/5 transition-colors disabled:opacity-40"
              >
                {testing ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : 'Test'}
              </button>
            </div>
            {testResult !== null && (
              <div
                className={`flex items-center gap-1.5 text-xs ${testResult ? 'text-emerald-400' : 'text-red-400'}`}
              >
                {testResult ? (
                  <CheckCircle2 className="w-3.5 h-3.5" />
                ) : (
                  <XCircle className="w-3.5 h-3.5" />
                )}
                {testResult ? 'Connection successful' : 'Authentication failed'}
              </div>
            )}
            <p className="text-[11px] text-zinc-600">
              Required when not behind IAP. Stored in your browser only.
            </p>
          </div>

          {/* History */}
          {history.length > 0 && (
            <div className="pt-3 border-t border-[#27272a]">
              <div className="flex items-center justify-between">
                <span className="text-xs text-zinc-500">
                  {history.length} transform{history.length !== 1 ? 's' : ''} in history
                </span>
                <button
                  onClick={clearHistory}
                  className="flex items-center gap-1 text-[11px] text-zinc-600 hover:text-red-400 transition-colors"
                >
                  <Trash2 className="w-3 h-3" />
                  Clear
                </button>
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex justify-end gap-2 px-5 py-4 border-t border-[#27272a]">
          <button
            onClick={handleClose}
            className="h-8 px-4 text-xs font-medium rounded-lg text-zinc-400 hover:text-zinc-200 hover:bg-white/5 transition-colors"
          >
            Cancel
          </button>
          <button
            onClick={handleSave}
            className="h-8 px-4 text-xs font-medium bg-blue-600 hover:bg-blue-500 text-white rounded-lg transition-colors"
          >
            Save
          </button>
        </div>
      </div>
    </div>
  )
}
