# Deploying CloudShift to Cloud Run (emea-mas)

Deploys on every push to `main` (tests run first). Access is via IAP (Searce sign-in); the service is not publicly callable.

**GitHub secret:** `GCP_SA_KEY` = full JSON key for your existing CloudShift service account (the one that has Cloud Run Admin / deploy rights). Create key in GCP → IAM → Service accounts → your SA → Keys → Add key → JSON, then paste the entire file into the secret.
