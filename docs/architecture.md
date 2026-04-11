# Architecture

## Goals

- Rust-first, deterministic engine
- Separate distributable engine workspace
- Vedic-primary model with Western tropical support
- Auditability before optimization
- No silent fallback in precision-sensitive paths

## Crate responsibilities

### `astro-core`

Owns canonical types, time/calendar utilities, astronomy primitives, error model, and ephemeris backend contracts.

### `astro-vedic`

Owns Vedic-domain computations such as sidereal sign decomposition, lagna modeling, and Vimshottari dasha sequencing.

### `astro-western`

Owns tropical Western-domain interpretation layers over shared core coordinates and house abstractions.

### `astro-api`

Owns the HTTP surface for product distribution and integration. API responses must always include computation metadata for auditability.

### `astro-napi`

Owns Node.js bindings using `napi-rs` for SDK distribution.

### `astro-wasm`

Owns WebAssembly bindings for browser and edge deployment targets.

## Determinism principles

- All input contracts are explicit and serializable.
- Timezone resolution errors are explicit for invalid, ambiguous, or nonexistent local times.
- Backend precision is never downgraded silently.
- Result metadata always includes engine mode, requested configuration, and version.
- Placeholder astronomy interfaces return typed errors or deterministic stub data during Phase 1.

## Vertical slice strategy

Phase 1 delivers a narrow but complete path:

1. Canonical request types
2. Deterministic UTC and Julian Day conversion
3. Backend trait for position, ayanamsa, and houses
4. Vedic derivations from sidereal longitude
5. API endpoints for health, positions, and dasha

## Accuracy and regression strategy

Planned regression sources:

- Swiss Ephemeris comparison vectors
- JPL Horizons comparison vectors

Planned CI rule:

- Placeholder manifests ship now.
- CI remains vector-aware.
- Once comparison vectors are added, regression failures must fail CI.

