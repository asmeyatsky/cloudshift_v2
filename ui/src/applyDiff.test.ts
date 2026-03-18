import { describe, expect, it } from 'vitest'
import { applyDiff } from './applyDiff'

describe('applyDiff', () => {
  it('returns original when diff empty', () => {
    expect(applyDiff('a\nb', '')).toBe('a\nb')
  })

  it('applies single replacement hunk', () => {
    const orig = 'line1\nold\nline3'
    const diff = `--- a/x\n+++ b/x\n@@ -1,3 +1,3 @@\n line1\n-old\n+new\n line3\n`
    expect(applyDiff(orig, diff)).toBe('line1\nnew\nline3')
  })
})
