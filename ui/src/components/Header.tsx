import { useEffect, useRef, useState } from 'react'
import { Settings, Cloud, Home, HelpCircle } from 'lucide-react'
import { useStore } from '../store'
import { runHomeTour, runWorkspaceTour } from '../tour/cloudshiftTour'

export default function Header() {
  const authVerified = useStore((s) => s.authVerified)
  const setShowSettings = useStore((s) => s.setShowSettings)
  const screen = useStore((s) => s.screen)
  const goHome = useStore((s) => s.goHome)
  const [tourOpen, setTourOpen] = useState(false)
  const tourWrapRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!tourOpen) return
    const close = (e: MouseEvent) => {
      if (tourWrapRef.current && !tourWrapRef.current.contains(e.target as Node)) {
        setTourOpen(false)
      }
    }
    document.addEventListener('click', close)
    return () => document.removeEventListener('click', close)
  }, [tourOpen])

  return (
    <header
      id="tour-header"
      className="h-11 flex items-center justify-between px-4 border-b border-[#1e1e22] bg-[#0e0e11] shrink-0"
    >
      <div className="flex items-center gap-2.5">
        <div className="w-6 h-6 rounded-md bg-gradient-to-br from-blue-500 to-violet-500 flex items-center justify-center">
          <Cloud className="w-3.5 h-3.5 text-white" />
        </div>
        <span className="font-semibold text-[13px] tracking-tight text-zinc-100">CloudShift</span>
        <span className="text-[10px] font-medium px-1.5 py-0.5 rounded-full bg-blue-500/10 text-blue-400 border border-blue-500/20">
          v2
        </span>
        {screen === 'workspace' && (
          <button
            type="button"
            onClick={() => goHome()}
            className="ml-2 flex items-center gap-1.5 h-7 px-2.5 rounded-md text-[11px] text-zinc-400 hover:text-zinc-200 hover:bg-white/5 border border-transparent hover:border-[#27272a] transition-colors"
            title="Back to menu"
          >
            <Home className="w-3.5 h-3.5" />
            Menu
          </button>
        )}
      </div>

      <div className="flex items-center gap-2 sm:gap-3">
        {authVerified === null && (
          <span className="text-[11px] text-zinc-600 hidden sm:inline">Checking API…</span>
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
            className="text-[11px] text-amber-400/90 hover:text-amber-300 underline-offset-2 hover:underline max-w-[140px] sm:max-w-none truncate"
          >
            Sign in — Settings
          </button>
        )}

        <div className="relative" ref={tourWrapRef}>
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation()
              setTourOpen((o) => !o)
            }}
            className="p-1.5 rounded-md hover:bg-white/5 transition-colors text-zinc-500 hover:text-zinc-300"
            title="Guided tour"
            aria-expanded={tourOpen}
            aria-haspopup="menu"
          >
            <HelpCircle className="w-4 h-4" />
          </button>
          {tourOpen && (
            <div
              role="menu"
              className="absolute right-0 top-full mt-1 py-1 min-w-[200px] rounded-lg border border-[#27272a] bg-[#141417] shadow-xl z-[100] text-left"
            >
              <button
                type="button"
                role="menuitem"
                className="w-full px-3 py-2 text-left text-xs text-zinc-300 hover:bg-white/5"
                onClick={() => {
                  setTourOpen(false)
                  if (screen !== 'home') {
                    goHome()
                    setTimeout(runHomeTour, 320)
                  } else {
                    runHomeTour()
                  }
                }}
              >
                Tour: Home menu
              </button>
              <button
                type="button"
                role="menuitem"
                disabled={screen !== 'workspace'}
                className="w-full px-3 py-2 text-left text-xs hover:bg-white/5 disabled:opacity-40 disabled:cursor-not-allowed text-zinc-300"
                onClick={() => {
                  setTourOpen(false)
                  runWorkspaceTour()
                }}
              >
                Tour: Editor workspace
              </button>
              <p className="px-3 py-1.5 text-[10px] text-zinc-600 border-t border-[#27272a]">
                Editor tour works from the workspace only.
              </p>
            </div>
          )}
        </div>

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
