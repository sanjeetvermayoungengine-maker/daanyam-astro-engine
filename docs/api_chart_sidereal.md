# `/chart/sidereal` JSON Contract

## Route purpose

`POST /chart/sidereal` returns the Phase 1 Parashari chart graha set in one auditable sidereal payload.

The route reuses the same DE440 tropical pipeline and Lahiri sidereal conversion used by `/positions/sidereal`; it does not introduce a duplicate astronomy path.

## Request

Stable request fields:

- `datetime`
- `geo`
- `ayanamsa`
- `gravitational_deflection`
- `as_of`
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
  "gravitational_deflection": false,
  "as_of": {
    "kind": "utc",
    "utc": "2000-01-01T12:00:00Z"
  },
  "compact": false,
  "projection": "full"
}
```

## Response shape

Stable top-level fields:

- `data`
- `metadata`

Stable `data` fields:

- `schema_version`
- `extensions`
- `summary`
- `grahas`
- `lagna`
- `houses`
- `house_system`
- `moon_sidereal_longitude_deg`
- `moon_nakshatra`
- `moon_pada`
- `dasha`
- `node_policy`
- `lahiri_algorithm`

`grahas[]` reuses the sidereal chart graha shape: tropical/sidereal coordinates, per-body computation metadata, plus `sidereal_rashi`, additive divisional placement fields such as `d3_rashi` and `d9_rashi`, `whole_sign_house`, and `house_context`.

The current chart route always returns these bodies in payload order:

- `sun`
- `moon`
- `mars`
- `mercury`
- `jupiter`
- `venus`
- `saturn`
- `rahu`
- `ketu`

Example response excerpt:

```json
{
  "data": {
    "schema_version": "chart_sidereal_v1",
    "extensions": {},
    "summary": {
      "moon_rashi": "tula",
      "lagna_rashi": "dhanu",
      "lagna_lord": "jupiter",
      "house_lords": ["jupiter", "saturn", "saturn", "jupiter", "mars", "venus", "mercury", "moon", "sun", "mercury", "venus", "mars"],
      "houses": [
        { "house": 1, "occupants": ["rahu"] },
        { "house": 2, "occupants": ["jupiter"] }
      ],
      "grahas_by_rashi": {
        "dhanu": ["sun"],
        "karka": ["moon"]
      },
      "dispositors": [
        { "body": "sun", "occupied_rashi": "dhanu", "dispositor": "jupiter" },
        { "body": "moon", "occupied_rashi": "karka", "dispositor": "moon" }
      ],
      "placement_table": [
        {
          "body": "sun",
          "sidereal_rashi": "dhanu",
          "d3_rashi": "mesha",
          "d9_rashi": "simha",
          "whole_sign_house": 10,
          "sign_lord": "jupiter",
          "house_context": {
            "whole_sign_house": 10,
            "house_lord": "jupiter"
          }
        }
      ],
      "motion": {
        "retrograde_bodies": ["saturn"],
        "fastest": {
          "body": "moon",
          "longitude_speed_deg_per_day": 13.2284
        }
      }
    },
    "lagna": {
      "rashi": "dhanu",
      "sidereal_longitude_deg": 252.4411
    },
    "houses": [
      {
        "house": 1,
        "rashi": "dhanu",
        "cusp_sidereal_longitude_deg": 240.0
      },
      {
        "house": 2,
        "rashi": "makara",
        "cusp_sidereal_longitude_deg": 270.0
      }
    ],
    "house_system": "whole_sign",
    "grahas": [
      {
        "body": "sun",
        "tropical_longitude_deg": 280.3689092,
        "tropical_latitude_deg": 0.0002381,
        "sidereal_longitude_deg": 256.5053219412652,
        "longitude_speed_deg_per_day": 1.0193,
        "sidereal_rashi": "dhanu",
        "d3_rashi": "mesha",
        "d9_rashi": "simha",
        "whole_sign_house": 10,
        "house_context": {
          "whole_sign_house": 10,
          "house_lord": "jupiter"
        },
        "retrograde": false,
        "distance_au": 0.98332768,
        "moon_division": null,
        "computation_meta": {
          "frame": "apparent_ecliptic_of_date",
          "observer": "geocenter",
          "topocentric_applied": false,
          "kernel": "de440_sun",
          "kernel_notes": "solar system barycentric Sun segment from DE440",
          "crate_version": "0.1.0",
          "light_time": true,
          "stellar_aberration": true,
          "gravitational_deflection": false,
          "node_policy": "true",
          "ayanamsa_algorithm": "lahiri_swe_zero_epoch_iau1976_v1"
        }
      }
    ],
    "moon_sidereal_longitude_deg": 199.46019874126517,
    "moon_nakshatra": "swati",
    "moon_pada": 4,
    "dasha": {
      "as_of_utc": "2000-01-01T12:00:00Z",
      "birth_nakshatra": "swati",
      "birth_pada": 4,
      "current": {
        "maha": {
          "lord": "rahu",
          "start": "2000-01-01T12:00:00Z",
          "end": "2017-12-27T12:00:00Z"
        },
        "antar": {
          "lord": "jupiter",
          "start": "2000-01-01T12:00:00Z",
          "end": "2001-12-31T12:00:00Z"
        },
        "pratyantar": {
          "lord": "saturn",
          "start": "2000-01-01T12:00:00Z",
          "end": "2000-03-22T12:00:00Z"
        }
      }
    },
    "node_policy": "true_node_mean_ecliptic_of_date",
    "lahiri_algorithm": "lahiri_swe_zero_epoch_iau1976_v1"
  },
  "metadata": {
    "engine_semantic_version": "0.17.0"
  }
}
```

`lagna.sidereal_longitude_deg` is the Lahiri sidereal ascendant longitude derived from the house pipeline's tropical ascendant reference longitude. `houses[]` uses a stable Whole Sign schema: 12 entries in house order, each carrying the house number, the house rashi, and the sidereal cusp reference longitude at the start of that sign.

Whole Sign house assignment for each graha is computed from sidereal sign indices using the chart lagna sign as house 1:
`whole_sign_house = ((graha_sign_index - lagna_sign_index + 12) % 12) + 1`.
`sidereal_rashi` is the graha's rashi derived from its sidereal longitude in the same Lahiri sidereal frame.
`d3_rashi` is the additive Drekkana (D3) rashi derived from the same sidereal longitude using the classical 1st/5th/9th sign sequence within each 10 degree segment.
`d9_rashi` is the additive Navamsa (D9) rashi derived from the same sidereal longitude. It is present in both full and compact chart responses.
`longitude_speed_deg_per_day` is the geometric ecliptic longitude speed from the same apparent geocentric DE440 longitude pipeline, measured as signed degrees per day around the resolved UTC instant. `retrograde` is derived from the sign of that same speed: negative means retrograde; zero or positive means direct.
The seeded demo backend used in non-kernel tests is intentionally static, so seeded fixture responses may report `longitude_speed_deg_per_day = 0.0`. Real DE440-backed responses should show non-zero motion for moving bodies like the Moon.

`dasha` is a compact chart summary derived deterministically from the chart Moon sidereal longitude and the resolved birth datetime. It returns the Moon's birth `nakshatra` and `pada`, plus the current Vimshottari `maha`, `antar`, and `pratyantar` periods as UTC ISO timestamps from the existing deterministic dasha engine. If `as_of` is omitted, `dasha.as_of_utc` defaults to the resolved birth datetime; if `as_of` is provided, the current dasha periods are selected for that explicit UTC instant.

`summary` is the compact whole-sign chart digest. `lagna_lord` is the classical ruler of `lagna_rashi`. `house_lords` is a 12-entry array aligned to houses 1..12, where each entry is the classical ruler of that house's whole-sign rashi. `summary.houses` is a 12-entry occupancy array aligned to houses 1..12, where each entry is `{ house, occupants }` and `occupants` is derived only from the existing `grahas[]` `body` and `whole_sign_house` fields. `summary.grahas_by_rashi` groups body ids by `grahas[].sidereal_rashi` and does not duplicate longitude data. `summary.dispositors` is a one-hop dispositor summary derived from each graha's occupied sidereal rashi using the same rashi-lord mapping already used for `house_lords`.
`summary.placement_table` is the canonical convenience view for at-a-glance placement rendering. It is derived only from the existing `grahas[]` payload, and `grahas[]` remains the canonical per-body source of truth for full chart detail. Additive divisional placement columns such as `d3_rashi` and `d9_rashi` extend `summary.placement_table` rather than introducing a parallel placement summary array.
Each graha `house_context` is also one-hop only: `house_lord` is the lord of the graha's occupied sidereal rashi, derived from the same sign-lord table as `house_lords`.
`summary.motion` is derived only from the existing `grahas[]` output. `retrograde_bodies` lists graha ids whose `retrograde` field is `true`. `fastest` is the graha with the highest absolute `longitude_speed_deg_per_day` among the chart grahas.

`projection = "sidereal_only"` omits tropical-only graha fields by omitting `grahas[].tropical_longitude_deg` and `grahas[].tropical_latitude_deg` from the JSON payload, while keeping all sidereal chart identity fields needed for whole-sign and dasha integrity.

If `compact = true`, the route keeps the same chart identity fields and `summary`, but omits heavy fields: `houses`, `dasha`, and per-graha `computation_meta`, `distance_au`, and `moon_division`.

Compact omission table:

| Field | Full response | `compact: true` |
| --- | --- | --- |
| `summary` | present | present |
| `summary.houses` | present | present |
| `summary.grahas_by_rashi` | present | present |
| `summary.dispositors` | present | present |
| `summary.placement_table` | present | present |
| `summary.motion` | present | present |
| `lagna` | present | present |
| `grahas[].tropical_longitude_deg` | present | omitted when `projection: "sidereal_only"` |
| `grahas[].tropical_latitude_deg` | present | omitted when `projection: "sidereal_only"` |
| `grahas[].sidereal_longitude_deg` | present | present |
| `grahas[].longitude_speed_deg_per_day` | present | present |
| `grahas[].sidereal_rashi` | present | present |
| `grahas[].d3_rashi` | present | present |
| `grahas[].d9_rashi` | present | present |
| `grahas[].whole_sign_house` | present | present |
| `grahas[].house_context` | present | present |
| `grahas[].retrograde` | present | present |
| `grahas[].distance_au` | present | `null` |
| `grahas[].moon_division` | present for Moon | `null` |
| `grahas[].computation_meta` | present | `null` |
| `houses` | present | `null` |
| `dasha` | present | `null` |

## Versioning

This document describes the `v0.17.x` chart response contract.

As of `engine_semantic_version = "0.17.0"`, `chart_sidereal_v1` remains additive-only and treats `summary.placement_table` as the canonical convenience placement view without changing the meaning of existing fields. The schema version does not bump because existing fields keep the same meaning.

`schema_version` is a stable chart-schema identifier. The current value is:

- `chart_sidereal_v1`

Additive-only rules for `chart_sidereal_v1`:

- adding new fields is allowed,
- adding new documented enum values is allowed,
- removing a field is not allowed without a `schema_version` bump,
- renaming a field is not allowed without a `schema_version` bump,
- incompatible nesting changes require both a `schema_version` bump and a `CHANGELOG.md` entry.

## Extensions policy

`data.extensions` is reserved for additive vendor- or project-specific fields.

Rules:

- `extensions` keys must be namespaced.
- Recommended key formats:
  - reverse-DNS, for example `com.daanyam.preview`
  - URI-style keys, for example `https://daanyam.com/ext/preview`
