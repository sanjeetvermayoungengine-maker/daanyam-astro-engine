# Cloud Run min-instances=1 (reliability)

Production should run with **one warm instance** to avoid cold-start p95 spikes. Do **not** set `min-instances=2` without product sign-off. Do not use Spot / preemptible for this service.

## Canonical deploy manifest

| File | Role |
| --- | --- |
| [`deploy/cloudrun/service.yaml`](../../deploy/cloudrun/service.yaml) | **Canonical** — used by [`.github/workflows/deploy.yml`](../../.github/workflows/deploy.yml) and [`cloudbuild.yaml`](../../cloudbuild.yaml) (`minScale: "1"`). |
| [`deploy/cloud_run.yaml`](../../deploy/cloud_run.yaml) | Alternate / hand-deploy template (`astro-engine` name, 2 CPU / 2Gi); aligned to `minScale: "1"` for parity. |

## Apply min-instances=1 (operator)

Replace `SERVICE`, `REGION`, and `PROJECT` before running. **Do not run in production without explicit approval.**

```bash
gcloud run services update astro-api \
  --project=PROJECT \
  --region=asia-south1 \
  --min-instances=1 \
  --max-instances=10 \
  --no-cpu-throttling
```

Or patch the Knative annotation on the next manifest deploy:

```yaml
autoscaling.knative.dev/minScale: "1"
autoscaling.knative.dev/maxScale: "10"
```

## Verify warm instance

After deploy, under steady `GET /health` traffic (or light authenticated chart traffic):

1. Scrape `GET /metrics` twice, 30s apart, with `Authorization: Bearer $METRICS_TOKEN`.
2. Confirm `astro_kernel_load_seconds` does **not** increment on every request after the first warm-up window.
3. Compare p95 on `POST /chart/sidereal` against [baseline-w4.md](../perf/baseline-w4.md) (min=1 era) vs [baseline-w2.md](../perf/baseline-w2.md) (min=0 era).

## Cost note

`min-instances=1` is approved for production stability. Rolling back to `minScale: "0"` is an emergency cost cut only — expect cold-start regression on p95 (see [cold-start-w2.md](../perf/cold-start-w2.md)).

## Related

- [deploy.md](../deploy.md) — env vars including `METRICS_TOKEN`
- [oncall.md](oncall.md) — rollback and escalation
