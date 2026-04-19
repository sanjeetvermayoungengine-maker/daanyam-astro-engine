# Monitoring Runbook

This Phase 1 slice keeps monitoring setup explicit and reproducible for the Cloud Run production service.

## What this covers

- public uptime check against `GET /health`
- alert policy for uptime failure
- alert policy for Cloud Run 5xx count
- alert policy for Cloud Run p95 latency
- post-deploy verification steps

The repo already emits structured request logs with `request_id`, `body_hash`, and `latency_ms`. The monitoring setup here uses built-in Cloud Run metrics first, then relies on logs for incident triage.

## Prerequisites

- `gcloud` authenticated against the production project
- permissions to manage Monitoring resources:
  - `roles/monitoring.editor`
  - `roles/run.viewer` or broader Cloud Run read access
- at least one existing Monitoring notification channel

List notification channels:

```bash
gcloud beta monitoring channels list --project "$PROJECT_ID"
```

## Starter thresholds

These defaults are intentionally conservative starter values for first production rollout:

- uptime check every `1` minute against `/health`
- uptime failure alert wired to the created uptime check
- 5xx count alert when more than `5` responses occur in `5` minutes across the service
- p95 latency alert when p95 exceeds `1500 ms` over a `10` minute window

Tune them after a few days of real traffic.

## One-command setup

Use the helper below to create or update the uptime check and the three alert policies:

```bash
PROJECT_ID="your-gcp-project" \
SERVICE_NAME="astro-api" \
REGION="asia-south1" \
NOTIFICATION_CHANNELS="projects/your-gcp-project/notificationChannels/1234567890" \
bash deploy/cloudrun/setup_monitoring.sh
```

Useful overrides:

- `SERVICE_URL` if you want to derive the host from a specific deployed URL
- `SERVICE_HOST` if you want to target a custom domain directly
- `UPTIME_CHECK_PERIOD=5` for a slower check cadence
- `ERROR_COUNT_THRESHOLD=10`
- `LATENCY_THRESHOLD_MS=2000`

What the script does:

1. resolves the Cloud Run service URL or host
2. creates or updates an uptime check on `https://<host>/health`
3. creates or updates these alert policies:
   - `<service> uptime failure`
   - `<service> 5xx count`
   - `<service> p95 latency`

## Manual console mapping

If you prefer the Cloud Console over the helper script, create the same resources with these inputs:

- Uptime check:
  - target type: `URL`
  - protocol: `HTTPS`
  - host: deployed Cloud Run host or custom domain
  - path: `/health`
  - accepted status class: `2xx`
  - check frequency: `1 minute`
- Uptime alert policy:
  - source metric: `monitoring.googleapis.com/uptime_check/check_passed`
  - scope: the created uptime check ID
- 5xx alert policy:
  - resource type: `cloud_run_revision`
  - metric: `run.googleapis.com/request_count`
  - filter: `response_code_class=5xx`
  - alignment: `5 minutes`
  - aggregation: sum across revisions for the service
- p95 latency alert policy:
  - resource type: `cloud_run_revision`
  - metric: `run.googleapis.com/request_latencies`
  - alignment: `10 minutes`
  - aligner: `95th percentile`
  - reducer: max across revisions for the service

Cloud Run metrics come from the `cloud_run_revision` monitored resource and the `run.googleapis.com/request_count` and `run.googleapis.com/request_latencies` metric types. Uptime checks emit `monitoring.googleapis.com/uptime_check/check_passed`.

## Post-deploy verification

Run the repo helper after a production deploy:

```bash
PROJECT_ID="your-gcp-project" \
SERVICE_NAME="astro-api" \
REGION="asia-south1" \
bash deploy/cloudrun/post_deploy_verify.sh
```

That helper:

1. resolves the deployed Cloud Run URL
2. verifies `GET /health`
3. runs the existing deployed contract test:

```bash
ASTRO_API_BASE_URL="https://your-cloud-run-url" cargo test -p astro-api --test production_contract
```

4. confirms the uptime check and alert policies exist

## Incident triage

When an alert fires, start here:

```bash
gcloud logging read \
  'resource.type="cloud_run_revision" AND resource.labels.service_name="astro-api"' \
  --project "$PROJECT_ID" \
  --freshness=30m \
  --limit=50
```

Because each request log includes `request_id`, `body_hash`, `latency_ms`, method, path, and status, you can pivot quickly from alert to request-level investigation in Cloud Logging.

## Manual GCP steps that still remain

- create and verify the notification channel destinations you want paged
- confirm the Monitoring resources are created in the same project that owns the production Cloud Run service
- optionally pin alert routing, escalation, and on-call ownership outside this repo