- `extensions` must never be required for core chart functionality.
- Core clients must be able to ignore `extensions` safely.

Current behavior:

- `chart_sidereal_v1` includes `data.extensions` as an empty object `{}` when no extension payloads are present.

### Governance

Cross-route schema governance is defined in:

- [docs/adr/README.md](/Users/sanjeet/Documents/Playground/docs/adr/README.md)
- [docs/adr/0002-api-schema-versioning.md](/Users/sanjeet/Documents/Playground/docs/adr/0002-api-schema-versioning.md)
- [docs/adr/0003-topocentric-observer-policy.md](/Users/sanjeet/Documents/Playground/docs/adr/0003-topocentric-observer-policy.md)

This chart route intentionally differs from `/positions` and `/positions/sidereal`:

- `/chart/sidereal` includes `data.extensions: {}` when unused,
- `/positions` and `/positions/sidereal` omit `data.extensions`, and omission means no route-specific extensions are present.

Breaking changes include:

- renaming stable fields listed above,
- changing the returned graha order,
- changing the Whole Sign `houses[]` ordering or interpretation,
- changing `node_policy` semantics,
- changing outputs without bumping `engine_semantic_version`.

## Future fields (not implemented)

The following names are reserved for future additive chart growth and are intentionally not implemented in `chart_sidereal_v1` yet:

- `ascendant_degree_sidereal`
- `ayanamsa`
- `ayanamsa_degrees`
- `cusps_sidereal`
- `mc_degree_sidereal`

Observer-related future work is planned in [docs/topocentric_plan.md](/Users/sanjeet/Documents/Playground/docs/topocentric_plan.md).
