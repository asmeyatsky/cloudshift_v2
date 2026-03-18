export interface TransformRequest {
  source: string
  language: string
  source_cloud?: string
  path_hint?: string
}

export interface PatternMatch {
  pattern_id: string[]
  span: {
    start_byte: number
    end_byte: number
    start_line: number
    end_line: number
    start_col: number
    end_col: number
  }
  confidence: number
  source_text: string
  replacement_text: string
  import_add: string[]
  import_remove: string[]
}

export interface Warning {
  message: string
  span: null | { start_line: number; end_line: number }
  severity: 'Error' | 'Warning' | 'Info'
}

export interface TransformResult {
  path: string
  language: string
  diff: string
  patterns: PatternMatch[]
  confidence: number
  warnings: Warning[]
  applied: boolean
}

export interface HistoryEntry {
  id: string
  timestamp: number
  code: string
  language: string
  sourceCloud: string
  result: TransformResult
}
