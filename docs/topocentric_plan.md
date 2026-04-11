# Topocentric Plan

This document describes the planned topocentric design surface for a future milestone. It does not imply that topocentric computation is implemented today.

## Required request fields

Topocentric requests will require:

- `latitude_deg`
- `longitude_deg`
- `elevation_m`

These fields already exist in the geolocation contract and will remain the canonical inputs.

## Earth model and observer assumptions

Planned baseline:

- ellipsoid: WGS84 unless a later ADR explicitly changes it,
- observer altitude: meters above the reference ellipsoid,
- geodetic latitude/longitude input,
- no silent fallback to a spherical Earth approximation.

## Refraction policy

Phase 1 and current sidereal routes remain airless/geometric at the observer level.

When topocentric support is added, the policy must be explicit:

- default: no atmospheric refraction,
- any refraction-enabled mode must be opt-in,
- `computation_meta` must state whether refraction was applied.

## `computation_meta` evolution

Current fields already reserve the observer-mode path:

- `observer = "geocenter"`
- `topocentric_applied = false`

Future topocentric implementation will evolve this contract additively:

- `observer = "topocentric"` when observer-dependent corrections are applied,
- `topocentric_applied = true`,
- additional additive fields may include observer model and refraction mode identifiers.

## Fixture layout proposal

Proposed future layout:

```text
tests/regression/topocentric/
  README.md
  topocentric_geocenter_baseline.json
  topocentric_observer_mumbai.json
```

Source of truth for filenames is [tests/regression/topocentric/README.md](/Users/sanjeet/Documents/Playground/tests/regression/topocentric/README.md).

Each future fixture group should include:

- source provenance,
- observer site coordinates,
- ellipsoid/model identifier,
- refraction policy,
- quantity/frame metadata,
- tolerances per metric.

## Testing / consistency

- the filename list in this document must match [tests/regression/topocentric/README.md](/Users/sanjeet/Documents/Playground/tests/regression/topocentric/README.md),
- this parity is enforced by `regression_docs_reference_future_plans` in [crates/astro-core/tests/regression_manifest.rs](/Users/sanjeet/Documents/Playground/crates/astro-core/tests/regression_manifest.rs).
