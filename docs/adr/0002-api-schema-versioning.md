# ADR 0002: API Schema Versioning

## Status

Accepted

## Context

Daanyam Astro Engine exposes multiple API payload families that evolve at different rates:

- `/positions`
- `/positions/sidereal`
- `/chart/sidereal`

We also expose astronomy-engine metadata that may change for legitimate numerical reasons without changing the transport shape.

## Decision

We separate transport-schema stability from engine-output stability.

### Route families

- `/positions`
  - stable JSON contract documented in [docs/api_positions.md](/Users/sanjeet/Documents/Playground/docs/api_positions.md)
  - currently governed by additive-only field rules and `engine_semantic_version`
- `/positions/sidereal`
  - stable JSON contract documented in [docs/api_positions_sidereal.md](/Users/sanjeet/Documents/Playground/docs/api_positions_sidereal.md)
  - currently governed by additive-only field rules and `engine_semantic_version`
- `/chart/sidereal`
  - explicit `schema_version`
  - current value: `chart_sidereal_v1`

### `schema_version`

- `schema_version` identifies the transport shape for a route family when the shape needs an explicit contract boundary.
- Removing a field requires a `schema_version` bump.
- Renaming a field requires a `schema_version` bump.
- Changing nesting in a non-additive way requires a `schema_version` bump.
- Additive fields do not require a `schema_version` bump.

### `engine_semantic_version`

- `engine_semantic_version` identifies the numerical/semantic output surface of the engine.
- It must change when an implementation update can move published results or alter metadata semantics in a meaningful way.
- A schema-stable but numerically different result is an `engine_semantic_version` change, not necessarily a `schema_version` change.

### CHANGELOG requirements

Every breaking or output-shifting change must be recorded in [CHANGELOG.md](/Users/sanjeet/Documents/Playground/CHANGELOG.md).

Required changelog coverage:

- `schema_version` bumps
- `engine_semantic_version` bumps
- renamed or removed public fields
- numerical changes that affect published fixtures or API outputs

## Consequences

- Clients can distinguish transport compatibility from astronomy-output compatibility.
- Additive route evolution remains cheap.
- Breaking changes are explicit and auditable.
