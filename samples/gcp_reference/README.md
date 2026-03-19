# GCP reference snippets

Hand-written targets for large AWS samples where automated transform is weak or unsafe.

See **`docs/AWS_GCP_TRANSFORM.md`** for how patterns + LLM fallback fit together and what is (not) guaranteed.

| File | AWS analogue |
|------|----------------|
| `gcs_storage_manager.py` | `aws_comprehensive_split/s3_manager.py` (S3 → GCS) |
| `gcp_monitoring_manager.py` | `azure_comprehensive_split/azure_monitor_manager.py` (Monitor → Cloud Monitoring) |
| `application_insights_logging_analogue.py` | `azure_comprehensive_split/application_insights_manager.py` (App Insights → Cloud Logging) |
| `compute_engine_manager.py` | EC2 / **Azure VM** → Compute Engine (`ec2_manager.py`, `azure_comprehensive_split/virtual_machine_manager.py`) |
| `gke_cluster_manager.py` | `aws_comprehensive_split/eks_manager.py` (EKS → GKE) |
| `api_gateway_manager.py` | `aws_comprehensive_split/apigateway_manager.py` (REST API → API Gateway + OpenAPI) |
| `firestore_manager.py` | DynamoDB / **Azure Cosmos DB** → Firestore (`dynamodb_manager.py`, `azure_comprehensive_split/cosmos_db_manager.py`) |
| `table_storage_firestore_analogue.py` | `azure_comprehensive_split/table_storage_manager.py` (Table Storage → Firestore collections) |
| `memorystore_redis_manager.py` | `aws_comprehensive_split/elasticache_manager.py` (ElastiCache → Memorystore Redis) |
| `kinesis_pubsub_manager.py` | `aws_comprehensive_split/kinesis_manager.py` (Kinesis → Pub/Sub) |
| `cloud_run_jobs_manager.py` | ECS Fargate / **Azure Container Instances** → Cloud Run Jobs (`ecs_manager.py`, `azure_comprehensive_split/container_instances_manager.py`) |
| `sns_pubsub_manager.py` | `aws_comprehensive_split/sns_manager.py` (SNS → Pub/Sub) |
| `service_bus_pubsub_analogue.py` | `azure_comprehensive_split/service_bus_manager.py` (Service Bus → Pub/Sub) |
| `cloud_dns_manager.py` | `aws_comprehensive_split/route53_manager.py` (Route 53 → Cloud DNS) |
| `sqs_pubsub_manager.py` | `aws_comprehensive_split/sqs_manager.py` (SQS → Pub/Sub topic + pull sub) |
| `cloud_tasks_queue_manager.py` | `azure_comprehensive_split/queue_storage_manager.py` (Storage Queue → Cloud Tasks HTTP enqueue) |
| `eventgrid_pubsub_manager.py` | `azure_comprehensive_split/event_grid_manager.py` (Event Grid topic → Pub/Sub; routing → Eventarc) |
| `lambda_cloud_run_manager.py` | Lambda / **Azure App Service** → Cloud Run (`lambda_manager.py`, `azure_comprehensive_split/app_service_manager.py`) |
| `workflows_manager.py` | `aws_comprehensive_split/stepfunctions_manager.py` (Step Functions → Workflows) |
| `cloud_sql_manager.py` | RDS / **Azure SQL Database** → Cloud SQL (`rds_manager.py`, `azure_comprehensive_split/sql_database_manager.py`) |
| `secret_manager_manager.py` | `azure_comprehensive_split/key_vault_manager.py` (Key Vault → Secret Manager) |
| `cognitive_services_gcp_manager.py` | `azure_comprehensive_split/cognitive_services_manager.py` (Text Analytics / Vision → NL + Vision API) |
| `azure_functions_gcp_manager.py` | `azure_comprehensive_split/azure_functions_manager.py` (HTTP invoke + handler sample) |
| `resource_manager_projects_analogue.py` | `azure_comprehensive_split/resource_manager.py` (RG → projects / Resource Manager) |
| `azure_ad_gcp_iam_analogue.py` | `azure_comprehensive_split/azure_ad_manager.py` (SP → service account; users → Workspace/Identity) |

Install (pick what you use):  
`pip install google-cloud-storage google-cloud-resource-manager google-cloud-tasks google-cloud-workflows google-api-python-client google-auth-httplib2 google-cloud-compute google-cloud-container google-cloud-apigateway google-cloud-firestore google-cloud-redis google-cloud-pubsub google-cloud-run google-cloud-dns google-cloud-secret-manager google-cloud-language google-cloud-vision google-cloud-logging google-auth requests functions-framework google-api-core`

Auth: Application Default Credentials (`gcloud auth application-default login`).

**Orchestrator:** `main_demo.py` — same narrative as `aws_comprehensive_split/main_demo.py` (GCS → Firestore → Pub/Sub ×2 → logging; extended: Secret Manager). Needs `GOOGLE_CLOUD_PROJECT`, `GCS_BUCKET`.
