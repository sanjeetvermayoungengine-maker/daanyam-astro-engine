# Launch Week Monitoring Runbook

This runbook defines the `/metrics` scrape contract and the minimum launch watch checks for the public beta window.

For request-ID tracing, SLO breach logs, and Cloud Logging → BigQuery setup, see [observability.md](observability.md).

## Scrape Target

- Base URL: Cloud Run public URL for `astro-engine`
- Path: `/metrics`
- Method: `GET`
- Auth: `Authorization: Bearer $METRICS_TOKEN` (set `METRICS_TOKEN` on the Cloud Run service; route is public for API-key middleware but bearer-gated in the handler)
- Scrape interval: `30s`
- Scrape timeout: `10s`

Prometheus-style scrape config example:

```yaml
scrape_configs:
  - job_name: astro-engine
    scrape_interval: 30s
    scrape_timeout: 10s
    metrics_path: /metrics
    authorization:
      type: Bearer
      credentials: "${METRICS_TOKEN}"
    static_configs:
      - targets:
          - astro-engine-592048110971.asia-south1.run.app
```

## What To Watch During Launch

- `RPS`:
  - Metric: request throughput from Cloud Run request count and/or `astro_requests_total`.
  - Watch for sudden drops to near-zero during expected traffic windows.
- `p95 latency`:
  - Metric: Cloud Run request latency p95 and/or `astro_request_latency_ms_bucket`.
  - Trigger investigation if p95 is sustained above `1500 ms` for `10m`.
- `5xx rate`:
  - Metric: Cloud Run 5xx response class and/or `astro_requests_total` with `status` label `5xx` (numeric status codes are exported per response).
  - Trigger investigation if 5xx rate exceeds `1%` for `5m` or if absolute 5xx count spikes.
- `kernel_load_seconds`:
  - Metric: `astro_kernel_load_seconds` (startup/runtime metadata).
  - Track for regressions between revisions; unexpected increases can indicate kernel mount or IO regressions.

## Launch-Day Triage

1. Confirm `GET /health` is 200.
2. Confirm `/metrics` scrape freshness is < 2 minutes old.
3. If 5xx spikes:
   - Inspect Cloud Run revision logs by `jsonPayload.request_id` (inbound `X-Request-Id` from the webapp; see [observability.md](observability.md)).
   - Check if errors are concentrated on authenticated endpoints (`/chart/sidereal`, `/dasha`) versus public endpoints.
4. If p95 spikes without 5xx growth:
   - Check concurrent request load versus instance count ceiling.
   - Check recent revision rollout and cold-start behavior.
5. If `kernel_load_seconds` regresses after a deploy:
   - Verify runtime image and kernel asset path are unchanged.
   - Roll back to last known-good revision if startup degradation is user-visible.

## Recommended Dashboard Panels

- Request rate (1m + 5m views)
- p50/p95/p99 latency
- 4xx and 5xx split by path
- Container instance count and concurrency utilization
- `astro_kernel_load_seconds` per revision
