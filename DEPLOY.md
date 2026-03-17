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

The load balancer must send **all paths** (including `/api/*`) to the same Cloud Run backend. If the URL map has path rules that only match `/` or specific paths, requests to `/api/transform` or `/api/auth-check` can hit a different backend or no backend and return 404. In GCP Console: **Network services → Load balancing → your LB → URL map**. Ensure the default route (or a rule like `/*`) targets **cloudshift-backend** so that both the app and the API are served by the same Cloud Run service.
