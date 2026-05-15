# ADR 0004: Uranus, Neptune, and Pluto deferred from DE440 motion surface

## Status

Accepted (Sprint 5, Phase 1)

## Context

Horizons-vetted station-window regressions require stable `CelestialBody` / kernel targets, API body enums, and motion pipelines. The engine today supports Sun–Saturn plus Rahu/Ketu only (`crates/astro-core/src/contracts.rs`). Sprint 5 delivered Jupiter and Saturn station fixtures under `tests/golden/horizons_stations/` and CI job `horizons-regression`.

## Decision

Defer Uranus, Neptune, and Pluto until a dedicated epic adds:

1. `CelestialBody` and `KernelTarget` entries in `de440.rs`
2. OpenAPI / positions body enums and chart graha lists (additive only)
3. Thirty Horizons station fixtures (3 bodies × 10 instants, 2020–2030)

Track B is not in Sprint 5 scope.

## Consequences

- Issue [outer-planet-station-regression.md](../issues/outer-planet-station-regression.md) remains open for the outer three bodies only.
- Jupiter/Saturn station coverage is closed for supported grahas.
- No HTTP JSON schema version bump required for deferral.

## Target

Phase 2 epic: outer-planet kernel + Horizons station suite (estimate: one sprint after Phase 1 checklist).
