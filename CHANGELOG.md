# Changelog

## 0.17.2

### Changed

- Cloud Run `asia-south1` deployment is live at `https://astro-engine-6ucc3jlvga-el.a.run.app`.
- Added structured observability logging and a token-protected `/metrics` endpoint.
- Kernel-provenance assertions accept derived-from-DE440 values.
- Hardened `cloudbuild.yaml` with explicit `gcloud` entrypoint and fail-loud service-account render check.
- Rust toolchain pinned to `1.88`.

## 0.17.1

### Changed

- Cloud Run `asia-south1` deployment target aligned for `astro-engine` runtime settings.
- DE440 kernel loading configured for startup streaming from GCS (`gs://daanyam-astroengine-kernels/de440.bsp`) with cache in `/tmp`.
- Test: kernel-provenance assertion now accepts derived-from-DE440 values for nodes.

## 0.17.0

### Changed

- Added typed response round-trip helpers in `astro-napi` and `astro-wasm` for `POST /positions`, `POST /positions/sidereal`, and `POST /chart/sidereal`, backed by serde response structs so SDK bindings can consume HTTP JSON without hand-mapping fields.
- Declared `summary.placement_table` as the canonical convenience placement view for `/chart/sidereal`; additive at-a-glance placement columns should extend this table instead of introducing parallel placement arrays.
- Deferred tighter outer-planet station-window motion coverage until more JPL Horizons-vetted UTC instants are curated.

### Migration note

API consumers should treat `engine_semantic_version = "0.17.0"` as the boundary where non-HTTP SDK bindings gain typed response round-tripping and chart placement-table governance is explicitly frozen for beta. The chart schema version remains `chart_sidereal_v1` because these changes are additive and clarifying.

## 0.16.0

### Changed

- Added chart `summary.placement_table` as a convenience derived view of existing graha placements.
- Added tropical `/positions` request JSON helpers to `astro-napi` and `astro-wasm`.
- Deferred tighter outer-planet station-window speed bounds until more Horizons instants are curated.

### Migration note

API consumers should treat `engine_semantic_version = "0.16.0"` as the boundary where chart summaries gain a convenience placement table and SDK bindings cover all three request families with HTTP-shaped JSON helpers. The chart schema version remains `chart_sidereal_v1` because these changes are additive.

## 0.15.0

### Changed

- Added DE440-backed Jupiter and Saturn retrograde/direct integration regressions on kernel-available paths.
- Added per-graha `house_context` to `/chart/sidereal`, exposing one-hop house-lord pairing derived from occupied sidereal rashis.
- Added binding request JSON helpers in `astro-napi` and `astro-wasm` for sidereal chart and sidereal positions payload parity with HTTP.

### Migration note

API consumers should treat `engine_semantic_version = "0.15.0"` as the boundary where outer-planet motion regressions are covered in the DE440 path, chart grahas gain one-hop `house_context`, and non-HTTP SDK bindings expose HTTP-shaped request JSON helpers for the same compact/projection knobs. The chart schema version remains `chart_sidereal_v1` because these changes are additive.

## 0.14.0

### Changed

- Added DE440-backed Jupiter and Saturn retrograde/direct integration regressions on kernel-available paths.
- Added chart `summary.dispositors` as a one-hop dispositor summary derived from occupied sidereal rashis.
- Added minimal projection/compact request parity surfaces in `astro-napi` and `astro-wasm`.

### Migration note

API consumers should treat `engine_semantic_version = "0.14.0"` as the boundary where outer-planet motion regressions are covered in the DE440 path, chart summaries gain one-hop dispositors, and non-HTTP SDK bindings expose the same compact/projection request knobs as the HTTP surface. The chart schema version remains `chart_sidereal_v1` because these changes are additive.

## 0.13.0

### Changed

- Added `projection = "sidereal_only"` to `/positions/sidereal`, making sidereal projection rules symmetric with `/chart/sidereal`.
- Added chart `summary.grahas_by_rashi`, derived from existing graha sidereal sign assignments without duplicating coordinate data.
- Added a sidereal-only compact mobile example for `/positions/sidereal`.

