import { describe, expect, it } from 'vitest'
import { analyzeAwsPythonInput } from './detectComplexAwsInput'

describe('analyzeAwsPythonInput', () => {
  it('flags large multi-service file', () => {
    const services = [
      's3',
      'dynamodb',
      'lambda',
      'sns',
      'sqs',
      'ec2',
      'rds',
      'cloudwatch',
      'logs',
      'apigateway',
      'stepfunctions',
    ]
    const lines = Array(800).fill('# pad')
    const body = services.map((s) => `boto3.client('${s}')`).join('\n')
    const src = `${lines.join('\n')}\n${body}`
    const r = analyzeAwsPythonInput(src)
    expect(r.isHighRisk).toBe(true)
    expect(r.botoServices.length).toBeGreaterThanOrEqual(8)
  })

  it('small snippet is low risk', () => {
    const r = analyzeAwsPythonInput(`import boto3\nboto3.client('s3').put_object(Bucket='a', Key='k', Body=b'x')`)
    expect(r.isHighRisk).toBe(false)
    expect(r.isMediumRisk).toBe(false)
  })
})
