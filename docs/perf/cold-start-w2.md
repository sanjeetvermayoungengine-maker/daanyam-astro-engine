# Sprint 2 — Cold-start measurement (W2)

W2 baseline used `minScale: "0"`. Production now targets `minScale: "1"` ([deploy/cloudrun/service.yaml](../../deploy/cloudrun/service.yaml), [reliability-min-instances.md](../runbooks/reliability-min-instances.md)). This document records time-to-first-200 on `/health` after a deploy or scale-to-zero wake (min=0 era).

## Procedure

1. Deploy a new revision **or** wait until the service has scaled to zero (no active instances in Cloud Run metrics).
2. Resolve the service URL:

```bash
URL=$(gcloud run services describe astro-engine --region asia-south1 --format='value(status.url)')
```

3. Measure first successful health check (wall-clock from first request):

```bash
time curl -sS -o /dev/null -w 'http_code=%{http_code} time_total=%{time_total}s\n' "${URL}/health"
```

Repeat 3–5 times after separate scale-to-zero events for a spread.

## Sample

| Event | time_total (s) | http_code | Notes |
|-------|----------------|-----------|-------|
| TBD | TBD | 200 | Run after scale-to-zero or fresh deploy |

## Recommendation (Sprint 4)

Cold-start latency on the first `/health` after scale-to-zero is user-visible when traffic arrives in bursts. Sprint 4 should set **`min-instances=1`** and re-run the W2 load test into `docs/perf/baseline-w4.md` (see [AstroEngine_Sprint_Prompts.md](../../AstroEngine_Sprint_Prompts.md)). **Do not** enable `min-instances=1` in Sprint 2 — measure first, then justify the cost.
