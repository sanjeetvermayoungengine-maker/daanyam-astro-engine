# Observability Runbook

End-to-end traceability from the webapp through Cloud Run structured logs and BigQuery. For Prometheus scrape and launch-week triage, see [launch-week-monitoring.md](launch-week-monitoring.md).

## Cloud Run service name

| Source | Service name |
|--------|----------------|
| Cloud Build default ([`cloudbuild.yaml`](../../cloudbuild.yaml)) | `astro-api` |
| Alternate manifest ([`deploy/cloud_run.yaml`](../../deploy/cloud_run.yaml)) | `astro-engine` |

Use the name that matches your deployed revision when creating log sinks and filters.

## Request ID contract

1. **Webapp** sends `X-Request-Id: <uuidv4>` (lowercase hyphenated UUID) on every engine HTTP call.
2. **Engine** accepts inbound `X-Request-Id` (case-insensitive; canonical casing recommended). If absent, it generates a UUID and still echoes it on the response.
3. **Response** includes the same value in the `x-request-id` header (HTTP header names are case-insensitive).
4. **Fallback headers:** `X-Correlation-Id` is promoted to `request_id` only when `X-Request-Id` is missing.

### Trace flow

```
PostHog event (request_id)
  → Engine call (X-Request-Id header)
  → Cloud Logging jsonPayload.request_id (api_usage + slo_breach lines)
  → BigQuery astro_engine_logs.run_googleapis_com_stdout_*
```

Cloud Logging filter for a single request:

```
jsonPayload.request_id="<uuid>"
```

## Structured log events

| message | When |
|---------|------|
| `api_usage` | Every request (latency, path, status, engine_version, kernel_hash) |
| `slo_breach` | Successful (2xx) `POST /chart/sidereal` over 200 ms or `POST /dasha` over 300 ms |

SLO breach filter:

```
resource.type="cloud_run_revision"
resource.labels.service_name="astro-api"
jsonPayload.message="slo_breach"
```

Example `slo_breach` line (stderr JSON):

```json
{
  "severity": "WARNING",
  "message": "slo_breach",
  "slo_breach": true,
  "path": "/chart/sidereal",
  "target_ms": 200,
  "actual_ms": 245,
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "engine_version": "0.17.2",
  "kernel_hash": "sha256:…"
}
```

## Prometheus metrics

Set `METRICS_TOKEN` on the Cloud Run service and scrape `GET /metrics` with `Authorization: Bearer $METRICS_TOKEN`. See [launch-week-monitoring.md](launch-week-monitoring.md) for scrape config and launch-day panels.

## Cloud Logging → BigQuery export

Export stdout/stderr JSON logs to BigQuery for SQL analysis. No Terraform is required; use `gcloud` or the Console.

### Prerequisites

- GCP project with Cloud Run service deployed (`astro-api` by default)
- `roles/logging.configWriter` (or equivalent) to create sinks
- `roles/bigquery.admin` or dataset-level permissions to create the dataset and grant the sink writer account

### 1. Create BigQuery dataset

```bash
export GCP_PROJECT="your-gcp-project-id"
export BQ_LOCATION="asia-south1"
export BQ_DATASET="astro_engine_logs"

gcloud bigquery datasets create "${BQ_DATASET}" \
  --project="${GCP_PROJECT}" \
  --location="${BQ_LOCATION}" \
  --description="Astro engine Cloud Run structured logs"
```

### 2. Create log sink

```bash
export CLOUD_RUN_SERVICE="astro-api"
export SINK_ID="astro-api-logs-to-bq"

gcloud logging sinks create "${SINK_ID}" \
  "bigquery.googleapis.com/projects/${GCP_PROJECT}/datasets/${BQ_DATASET}" \
  --project="${GCP_PROJECT}" \
  --log-filter='resource.type="cloud_run_revision"
resource.labels.service_name="'"${CLOUD_RUN_SERVICE}"'"' \
  --use-partitioned-tables
```

If the sink already exists, update the filter:

```bash
gcloud logging sinks update "${SINK_ID}" \
  --project="${GCP_PROJECT}" \
  --log-filter='resource.type="cloud_run_revision"
resource.labels.service_name="'"${CLOUD_RUN_SERVICE}"'"'
```

Note the **writer identity** from sink creation output:

```bash
gcloud logging sinks describe "${SINK_ID}" \
  --project="${GCP_PROJECT}" \
  --format='value(writerIdentity)'
```

### 3. Grant BigQuery Data Editor to sink writer

```bash
export SINK_WRITER="serviceAccount:...@gcp-sa-logging.iam.gserviceaccount.com"

gcloud projects add-iam-policy-binding "${GCP_PROJECT}" \
  --member="${SINK_WRITER}" \
  --role="roles/bigquery.dataEditor"
```

Or grant at dataset scope:

```bash
bq add-iam-policy-binding \
  --member="${SINK_WRITER}" \
  --role=roles/bigquery.dataEditor \
  "${GCP_PROJECT}:${BQ_DATASET}"
```

### 4. Resulting table names

Partitioned daily tables (wildcard in queries):

```
${GCP_PROJECT}.${BQ_DATASET}.run_googleapis_com_stdout_YYYYMMDD
```

Example full table prefix for queries:

```
your-gcp-project-id.astro_engine_logs.run_googleapis_com_stdout_*
```

Confirm field names (`jsonPayload.path`, `jsonPayload.latency_ms`, etc.) against one exported row after the sink is active.

### Optional idempotent script

```bash
export GCP_PROJECT="your-gcp-project-id"
./scripts/gcp/create-log-sink.sh
```

## BigQuery query templates

- [queries/latency_p95.sql](queries/latency_p95.sql) — p50/p95/p99 per path and `slo_breach` counts (last 24h)

## Perf baselines

Load-test results for `/chart/sidereal` live in [docs/perf/baseline-w2.md](../perf/baseline-w2.md). Fill p50/p95/p99 after running `scripts/load/baseline-chart-sidereal.sh` before Sprint 4 SLO tuning.
