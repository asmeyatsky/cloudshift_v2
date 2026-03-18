import { useState } from 'react'
import {
  ChevronDown,
  ChevronUp,
  AlertTriangle,
  AlertCircle,
  Info,
  Sparkles,
  ShieldCheck,
} from 'lucide-react'
import clsx from 'clsx'
import { useStore } from '../store'
import { Warning } from '../types'

const SEVERITY: Record<Warning['severity'], { icon: typeof AlertCircle; color: string; bg: string; border: string }> = {
  Error: { icon: AlertCircle, color: 'text-red-400', bg: 'bg-red-500/10', border: 'border-red-500/20' },
  Warning: { icon: AlertTriangle, color: 'text-amber-400', bg: 'bg-amber-500/10', border: 'border-amber-500/20' },
  Info: { icon: Info, color: 'text-blue-400', bg: 'bg-blue-500/10', border: 'border-blue-500/20' },
}

export default function InsightsBar() {
  const result = useStore((s) => s.result)
  const [expanded, setExpanded] = useState(false)

  if (!result) return null

  const { patterns, warnings, confidence } = result
  const pct = Math.round(confidence * 100)
  const confColor =
    confidence >= 0.8 ? 'text-emerald-400' : confidence >= 0.5 ? 'text-amber-400' : 'text-red-400'
  const confBg =
    confidence >= 0.8 ? 'bg-emerald-400' : confidence >= 0.5 ? 'bg-amber-400' : 'bg-red-400'

  return (
    <div className="border-t border-[#1e1e22] bg-[#0e0e11] shrink-0">
      {/* Summary row */}
      <button
        onClick={() => setExpanded(!expanded)}
        className="w-full flex items-center gap-4 px-4 py-2 hover:bg-white/[0.02] transition-colors"
      >
        <div className="flex items-center gap-1.5 text-xs text-zinc-400">
          <Sparkles className="w-3.5 h-3.5 text-violet-400" />
          <span className="font-medium">{patterns.length}</span>
          <span>{patterns.length === 1 ? 'pattern' : 'patterns'}</span>
        </div>

        {warnings.length > 0 && (
          <div className="flex items-center gap-1.5 text-xs text-zinc-400">
            <AlertTriangle className="w-3.5 h-3.5 text-amber-400" />
            <span className="font-medium">{warnings.length}</span>
            <span>{warnings.length === 1 ? 'warning' : 'warnings'}</span>
          </div>
        )}

        <div className="flex items-center gap-2 text-xs">
          <span className={clsx('font-medium', confColor)}>{pct}%</span>
          <div className="w-16 h-1.5 rounded-full bg-zinc-800 overflow-hidden">
            <div
              className={clsx('h-full rounded-full transition-all duration-500', confBg)}
              style={{ width: `${pct}%` }}
            />
          </div>
        </div>

        <div className="flex-1" />
        {expanded ? (
          <ChevronDown className="w-3.5 h-3.5 text-zinc-600" />
        ) : (
          <ChevronUp className="w-3.5 h-3.5 text-zinc-600" />
        )}
      </button>

      {/* Expanded details */}
      {expanded && (
        <div className="px-4 pb-4 grid grid-cols-2 gap-4 max-h-64 overflow-y-auto">
          {/* Patterns */}
          <div className="space-y-2">
            <h3 className="text-[11px] font-medium text-zinc-500 uppercase tracking-wider">
              Patterns matched
            </h3>
            {patterns.map((p, i) => (
              <div key={i} className="p-2.5 rounded-lg bg-[#141417] border border-[#222228]">
                <div className="flex items-center justify-between mb-1.5">
                  <span className="text-[11px] font-mono text-violet-400 truncate">
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
                <div className="text-[11px] font-mono space-y-0.5">
                  <div className="text-red-400/70 truncate">- {p.source_text}</div>
                  <div className="text-emerald-400/70 truncate">+ {p.replacement_text}</div>
                </div>
              </div>
            ))}
            {patterns.length === 0 && (
              <p className="text-xs text-zinc-600 italic">No patterns matched</p>
            )}
          </div>

          {/* Warnings */}
          <div className="space-y-2">
            <h3 className="text-[11px] font-medium text-zinc-500 uppercase tracking-wider">
              Warnings
            </h3>
            {warnings.map((w, i) => {
              const cfg = SEVERITY[w.severity]
              const Icon = cfg.icon
              return (
                <div
                  key={i}
                  className={clsx(
                    'p-2.5 rounded-lg border flex items-start gap-2',
                    cfg.bg,
                    cfg.border,
                  )}
                >
                  <Icon className={clsx('w-3.5 h-3.5 shrink-0 mt-0.5', cfg.color)} />
                  <span className="text-xs text-zinc-300">{w.message}</span>
                </div>
              )
            })}
            {warnings.length === 0 && (
              <div className="flex items-center gap-1.5 text-xs text-emerald-400/60">
                <ShieldCheck className="w-3.5 h-3.5" />
                No warnings
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  )
}
