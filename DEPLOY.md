# Deploying CloudShift to Cloud Run (emea-mas)

Deploys on every push to `main` (tests run first). Access is via IAP (Searce sign-in); the service is not publicly callable.

**GitHub secret:** `GCP_SA_KEY` = full JSON key for your existing CloudShift service account (the one that has Cloud Run Admin / deploy rights). Create key in GCP → IAM → Service accounts → your SA → Keys → Add key → JSON, then paste the entire file into the secret.

**Gemini key (for the app at runtime):** In GitHub → Settings → Secrets and variables → Actions, add a repository secret named `GEMINI_API_KEY` with your Gemini API key. The workflow passes it to Cloud Run as env var `GEMINI_API_KEY`. If you don’t need it yet, add a dummy value or remove the `--set-env-vars "GEMINI_API_KEY=..."` line from the workflow.
