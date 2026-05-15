# Sprint 4 — Chart sidereal load baseline (W4, min-instances=1)

Run **after** production deploy with `min-instances=1` ([reliability-min-instances.md](../runbooks/reliability-min-instances.md)). Same scenario as W2; only the warm-instance era differs.

## Run metadata

| Field | Value |
|-------|-------|
| Date | TBD |
| Cloud Run URL | `<redacted>` |
| minScale | `1` |
| Tool | `oha` or `vegeta` (via script) |
| Endpoint | `POST /chart/sidereal` |
| Scenario | 50 RPS for 5 minutes, rotating birth fixtures |
| Auth | `x-api-key: $ASTRO_API_KEY` |

## How to run

```bash
export ASTRO_API_BASE_URL="https://your-cloud-run-url"
export ASTRO_API_KEY="your-valid-key"
./scripts/load/baseline-chart-sidereal.sh
```

Record results in the table below (same script as [baseline-w2.md](baseline-w2.md)).

## Results

| Metric | Value |
|--------|-------|
| p50 latency | TBD |
| p95 latency | TBD |
| p99 latency | TBD |
| Error rate | TBD |
| 401 responses | TBD |
| 429 responses | TBD |

## Diff vs W2 (minScale=0 era)

| Metric | W2 (baseline-w2.md) | W4 (this doc) | Δ |
|--------|---------------------|---------------|---|
| p50 | TBD | TBD | TBD |
| p95 | TBD | TBD | TBD |
| p99 | TBD | TBD | TBD |
| Error rate | TBD | TBD | TBD |

**Cold-start note:** W2 includes scale-to-zero wake; W4 should show lower p95 tail if min=1 is effective. Paste W2 numbers into this section when [baseline-w2.md](baseline-w2.md) is filled.

## Targets

- Steady-state p95 &lt; **200 ms** on `POST /chart/sidereal`
- Launch guardrail: roll back webapp flip if p95 &gt; **800 ms** or error rate &gt; **2%**

Compare with `GET /metrics` (`astro_request_latency_ms_bucket`) using bearer `METRICS_TOKEN`.
