/** Apply a unified diff to original source (fallback when server omits transformed_source). */
export function applyDiff(original: string, diff: string): string {
  if (!diff) return original

  const origLines = original.split('\n')
  const diffLines = diff.split('\n')
  const result: string[] = []
  let origIdx = 0

  for (const line of diffLines) {
    if (line.startsWith('@@')) {
      const m = line.match(/@@ -(\d+)/)
      if (m) {
        const start = parseInt(m[1], 10) - 1
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
