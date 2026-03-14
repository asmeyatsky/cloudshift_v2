data "archive_file" "function_zip" {
  type        = "zip"
  source_dir  = "${path.module}/src"
  output_path = "${path.module}/function.zip"
}

resource "google_storage_bucket" "function_source" {
  name     = "${var.project_id}-function-source"
  location = var.region
  project  = var.project_id

  uniform_bucket_level_access = true
}

resource "google_storage_bucket_object" "function_archive" {
  name   = "document-processor-${data.archive_file.function_zip.output_md5}.zip"
  bucket = google_storage_bucket.function_source.name
  source = data.archive_file.function_zip.output_path
}

resource "google_cloudfunctions2_function" "processor" {
  name     = "document-processor"
  location = var.region
  project  = var.project_id

  build_config {
    runtime     = "python312"
    entry_point = "handle_gcs_event"
    source {
      storage_source {
        bucket = google_storage_bucket.function_source.name
        object = google_storage_bucket_object.function_archive.name
      }
    }
  }

  service_config {
    max_instance_count    = 100
    available_memory      = "512Mi"
    timeout_seconds       = 300
    service_account_email = google_service_account.function_sa.email

    environment_variables = {
      BUCKET_NAME = google_storage_bucket.data_lake.name
      LOG_LEVEL   = "INFO"
    }
  }

  event_trigger {
    trigger_region = var.region
    event_type     = "google.cloud.storage.object.v1.finalized"
    event_filters {
      attribute = "bucket"
      value     = google_storage_bucket.data_lake.name
    }
  }

  labels = {
    environment = "production"
  }
}
