# AWS comprehensive example — split by service

These files are extracted from [`../aws_comprehensive_example.py`](../aws_comprehensive_example.py) so you can:

- **Transform one service at a time** in CloudShift (web UI or CLI) for cleaner GCP-oriented output.
- **Batch the folder** in the UI or run `cloudshift transform ./aws_comprehensive_split --source aws`.

| File | AWS service |
|------|-------------|
| `s3_manager.py` | S3 |
| `dynamodb_manager.py` | DynamoDB |
| `lambda_manager.py` | Lambda |
| `sns_manager.py` | SNS |
| `sqs_manager.py` | SQS |
| `ec2_manager.py` | EC2 |
| `rds_manager.py` | RDS |
| `cloudwatch_manager.py` | CloudWatch + Logs |
| `apigateway_manager.py` | API Gateway |
| `stepfunctions_manager.py` | Step Functions |
| `iam_manager.py` | IAM |
| `secrets_manager_sample.py` | Secrets Manager (`SecretsManagerSample` class) |
| `kinesis_manager.py` | Kinesis |
| `elasticache_manager.py` | ElastiCache |
| `ecs_manager.py` | ECS |
| `eks_manager.py` | EKS |
| `ses_manager.py` | SES |
| `route53_manager.py` | Route 53 |
| `main_demo.py` | Wires managers together (optional; needs `PYTHONPATH=.` and AWS creds) |

The monolithic file in the parent folder is unchanged for regression / stress tests.
