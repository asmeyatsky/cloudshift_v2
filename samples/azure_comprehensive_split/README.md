# Azure comprehensive example — split by service

Extracted from [`../azure_comprehensive_example.py`](../azure_comprehensive_example.py) so you can transform **one service at a time** or batch the folder:

```bash
cloudshift transform samples/azure_comprehensive_split --source azure --dry-run
```

| File | Azure service |
|------|----------------|
| `blob_storage_manager.py` | Blob Storage |
| `cosmos_db_manager.py` | Cosmos DB |
| `azure_functions_manager.py` | Azure Functions |
| `service_bus_manager.py` | Service Bus |
| `event_grid_manager.py` | Event Grid |
| `virtual_machine_manager.py` | Virtual Machines |
| `sql_database_manager.py` | SQL Database |
| `key_vault_manager.py` | Key Vault |
| `application_insights_manager.py` | Application Insights |
| `resource_manager.py` | Resource groups |
| `azure_ad_manager.py` | Azure AD (Graph) |
| `queue_storage_manager.py` | Storage Queue |
| `table_storage_manager.py` | Table Storage |
| `cognitive_services_manager.py` | Cognitive Services |
| `app_service_manager.py` | App Service |
| `container_instances_manager.py` | Container Instances |
| `azure_monitor_manager.py` | Azure Monitor |
| `main_demo.py` | `main()` + `extended_main()` (needs `PYTHONPATH=.` and credentials) |

The monolithic file in the parent folder is unchanged for full-file tests.

**Run demo** (optional):

```bash
cd samples/azure_comprehensive_split && PYTHONPATH=. python3 main_demo.py
```

**Note:** `azure_monitor_manager.create_metric_alert` uses `self.subscription_id` (fixed vs. the monolithic file, where `subscription_id` was undefined in that method).
