import { describe, expect, it } from 'vitest'
import { transformHasEffect } from './transformOutcome'
import type { TransformResult } from './types'

function base(over: Partial<TransformResult>): TransformResult {
  return {
    path: 'x.py',
    language: 'python',
    diff: '',
    patterns: [],
    confidence: 1,
    warnings: [],
    applied: false,
    ...over,
  }
}

describe('transformHasEffect', () => {
  it('is false when no patterns and source equals transformed_source', () => {
    const src = 'import foo\n'
    expect(transformHasEffect(base({ transformed_source: src }), src)).toBe(false)
  })

  it('is true when patterns present', () => {
    const src = 'x'
    expect(
      transformHasEffect(
        base({
          patterns: [
            {
              pattern_id: 'p',
              span: {
                start_byte: 0,
                end_byte: 1,
                start_line: 1,
                end_line: 1,
                start_col: 0,
                end_col: 1,
              },
              confidence: 0.9,
              source_text: 'x',
              replacement_text: 'y',
              import_add: [],
              import_remove: [],
            },
          ],
        }),
        src,
      ),
    ).toBe(true)
  })

  it('is true when diff non-empty', () => {
    expect(transformHasEffect(base({ diff: '@@ -1 +1 @@\n-x\n+y\n' }), 'x')).toBe(true)
  })

  it('is true when transformed text differs', () => {
    expect(transformHasEffect(base({ transformed_source: 'b' }), 'a')).toBe(true)
  })
})
