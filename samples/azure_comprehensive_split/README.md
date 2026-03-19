# Azure comprehensive example — split by service

Extracted from [`../azure_comprehensive_example.py`](../azure_comprehensive_example.py) so you can transform **one service at a time** or batch the folder:

```bash
cloudshift transform samples/azure_comprehensive_split --source azure --dry-run
```

| File | Azure service |
|------|----------------|
| `blob_storage_manager.py` | Blob Storage (GCP analogue: `gcp_reference/gcs_storage_manager.py`) |
| `cosmos_db_manager.py` | Cosmos DB (GCP analogue: `gcp_reference/firestore_manager.py`) |
| `azure_functions_manager.py` | Azure Functions (GCP: `gcp_reference/azure_functions_gcp_manager.py`) |
| `service_bus_manager.py` | Service Bus (GCP: `gcp_reference/service_bus_pubsub_analogue.py`) |
| `event_grid_manager.py` | Event Grid |
| `virtual_machine_manager.py` | Virtual Machines (GCP: `gcp_reference/compute_engine_manager.py`) |
| `sql_database_manager.py` | SQL Database (GCP: `gcp_reference/cloud_sql_manager.py`) |
| `key_vault_manager.py` | Key Vault (GCP: `gcp_reference/secret_manager_manager.py`) |
| `application_insights_manager.py` | Application Insights (GCP: `gcp_reference/application_insights_logging_analogue.py`) |
| `resource_manager.py` | Resource groups (GCP: `gcp_reference/resource_manager_projects_analogue.py`) |
| `azure_ad_manager.py` | Azure AD (Graph) (GCP: `gcp_reference/azure_ad_gcp_iam_analogue.py`) |
| `queue_storage_manager.py` | Storage Queue |
| `table_storage_manager.py` | Table Storage (GCP: `gcp_reference/table_storage_firestore_analogue.py`) |
| `cognitive_services_manager.py` | Cognitive Services (GCP: `gcp_reference/cognitive_services_gcp_manager.py`) |
| `app_service_manager.py` | App Service (GCP: Cloud Run — `gcp_reference/lambda_cloud_run_manager.py`) |
| `container_instances_manager.py` | Container Instances (GCP: Cloud Run / Jobs — `gcp_reference/`) |
| `azure_monitor_manager.py` | Azure Monitor |
| `main_demo.py` | `main()` + `extended_main()` — runs steps only when env vars are set (see docstring) |

The monolithic file in the parent folder is unchanged for full-file tests.

**Run demo** (optional; skips missing services):

```bash
# from repo root
PYTHONPATH=samples/azure_comprehensive_split python3 samples/azure_comprehensive_split/main_demo.py
```

Set env vars for the services you want to exercise (e.g. Cosmos emulator:
`COSMOS_ENDPOINT`, `COSMOS_KEY`). No hardcoded secrets in source.

**Note:** `azure_monitor_manager.create_metric_alert` uses `self.subscription_id` (fixed vs. the monolithic file, where `subscription_id` was undefined in that method).
