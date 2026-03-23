import type { TransformResult } from './types'

/** True if the server applied at least one pattern or produced different text than the input. */
export function transformHasEffect(result: TransformResult, sourceBefore: string): boolean {
  if (result.patterns.length > 0) return true
  if (result.diff?.trim().length) return true
  const ts = result.transformed_source
  if (ts != null && ts.length > 0 && ts !== sourceBefore) return true
  return false
}
