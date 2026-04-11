# Daanyam Astro Engine

Daanyam Astro Engine is a standalone, Rust-first astrology computation workspace focused on deterministic, auditable outputs.

Phase 1 prioritizes:

- a production-grade workspace foundation,
- canonical domain contracts,
- deterministic time/calendar utilities,
- astronomy primitive interfaces,
- a backend abstraction for ephemeris providers,
- a minimum Vedic vertical slice,
- a minimum HTTP API slice,
- test-first quality gates and regression harness placeholders.

## Workspace layout

```text
crates/
  astro-api
  astro-core
  astro-napi
  astro-vedic
  astro-wasm
  astro-western
tests/
  golden
  regression
benches/
docs/
  adr/
```

## Phase 1 implementation order

1. Workspace bootstrap
2. Domain contracts
3. Time and calendar utilities
4. Astronomical primitives
5. Ephemeris backend abstraction
6. Vedic minimum vertical slice
7. API minimum vertical slice
8. Quality and regression gates

## Quality gates

Every checkpoint must pass:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo bench -p astro-core --no-run
```

PR automation lives in [`.github/workflows/ci.yml`](/Users/sanjeet/Desktop/daanyam%20-%20astro%20engine/.github/workflows/ci.yml). Main-branch delivery to GCP Cloud Run lives in [`.github/workflows/deploy.yml`](/Users/sanjeet/Desktop/daanyam%20-%20astro%20engine/.github/workflows/deploy.yml), with [cloudbuild.yaml](/Users/sanjeet/Desktop/daanyam%20-%20astro%20engine/cloudbuild.yaml) as the in-project fallback.

## Current status

This sprint delivers the workspace skeleton, initial contracts/utilities, the DE440 apparent-position pipeline for the current graha set, the first Lahiri sidereal slice, API routes, tests, and CI skeleton. Unsupported precision-sensitive features still fail explicitly rather than silently falling back.

## DE440 ephemeris file

Phase 1 Moon support expects the JPL DE440 kernel to live outside the repository and be referenced with `ASTRO_EPHE_PATH`.

Example:

```bash
mkdir -p /tmp/astro-ephe
curl -L https://ssd.jpl.nasa.gov/ftp/eph/planets/bsp/de440.bsp -o /tmp/astro-ephe/de440.bsp
export ASTRO_EPHE_PATH=/tmp/astro-ephe/de440.bsp
```

## Apparent-position pipeline

The current DE440 apparent-position slice is implemented and tested for Sun, Moon, Mercury, Venus, Mars, Jupiter, Saturn, Rahu, and Ketu.
The default engine setting is `gravitational_deflection = true`, matching the current Horizons regression baseline.

Pipeline stages:

1. Convert UTC Julian Day to TDB.
2. Evaluate DE440 type-2 Chebyshev state vectors.
3. Build barycentric target and Earth states.
4. Iterate down-leg light-time.
5. Apply stellar aberration from the observer velocity.
6. Optionally apply solar gravitational deflection when enabled in `EngineConfig`.
7. Precess from J2000 to mean equator-of-date.
8. Rotate into mean ecliptic-of-date and serialize geocentric longitude/latitude.

Regression fixtures in [tests/regression/manifest.json](/Users/sanjeet/Documents/Playground/tests/regression/manifest.json) use official JPL Horizons observer-table quantity `31` values. Earlier placeholder Moon expectations were replaced because they did not match that documented frame definition.

The dedicated sidereal API route documents its JSON contract in [docs/api_positions_sidereal.md](/Users/sanjeet/Documents/Playground/docs/api_positions_sidereal.md). Lahiri provenance and versioning rules are documented in [docs/astronomy.md](/Users/sanjeet/Documents/Playground/docs/astronomy.md) and [CHANGELOG.md](/Users/sanjeet/Documents/Playground/CHANGELOG.md).
The chart-oriented sidereal contract is documented in [docs/api_chart_sidereal.md](/Users/sanjeet/Documents/Playground/docs/api_chart_sidereal.md).
Compact mobile sample payloads live in [docs/examples/mobile_chart_compact.json](/Users/sanjeet/Documents/Playground/docs/examples/mobile_chart_compact.json), [docs/examples/mobile_chart_compact_sidereal_only.json](/Users/sanjeet/Documents/Playground/docs/examples/mobile_chart_compact_sidereal_only.json), [docs/examples/mobile_positions_compact.json](/Users/sanjeet/Documents/Playground/docs/examples/mobile_positions_compact.json), and [docs/examples/mobile_positions_sidereal_compact_sidereal_only.json](/Users/sanjeet/Documents/Playground/docs/examples/mobile_positions_sidereal_compact_sidereal_only.json).
The live OpenAPI 3.1 document is served from `/openapi.json`, and the release artifact is committed at [dist/openapi.json](/Users/sanjeet/Documents/Playground/dist/openapi.json).
Release candidate local run and packaging steps are documented in [RELEASE.md](/Users/sanjeet/Documents/Playground/RELEASE.md).
Container and readiness guidance are documented in [docs/deploy.md](/Users/sanjeet/Documents/Playground/docs/deploy.md).
Cross-route schema governance is documented in [docs/adr/0002-api-schema-versioning.md](/Users/sanjeet/Documents/Playground/docs/adr/0002-api-schema-versioning.md).
Accepted ADRs are indexed in [docs/adr/README.md](/Users/sanjeet/Documents/Playground/docs/adr/README.md).

For workspace test hygiene, the current DE440 kernel discovery helper remains a lightweight shared test module at [tests/support/de440_kernel.rs](/Users/sanjeet/Documents/Playground/tests/support/de440_kernel.rs) instead of a dedicated `test-support` crate. This keeps the refactor surface small until more shared test-only utilities justify a separate crate.
