/** API may serialize pattern_id as a string or string[]. */
export function formatPatternIds(pattern_id: unknown): string {
  if (pattern_id == null) return ''
  if (Array.isArray(pattern_id)) return pattern_id.map(String).filter(Boolean).join(', ')
  return String(pattern_id)
}
