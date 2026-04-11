# `/positions/sidereal` JSON Contract

## Route purpose

`POST /positions/sidereal` computes tropical DE440 positions first, then derives Lahiri sidereal longitudes from the same resolved UTC instant.

The route accepts geolocation input for forward compatibility, but the current Phase 1 backend remains geocentric. The actual observer used for computation is always surfaced in each body's `computation_meta.observer`.

## Request

Stable request fields:

- `datetime`
- `geo`
- `ayanamsa`
- `bodies`
- `gravitational_deflection`
- `compact`
- `projection`

Example request:

```json
{
  "datetime": {
    "kind": "utc",
    "utc": "2000-01-01T12:00:00Z"
  },
  "geo": {
    "latitude_deg": 12.9716,
    "longitude_deg": 77.5946,
    "elevation_m": 920.0
  },
  "ayanamsa": "lahiri",
  "bodies": ["moon", "sun", "mercury"],
  "gravitational_deflection": false,
  "compact": false,
  "projection": "full"
}
```

Phase 1 supports `lahiri` only. Other ayanamsa values return a typed `unsupported ayanamsa` error response.

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

- `body`
- `tropical_longitude_deg`
- `tropical_latitude_deg`
- `sidereal_longitude_deg`
- `longitude_speed_deg_per_day`
- `retrograde`
- `distance_au`
- `moon_division`
- `computation_meta`

Stable `moon_division` fields when present:

- `rashi`
- `nakshatra`
- `pada`

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
        "body": "moon",
        "tropical_longitude_deg": 223.323786,
        "tropical_latitude_deg": 5.1707422,
        "sidereal_longitude_deg": 199.46019874126517,
        "longitude_speed_deg_per_day": 13.176358,
        "retrograde": false,
        "distance_au": 0.002689,
        "moon_division": {
          "rashi": "tula",
          "nakshatra": "swati",
          "pada": 4
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
          "gravitational_deflection": false,
          "motion_model": null,
          "node_policy": "true",
          "ayanamsa_algorithm": "lahiri_swe_zero_epoch_iau1976_v1"
        }
      }
    ]
  },
  "metadata": {
    "engine_mode": "vedic",
    "ayanamsa_used": "lahiri",
    "house_system": "whole_sign",
    "gravitational_deflection": false,
    "engine_semantic_version": "0.17.0",
    "version": "0.1.0"
  }
}
```

`longitude_speed_deg_per_day` is the geometric ecliptic longitude speed from the same apparent geocentric DE440 longitude pipeline, measured as signed degrees per day around the resolved UTC instant. `retrograde` is derived from the sign of that same speed: negative means retrograde; zero or positive means direct.

The seeded demo backend used in non-kernel tests is intentionally static, so seeded fixture responses may report `longitude_speed_deg_per_day = 0.0`. Real DE440-backed responses should show non-zero motion for moving bodies like the Moon.

`projection = "sidereal_only"` omits tropical-only coordinate fields by omitting `tropical_longitude_deg` and `tropical_latitude_deg` from the JSON payload, while keeping the sidereal position, motion, and Moon-division semantics unchanged.

If `compact = true`, `/positions/sidereal` keeps the same coordinate and motion identity fields but omits heavy fields: `distance_au`, `moon_division`, and `computation_meta`.

Compact omission table:

| Field | Full response | `compact: true` |
| --- | --- | --- |
| `body` | present | present |
| `tropical_longitude_deg` | present | omitted when `projection: "sidereal_only"` |
| `tropical_latitude_deg` | present | omitted when `projection: "sidereal_only"` |
| `sidereal_longitude_deg` | present | present |
| `longitude_speed_deg_per_day` | present | present |
| `retrograde` | present | present |
| `distance_au` | present | `null` |
| `moon_division` | present for Moon | `null` |
| `computation_meta` | present | `null` |

`motion_model` is optional computation metadata. It is reserved for cases where speed is emitted directly from a native ephemeris motion model. Current sidereal speed fields are derived in the API layer from apparent longitude differences, so `motion_model` is `null`.

## Versioning and breaking changes

This document describes the `v0.17.x` sidereal response contract.

Cross-route schema governance is defined in [docs/adr/0002-api-schema-versioning.md](/Users/sanjeet/Documents/Playground/docs/adr/0002-api-schema-versioning.md).

Breaking changes include:

- renaming any stable field listed above,
- removing a stable field,
- changing the meaning of `ayanamsa_algorithm`,
- changing the semantics of `moon_division`,
- changing sidereal outputs without bumping `engine_semantic_version`.

Non-breaking changes may add new fields, new supported bodies, or new ayanamsa implementations.

## Extensions

`/positions/sidereal` does not currently expose `data.extensions`.

Policy:

- omission means there are no route-specific extension payloads,
- if extensions are added later, they must be additive-only,
- extension keys must be namespaced,
- recommended formats are reverse-DNS keys like `com.daanyam.preview` or URI-style keys like `https://daanyam.com/ext/preview`,
- extensions must never be required for core route functionality.

This intentional omission differs from `/chart/sidereal`, which returns `data.extensions: {}` today because that route exposes an explicit chart-schema envelope and reserved extension surface.
