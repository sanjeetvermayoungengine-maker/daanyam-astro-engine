# Perf Baseline W2 (Live Cloud Run)

- Date: 2026-04-30
- Target: `https://astro-engine-6ucc3jlvga-el.a.run.app/chart/sidereal`
- Command:

```bash
env -u NO_COLOR oha -z 5m -q 50 -m POST -D fixtures/birth.json \
  https://astro-engine-6ucc3jlvga-el.a.run.app/chart/sidereal
```

## Results

- p50: `40.6240 ms`
- p95: `56.0237 ms`
- p99: `67.8126 ms`
- Success rate (transport): `100.00%`
- Non-2xx response rate (HTTP): `100%` (`14998` responses were `401`)
- Deadline aborts: `3` (`~0.02%` of requests)

## Cold-start probe after fresh revision deploy

- Fresh revision: `astro-engine-00003-gfh`
- Probe command: `curl -sS -o /tmp/cold_health.out -w "%{http_code}" "$URL/health"`
- First 200 latency: `192 ms`

## Notes

- The live service currently requires API key auth on POST routes, so percentile numbers above reflect authenticated-path middleware + `401` responses without a key.
- Constraint preserved: min instances explicitly set to `0` for this probe.