### Migration note

API consumers should treat `engine_semantic_version = "0.13.0"` as the boundary where sidereal positions gain projection parity with chart payloads and chart summaries gain explicit graha grouping by rashi. The chart schema version remains `chart_sidereal_v1` because these changes are additive.

## 0.12.0

### Changed

- Added `summary.houses` to `/chart/sidereal`, exposing 12 whole-sign house occupancy entries derived from existing graha placements.
- Added chart request `projection = "sidereal_only"` to omit tropical-only graha coordinate fields while preserving sidereal chart integrity.
- Added a compact sidereal-only mobile chart example payload.

### Migration note

API consumers should treat `engine_semantic_version = "0.12.0"` as the boundary where `/chart/sidereal` gains explicit whole-sign occupancy summaries and chart projection can suppress tropical-only graha coordinates without changing `chart_sidereal_v1`. The schema version remains unchanged because the response change is additive.

## 0.11.0

### Changed

- Added `summary.motion` to `/chart/sidereal`, exposing `retrograde_bodies` and the fastest graha by absolute longitude speed.
- Added DE440-backed Venus and Mars retrograde regression tests on kernel-available paths.
- Added compact mobile example payloads for chart and positions responses.

### Migration note

API consumers should treat `engine_semantic_version = "0.11.0"` as the boundary where `/chart/sidereal` gains a compact motion summary block and mobile sample payloads are published alongside the live OpenAPI contract. `schema_version` remains `chart_sidereal_v1` because the response change is additive.

## 0.10.0

### Changed

- Added `compact` mode to `/positions`, mirroring `/positions/sidereal` by omitting heavy fields for lightweight clients.
- Extended the DE440-backed sidereal integration test with Sun and Mercury speed sanity windows alongside the Moon speed proof.
- Added optional `computation_meta.motion_model`, currently `null` for API-derived speed fields.

### Migration note

API consumers should treat `engine_semantic_version = "0.10.0"` as the boundary where both `/positions` and `/positions/sidereal` support compact responses and computation metadata explicitly reserves `motion_model` for future native-speed backends.

## 0.9.0

### Changed

- Added optional `compact` mode to `/positions/sidereal`, mirroring chart compact responses by omitting heavy fields.
- Documented seeded demo speed semantics explicitly and added a DE440-backed Moon speed sanity assertion.
- Extended the OpenAPI 3.1 document to include `/positions/sidereal` compact mode and motion-field semantics.

### Migration note

API consumers should treat `engine_semantic_version = "0.9.0"` as the boundary where `/positions/sidereal` gains compact-mode response shaping and motion-field semantics are explicitly stabilized across chart and sidereal payloads. `schema_version` remains `chart_sidereal_v1` because chart shape is unchanged in this increment.

## 0.8.0

### Changed

- Added `longitude_speed_deg_per_day` to `/positions/sidereal` and `/chart/sidereal` grahas, with `retrograde` derived from the same signed speed.
- Added chart `summary` with whole-sign lagna and house-lord mapping.
- Added optional chart request `compact` mode that omits heavy chart fields while preserving the same route and schema family.
- Extended the OpenAPI 3.1 document to include chart summary, speed, retrograde, dasha reproducibility, and compact-mode request fields.

### Migration note

API consumers should treat `engine_semantic_version = "0.8.0"` as the boundary where sidereal outputs gain explicit longitude speed auditability and `/chart/sidereal` gains a compact whole-sign summary plus optional compact response shaping. `schema_version` remains `chart_sidereal_v1` because these changes are additive.

## 0.7.0

### Changed

- Added `GET /openapi.json` serving an OpenAPI 3.1 schema for `/positions`, `/positions/sidereal`, and `/chart/sidereal`.
- Added per-body `retrograde` to sidereal positions and chart grahas.
- Added chart-request `as_of` and chart dasha `as_of_utc` for reproducible current-period selection.

