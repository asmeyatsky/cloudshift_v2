import { Settings, Cloud } from 'lucide-react'
import { useStore } from '../store'

export default function Header() {
  const authVerified = useStore((s) => s.authVerified)
  const setShowSettings = useStore((s) => s.setShowSettings)

  return (
    <header className="h-11 flex items-center justify-between px-4 border-b border-[#1e1e22] bg-[#0e0e11] shrink-0">
      <div className="flex items-center gap-2.5">
        <div className="w-6 h-6 rounded-md bg-gradient-to-br from-blue-500 to-violet-500 flex items-center justify-center">
          <Cloud className="w-3.5 h-3.5 text-white" />
        </div>
        <span className="font-semibold text-[13px] tracking-tight text-zinc-100">CloudShift</span>
        <span className="text-[10px] font-medium px-1.5 py-0.5 rounded-full bg-blue-500/10 text-blue-400 border border-blue-500/20">
          v2
        </span>
      </div>

      <div className="flex items-center gap-3">
        {authVerified === null && (
          <span className="text-[11px] text-zinc-600">Checking API…</span>
        )}
        {authVerified === true && (
          <div className="flex items-center gap-1.5 text-[11px] text-emerald-400/80">
            <div className="w-1.5 h-1.5 rounded-full bg-emerald-400" />
            API ready
          </div>
        )}
        {authVerified === false && (
          <button
            type="button"
            onClick={() => setShowSettings(true)}
            className="text-[11px] text-amber-400/90 hover:text-amber-300 underline-offset-2 hover:underline"
          >
            Sign in — Settings (API key) or IAP
          </button>
        )}
        <button
          type="button"
          onClick={() => setShowSettings(true)}
          className="p-1.5 rounded-md hover:bg-white/5 transition-colors text-zinc-500 hover:text-zinc-300"
          title="Settings"
        >
          <Settings className="w-4 h-4" />
        </button>
      </div>
    </header>
  )
}
