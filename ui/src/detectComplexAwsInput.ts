/**
 * Heuristic: large files with many boto3 services rarely produce coherent single-file GCP output.
 */
export type AwsComplexity = {
  botoServices: string[]
  lineCount: number
  /** Many services or very long file */
  isHighRisk: boolean
  /** Elevated but not extreme */
  isMediumRisk: boolean
}

const CLIENT = /boto3\.client\(\s*['"]([a-z0-9-]+)['"]/gi
const RESOURCE = /boto3\.resource\(\s*['"]([a-z0-9-]+)['"]/gi

export function analyzeAwsPythonInput(source: string): AwsComplexity {
  const services = new Set<string>()
  let m: RegExpExecArray | null
  const c = source.slice()
  CLIENT.lastIndex = 0
  while ((m = CLIENT.exec(c)) !== null) services.add(m[1])
  RESOURCE.lastIndex = 0
  while ((m = RESOURCE.exec(c)) !== null) services.add(m[1])

  const lineCount = source.split(/\r?\n/).length
  const n = services.size

  const isHighRisk = n >= 8 || lineCount >= 700 || (n >= 5 && lineCount >= 400)
  const isMediumRisk =
    !isHighRisk && (n >= 4 || lineCount >= 350 || (n >= 3 && lineCount >= 250))

  return {
    botoServices: [...services].sort(),
    lineCount,
    isHighRisk,
    isMediumRisk,
  }
}