### Migration note

API consumers should treat `engine_semantic_version = "0.7.0"` as the boundary where sidereal payloads gain explicit retrograde flags and `/chart/sidereal` dasha output becomes explicitly reproducible through `as_of` and `as_of_utc`. `schema_version` remains `chart_sidereal_v1` because these changes are additive.

## 0.6.0

### Changed

- Added compact chart-level `dasha` to `/chart/sidereal`, derived from Moon sidereal longitude and the resolved birth datetime.

### Migration note

API consumers should treat `engine_semantic_version = "0.6.0"` as the boundary where `/chart/sidereal` gains deterministic Vimshottari `maha`, `antar`, and `pratyantar` periods plus explicit birth nakshatra/pada in the chart payload. `schema_version` remains `chart_sidereal_v1` because the response change is additive.

## 0.5.0

### Changed

- Added per-graha `sidereal_rashi` and `whole_sign_house` to `/chart/sidereal`.

### Migration note

API consumers should treat `engine_semantic_version = "0.5.0"` as the boundary where `/chart/sidereal` graha entries gain explicit sidereal sign and Whole Sign house placement fields. `schema_version` remains `chart_sidereal_v1` because the response change is additive.

## 0.4.0

### Changed

- Added Lahiri sidereal `lagna`, `houses`, and `house_system` to `/chart/sidereal`.
- Defined the stable Whole Sign chart-house schema as 12 ordered house entries with sidereal cusp reference longitudes.

### Migration note

API consumers should treat `engine_semantic_version = "0.4.0"` as the boundary where `/chart/sidereal` expands from graha-only chart data to include Lagna and Whole Sign house payloads while keeping `schema_version = "chart_sidereal_v1"` unchanged because the new fields are additive.

## 0.3.0

### Changed

- Added `/chart/sidereal` as a chart-oriented Parashari graha payload.
- Extended `computation_meta` with `topocentric_applied`.
- Standardized Phase 1 observer metadata as explicit geocentric output.

### Migration note

API consumers should treat `engine_semantic_version = "0.3.0"` as the boundary where:

- chart payloads become available at `/chart/sidereal`, and
- `computation_meta` gains `topocentric_applied`.

## 0.2.0

### Changed

- Added an explicit `engine_semantic_version` metadata field for API responses.
- Upgraded the Lahiri implementation from the prior J2000-anchored linear approximation to `lahiri_swe_zero_epoch_iau1976_v1`.
- Added a dedicated `/positions/sidereal` API surface with auditable per-body metadata.

### Lahiri fixture change summary

The Lahiri golden fixtures changed because the implementation now uses:

- the Swiss Ephemeris Lahiri zero epoch of `285-03-22 17:54:02 TT`, and
- the IAU 1976 precession-in-longitude polynomial between that epoch and the requested TDB instant.

Before/after fixture summary:

| UTC | Old ayanamsa | New ayanamsa | Old sidereal Moon lon | New sidereal Moon lon |
| --- | --- | --- | --- | --- |
| 1900-01-01T12:00:00Z | 22.456122539 | 22.46696099991632 | 257.160196361 | 257.1493579000837 |
| 1947-08-15T00:00:00Z | 23.121295051 | 23.131910418629626 | 97.444453849 | 97.43383848137037 |
| 1995-08-12T09:00:00Z | 23.791740879 | 23.802273061738678 | 316.968856221 | 316.9583240382613 |
| 2000-01-01T12:00:00Z | 23.853055584 | 23.863587258734846 | 199.470730416 | 199.46019874126517 |
| 2024-04-08T18:00:00Z | 24.192086460 | 24.202636795562203 | 354.991128240 | 354.9805779044378 |

### Migration note

API consumers that persist or compare Lahiri sidereal longitudes should treat `engine_semantic_version = "0.2.0"` and `ayanamsa_algorithm = "lahiri_swe_zero_epoch_iau1976_v1"` as a numerical-output change boundary.
