# Deploying CloudShift to Cloud Run (emea-mas)

**Live URL (direct):** https://cloudshift-cux4sclfpq-ew.a.run.app  
**IAP URL (Searce sign-in):** https://cloudshift.poc-searce.com

Deploys on every push to `main`. (tests run first). Access is via IAP (Searce sign-in); the service is not publicly callable.

**GitHub secret:** `GCP_SA_KEY` = full JSON key for your existing CloudShift service account (the one that has Cloud Run Admin / deploy rights). Create key in GCP → IAM → Service accounts → your SA → Keys → Add key → JSON, then paste the entire file into the secret.

**Secrets (for the app at runtime):** In the repo on GitHub, go to Settings → Secrets and variables → Actions and add:

- `GEMINI_API_KEY` — your Gemini API key (or a dummy value if unused).
- `CLOUDSHIFT_API_KEY` — the API key for direct Cloud Run URL auth. Must match the value you enter in the app’s Settings. Do not use `##` in this value (used as delimiter in deploy).

The workflow passes both to Cloud Run in a single `--set-env-vars` (repeated flags replace all vars, so we use one flag with a `##` delimiter).

### Repoint cloudshift.poc-searce.com from v1 to v2

The load balancer host **cloudshift.poc-searce.com** may still point at the v1 Cloud Run service (us-central1). To send traffic to v2 (europe-west1), run once (from a machine with `gcloud` auth to **emea-mas**):

```bash
chmod +x deploy/gcp/repoint-lb-to-v2.sh
./deploy/gcp/repoint-lb-to-v2.sh
```

This creates a NEG in europe-west1 for the **cloudshift** service, removes the us-central1 NEG from **cloudshift-backend**, and attaches the new NEG. After that, https://cloudshift.poc-searce.com/ serves v2.

### If you get 404 on /api/transform or /api/auth-check via the LB URL

The load balancer must send **all paths** (including `/api/*`) to the same Cloud Run backend. If the URL map’s default service is not **cloudshift-backend**, or path matchers send `/api/*` elsewhere, you get 404.

**Option A — GCP Console**  
**Network services → Load balancing** → open the load balancer for cloudshift.poc-searce.com → **URL map**. Set the **default backend** (and any path matcher default) to **cloudshift-backend** so `/`, `/api/transform`, and `/api/auth-check` all go to Cloud Run.

**Option B — gcloud** (project **emea-mas**, global LB):

```bash
# 1. List URL maps and find the one used by your LB (e.g. cloudshift-lb or similar)
gcloud compute url-maps list --global --project=emea-mas

# 2. Inspect it (replace URL_MAP_NAME with the name from step 1)
gcloud compute url-maps describe URL_MAP_NAME --global --project=emea-mas

# 3. Set default backend to cloudshift-backend so all paths (including /api/*) go to Cloud Run
gcloud compute url-maps set-default-service URL_MAP_NAME \
  --default-service=cloudshift-backend \
  --global \
  --project=emea-mas
```

If the URL map has **path matchers** that send `/api` or `/api/*` to a different backend, remove or change those so the default (or the relevant matcher) is **cloudshift-backend**.
