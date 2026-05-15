# Monitoring Runbook

This Phase 1 slice keeps monitoring setup explicit and reproducible for the Cloud Run production service.

**Related runbooks:** [observability.md](runbooks/observability.md) (logs, BQ, SLO), [oncall.md](runbooks/oncall.md) (paging, triage, rollback), [monitoring-console-steps.md](runbooks/monitoring-console-steps.md) (manual GCP steps).

## What this covers

- multi-region uptime check against `GET /health` (60s period)
- alert policy for uptime failure (2 consecutive failures)
- alert policy for Cloud Run 5xx count
- alert policy for Cloud Run p95 latency
- synthetic chart transaction monitor (script + CI golden test)
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

| Policy display name (default) | Condition |
| --- | --- |
| `<service> health (multi-region)` | `GET /health` every **60s** from `asia-southeast1`, `usa-iowa`, `europe-west1` |
| `<service> uptime failure` | Uptime check failed **2** consecutive evaluation periods |
| `<service> 5xx count` | More than **5** `5xx` responses in **5** minutes |
| `<service> p95 latency` | p95 &gt; **1500 ms** over **10** minutes |

Tune them after a few days of real traffic.

### Multi-region uptime

Cloud Run serves from **asia-south1**; synthetic probes use the nearest supported Monitoring regions (see [monitoring-console-steps.md](runbooks/monitoring-console-steps.md)):

- **asia-southeast1** (stand-in for asia-south1)
- **usa-iowa** (us-central1)
- **europe-west1**

Override with `UPTIME_REGIONS` when running `setup_monitoring.sh`.

### Synthetic chart monitor

- **Script:** [`scripts/monitoring/synthetic-chart-sidereal.sh`](../scripts/monitoring/synthetic-chart-sidereal.sh) — every **5 min** recommended (Cloud Scheduler or cron).
- **Fixture:** [`tests/golden/synthetic/delhi-1990-chart.json`](../tests/golden/synthetic/delhi-1990-chart.json)
- **Expected lagna:** `275.1573701670353°` (±1e-6°), rashi `makara` (DE440)
- **CI:** `cargo test -p astro-api --test synthetic_chart_golden`
- **Alerting:** alert on non-zero exit (Scheduler failure notification) or wire a log-based metric; URL uptime checks cannot assert JSON fields.

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
- `UPTIME_CHECK_PERIOD=120s` for a slower check cadence
- `UPTIME_REGIONS=asia-southeast1,usa-iowa,europe-west1`
- `UPTIME_CONSECUTIVE_FAILURES=2`
- `ERROR_COUNT_THRESHOLD=10`
- `LATENCY_THRESHOLD_MS=2000`

What the script does:

1. resolves the Cloud Run service URL or host
2. creates or updates an uptime check on `https://<host>/health`
3. creates or updates these alert policies:
   - `<service> health (multi-region)` uptime check
   - `<service> uptime failure` (2 consecutive failures)
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

4. confirms the multi-region uptime check (three probe regions) and alert policies exist
5. optional: run [`scripts/monitoring/synthetic-chart-sidereal.sh`](../scripts/monitoring/synthetic-chart-sidereal.sh) against staging

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
