# Phase 1 retrospective

**Release:** v0.18.0 · **Branch:** `phase1/release-v0.18.0` · **Close date:** 2026-05

## What worked

- **Golden fixtures** — Horizons regression manifest, synthetic Delhi-1990 chart (`275.1573701670353°`), and station windows gave deterministic CI signal without live JPL calls in every test run.
- **DE440 kernel path** — `ASTRO_EPHE_PATH` + Cloud Run DE440 backend with coverage gate (2024–2150) kept prod and tests on the same ephemeris source.
- **Sprint prompts** — `AstroEngine_Sprint_Prompts.md` sequenced observability → reliability → Horizons → release without scope bleed.
- **Repo split** — `daanyam-astroengine` (compute + API) vs `daanyam-webapp` (UX) let engine ship `/provenance`, metrics, and contracts while webapp iterated on founder funnel.

## What broke / friction

- **Demo vs DE440 lagna** — Local `ASTRO_BACKEND=demo` produced different lagna than DE440-backed prod; confused early chart QA until examples pinned `ASTRO_EPHE_PATH`.
- **Baseline docs unfilled** — [baseline-w2.md](../perf/baseline-w2.md) and [baseline-w4.md](../perf/baseline-w4.md) still TBD; webapp flip gate lacked numeric p95 evidence.
- **Outer planets deferred** — Uranus/Neptune/Pluto station regressions parked in [ADR 0004](../adr/0004-outer-planets-deferred.md); Jupiter/Saturn only in Horizons CI.
- **Version drift** — `ENGINE_SEMANTIC_VERSION` lagged CHANGELOG until v0.18.0 alignment.

## Metrics (fill after load test)

| Metric | Target / note | Source |
|--------|---------------|--------|
| p95 `POST /chart/sidereal` | TBD ms | [baseline-w2.md](../perf/baseline-w2.md), `scripts/load/baseline-chart-sidereal.sh` |
| p95 `POST /dasha` | TBD ms | Same |
| Error rate (5xx) | TBD % | Cloud Monitoring / BQ `api_usage` |
| `slo_breach` rate | TBD / day | [observability.md](../runbooks/observability.md), BQ [latency_p95.sql](../runbooks/queries/latency_p95.sql) |

*Placeholder — run W2/W4 load scripts and paste numbers before Phase 2 webapp performance claims.*

## Process improvements for Phase 2

1. **Pin DE440 in all examples** — README, runbooks, and webapp snippets default to `ASTRO_EPHE_PATH` + `ASTRO_BACKEND=de440`; demo called out as non-audit.
2. **Single PR for version + changelog** — Bump `ENGINE_SEMANTIC_VERSION` and `CHANGELOG.md` in the same commit as any user-visible release.
3. **Webapp flip gate on baseline-w2** — Block `USE_ASTRO_API_V2` prod flip until p95 row filled in baseline-w2 with date + Cloud Run revision.
4. **Provenance in CI smoke** — Post-deploy verify curls `/provenance` and asserts `kernel_id=de440` + `engine_semantic_version`.
5. **Horizons tolerance in manifest** — `/provenance.tolerance_arcsec` documents station-suite (1e-7°) vs general regression (1e-9°) to reduce support confusion.

## Handoff

- **Phase 2 entry:** Sprint 7 in [AstroEngine_Sprint_Prompts.md](../../AstroEngine_Sprint_Prompts.md) (Raman ayanamsa / feature superiority).
- **Checklist:** [phase1-checklist-evidence.md](../runbooks/phase1-checklist-evidence.md).
