# Sprint 2 — Chart sidereal load baseline (W2)

Baseline captured before / during the webapp `USE_ASTRO_API_V2` flag flip. **Do not commit production URLs or API keys.**

## Run metadata

| Field | Value |
|-------|-------|
| Date | TBD |
| Cloud Run URL | `<redacted>` |
| Tool | `oha` or `vegeta` (via script) |
| Endpoint | `POST /chart/sidereal` |
| Scenario | 50 RPS for 5 minutes, rotating birth fixtures |
| Auth | `x-api-key: $ASTRO_API_KEY` (see [deploy.md](../deploy.md)) |

## How to run

```bash
export ASTRO_API_BASE_URL="https://your-cloud-run-url"
export ASTRO_API_KEY="your-valid-key"
./scripts/load/baseline-chart-sidereal.sh
```

Fixtures rotate:

1. Delhi 1990-05-17 (production contract shape)
2. Bangalore 2000-01-01 (golden lahiri vector date)
3. Independence 1947-08-15 (golden vector date)
4. Bangalore 1995-08-12 (golden vector date)

## Results

| Metric | Value |
|--------|-------|
| p50 latency | TBD |
| p95 latency | TBD |
| p99 latency | TBD |
| Error rate | TBD |
| 401 responses | TBD (should be 0 with valid key) |
| 429 responses | TBD (note if `RATE_LIMIT_RPM` is low) |

## Targets

- **Steady-state goal:** p95 &lt; **200 ms** on `POST /chart/sidereal` (see [WEEK1_PUBLIC_LAUNCH.md](../../WEEK1_PUBLIC_LAUNCH.md)).
- **Launch guardrail:** roll back webapp flip if p95 &gt; **800 ms** or error rate &gt; **2%** (see [AstroEngine_Sprint_Prompts.md](../../AstroEngine_Sprint_Prompts.md) Sprint 3).

## Notes

- Run only against production after explicit approval.
- Compare with Cloud Run console p95 and `astro_request_latency_ms_bucket` from `GET /metrics` (bearer `METRICS_TOKEN`).
