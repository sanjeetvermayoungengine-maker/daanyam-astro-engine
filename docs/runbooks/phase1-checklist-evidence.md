# Phase 1 checklist — evidence

Honest status for Phase 1 engineering close. Update checkboxes when evidence is verified in prod or CI.

| Item | Status | Evidence |
|------|--------|----------|
| Cloud Run DE440 | ☑ | [deploy/cloudrun/service.yaml](../../deploy/cloudrun/service.yaml), [docs/deploy.md](../deploy.md) |
| API keys + rate limit | ☑ | `auth_middleware` / `RateLimiter` in [crates/astro-api/src/lib.rs](../../crates/astro-api/src/lib.rs) |
| `/metrics` + `METRICS_TOKEN` | ☑ | [docs/runbooks/observability.md](observability.md), Sprint 2 metrics route |
| Request ID + SLO breach | ☑ | [docs/runbooks/observability.md](observability.md) |
| BQ log sink | ☑ | [scripts/gcp/create-log-sink.sh](../../scripts/gcp/create-log-sink.sh) |
| min-instances=1 | ☑ | [docs/runbooks/reliability-min-instances.md](reliability-min-instances.md) |
| Multi-region uptime | ☑ | [docs/monitoring.md](../monitoring.md), `deploy/cloudrun/setup_monitoring.sh` |
| Synthetic chart monitor | ☑ | [scripts/monitoring/synthetic-chart-sidereal.sh](../../scripts/monitoring/synthetic-chart-sidereal.sh), golden lagna `275.1573701670353°` in [tests/golden/synthetic/delhi-1990-chart.json](../../tests/golden/synthetic/delhi-1990-chart.json) |
| Horizons station CI | ☑ | [.github/workflows/horizons.yml](../../.github/workflows/horizons.yml), [tests/golden/horizons_stations/](../../tests/golden/horizons_stations/) |
| Retrograde motion contract | ☑ | [crates/astro-core/src/motion.rs](../../crates/astro-core/src/motion.rs), `retrograde_motion_proptest` |
| Rich `/provenance` manifest | ☑ | `GET /provenance`, `ENGINE_SEMANTIC_VERSION = 0.18.0` |
| p95 load baseline (W2) | ☐ TBD | Run `./scripts/load/baseline-chart-sidereal.sh`, paste results into [docs/perf/baseline-w2.md](../perf/baseline-w2.md) |
| p95 load baseline (W4) | ☐ TBD | After min-instances deploy, run same script; paste into [docs/perf/baseline-w4.md](../perf/baseline-w4.md) |

## Verify commands

```bash
export ASTRO_EPHE_PATH=./ephe/de440.bsp
cargo test --workspace

# Provenance (local)
cargo run -p astro-api &
curl -sS http://127.0.0.1:3000/provenance | jq .
curl -sSI http://127.0.0.1:3000/provenance | grep -i cache-control

# Synthetic chart (prod monitor script)
export ASTRO_API_BASE_URL="https://your-cloud-run-url"
export ASTRO_API_KEY="your-key"
./scripts/monitoring/synthetic-chart-sidereal.sh

# Horizons station regression (CI parity)
cargo test -p astro-core outer_planet_stations -- --nocapture
```

## p95 baseline (fill when run)

1. Set `ASTRO_API_BASE_URL` and `ASTRO_API_KEY` per [docs/deploy.md](../deploy.md).
2. Run `./scripts/load/baseline-chart-sidereal.sh`.
3. Copy p50/p95/p99 and error rate into [baseline-w2.md](../perf/baseline-w2.md) and [baseline-w4.md](../perf/baseline-w4.md).
4. Query template: [docs/runbooks/queries/latency_p95.sql](queries/latency_p95.sql).
