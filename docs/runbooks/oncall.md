# On-call runbook (astro-engine)

Primary service: Cloud Run `astro-api` (asia-south1). Cross-links: [monitoring.md](../monitoring.md), [observability.md](observability.md), [launch-week-monitoring.md](launch-week-monitoring.md), [cold-start-w2.md](../perf/cold-start-w2.md).

## Who gets paged

- **Notification channels:** set `NOTIFICATION_CHANNELS` when running [`setup_monitoring.sh`](../../deploy/cloudrun/setup_monitoring.sh) (comma-separated Monitoring channel resource names).
- **PagerDuty:** `projects/<PROJECT>/notificationChannels/<PAGERDUTY_CHANNEL_ID>` — replace with your service integration channel id.
- **PagerDuty service name (placeholder):** `Daanyam Astro Engine Production`

## Severity guide

| Signal | Typical cause | First action |
| --- | --- | --- |
| **Uptime fail** (`astro-api health (multi-region)`) | Bad deploy, DNS, SSL, instance crash | `GET /health` from laptop; check latest revision status |
| **5xx spike** | Kernel mount, OOM, panic | Cloud Logging errors; memory (2Gi limit on alternate manifest) |
| **p95 regression** | Cold start, load, DE440 IO | Compare [baseline-w4](../perf/baseline-w4.md); check `min-instances`; metrics histogram |
| **Synthetic chart mismatch** | Ephemeris drift, wrong backend, bad deploy | Run [`synthetic-chart-sidereal.sh`](../../scripts/monitoring/synthetic-chart-sidereal.sh); confirm `ASTRO_BACKEND=de440` |
| **`slo_breach` log spike** | Latency SLO (>200ms chart, >300ms dasha) | BQ/query templates in [observability.md](observability.md); sample by `request_id` |

## Triage

1. **Cloud Logging** (last 30m):

```bash
gcloud logging read \
  'resource.type="cloud_run_revision" AND resource.labels.service_name="astro-api"' \
  --project "$PROJECT_ID" \
  --freshness=30m \
  --limit=50
```

2. **Pivot on `request_id`** — present on every `api_usage` line and response header `x-request-id`.
3. **BigQuery** — log sink and latency SQL: [observability.md](observability.md), [queries/latency_p95.sql](queries/latency_p95.sql).
4. **Metrics** — `GET /metrics` with `Authorization: Bearer $METRICS_TOKEN` (see [launch-week-monitoring.md](launch-week-monitoring.md)).

## Common failures

| Symptom | Likely cause | Mitigation |
| --- | --- | --- |
| Startup crash / 503 on chart | GCS kernel mount fail, missing `de440.bsp` | Verify bucket mount / `ASTRO_EPHE_PATH`; redeploy previous image |
| OOM / restarts | Memory pressure (2Gi on `deploy/cloud_run.yaml` path) | Raise memory or reduce concurrency; check logs for OOM |
| 401 storm | Invalid or rotated API key | Client config; `VALID_API_KEYS` on revision |
| 429 storm | `RATE_LIMIT_RPM` per instance | Raise limit or scale; check abusive key prefix in logs |
| Synthetic lagna drift | `demo` backend or wrong kernel | Enforce `ASTRO_BACKEND=de440`; compare CI golden test |

## Rollback

1. **Cloud Run:** deploy previous good image digest / revision (GitHub Actions artifact or `gcloud run services update-traffic` to last revision).
2. **Emergency cost cut:** set `min-instances=0` only if approved — **expect p95 cold-start regression** ([reliability-min-instances.md](reliability-min-instances.md)).
3. **Webapp:** feature flag rollback is in **daanyam-webapp** — set `USE_ASTRO_API_V2=false` (not implemented in this repo).

## Escalation

- **`kernel_hash` mismatch across replicas** in `api_usage` logs → drain traffic, redeploy single known-good image; verify GCS fuse / kernel version.
- **Horizons / ephemeris regression** → file under [outer-planet-station-regression.md](../issues/outer-planet-station-regression.md); do not change chart JSON schema under pressure.

## Synthetic reference

- Script: [`scripts/monitoring/synthetic-chart-sidereal.sh`](../../scripts/monitoring/synthetic-chart-sidereal.sh)
- Expected lagna (DE440, Delhi 1990): **275.1573701670353°** (±1e-6°), rashi `makara`
- CI: `cargo test -p astro-api --test synthetic_chart_golden`
