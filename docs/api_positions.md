# `/positions` JSON Contract

## Test strategy

Integration coverage uses Axum's in-process `oneshot` pattern rather than binding a real TCP socket. This keeps the contract test deterministic while still exercising the full HTTP route stack and JSON serialization.

## Request

`POST /positions`

```json
{
  "julian_day": 2451545.0,
  "bodies": ["moon", "sun", "mercury"],
  "compact": false
}
```

## Response shape

Stable top-level fields:

- `data`
- `metadata`

Stable `metadata` fields:

- `engine_mode`
- `ayanamsa_used`
- `house_system`
- `gravitational_deflection`
- `engine_semantic_version`
- `version`

Stable `data.positions[]` fields:

- `position`
- `computation_meta`

Stable `position` fields:

- `body`
- `longitude_deg`
- `latitude_deg`
- `distance_au`
- `frame`

Stable `computation_meta` fields:

- `frame`
- `observer`
- `topocentric_applied`
- `kernel`
- `kernel_notes`
- `crate_version`
- `light_time`
- `stellar_aberration`
- `gravitational_deflection`
- `motion_model`
- `node_policy`
- `ayanamsa_algorithm`

Example response:

```json
{
  "data": {
    "positions": [
      {
        "position": {
          "body": "moon",
          "longitude_deg": 223.323786,
          "latitude_deg": 5.1707422,
          "distance_au": 0.002689,
          "frame": "ecliptic_geocentric"
        },
        "computation_meta": {
          "frame": "apparent_ecliptic_of_date",
          "observer": "geocenter",
          "topocentric_applied": false,
          "kernel": "de440_moon",
          "kernel_notes": "Moon segment relative to the earth-moon barycenter plus EMB state",
          "crate_version": "0.1.0",
          "light_time": true,
          "stellar_aberration": true,
          "gravitational_deflection": true,
          "motion_model": null,
          "node_policy": "true",
          "ayanamsa_algorithm": null
        }
      }
    ]
  },
  "metadata": {
    "engine_mode": "vedic",
    "ayanamsa_used": "lahiri",
    "house_system": "whole_sign",
    "gravitational_deflection": true,
    "engine_semantic_version": "0.17.0",
    "version": "0.1.0"
  }
}
```

`compact = true` keeps the same route and position identity fields but omits heavy fields by returning `distance_au = null` and `computation_meta = null`.

Compact omission table:

| Field | Full response | `compact: true` |
| --- | --- | --- |
| `position.body` | present | present |
| `position.longitude_deg` | present | present |
| `position.latitude_deg` | present | present |
| `position.frame` | present | present |
| `position.distance_au` | present | `null` |
| `computation_meta` | present | `null` |

`motion_model` is optional computation metadata. It is reserved for cases where speed is emitted directly from a native ephemeris motion model. Current API speed fields are derived in the API layer, so `motion_model` is `null`.

## Versioning and breaking changes

This document describes the `v0.17.x` response contract.

Cross-route schema governance is defined in [docs/adr/0002-api-schema-versioning.md](/Users/sanjeet/Documents/Playground/docs/adr/0002-api-schema-versioning.md).

Breaking changes include:

- renaming any stable field listed above,
- removing a listed field,
- changing the meaning of a stable string enum value,
- changing the nesting shape of `data.positions[]`.

Non-breaking changes may add new fields, new supported bodies, or new documented enum values.

## Extensions

`/positions` does not currently expose `data.extensions`.

Policy:

- omission means there are no route-specific extension payloads,
- if extensions are added later, they must be additive-only,
- extension keys must be namespaced,
- recommended formats are reverse-DNS keys like `com.daanyam.preview` or URI-style keys like `https://daanyam.com/ext/preview`,
- extensions must never be required for core route functionality.

This intentional omission differs from `/chart/sidereal`, which returns `data.extensions: {}` today because that route has an explicit chart-schema envelope.
