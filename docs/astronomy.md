# Astronomy Notes

## DE440 body policy

The current DE440-backed apparent-position pipeline supports:

- Sun
- Moon
- Mercury
- Venus
- Mars
- Jupiter
- Saturn
- Rahu
- Ketu

For Venus, Mars, Jupiter, and Saturn, Phase 1 currently uses the corresponding DE440 planetary barycenter segments exposed by the kernel summary records. This is surfaced explicitly in `computation_meta.kernel`; there is no silent fallback.

## Node policy

The default node policy is `true`, matching the current `EngineConfig` default.

Rahu and Ketu are currently derived from the Moon's geocentric state by computing the instantaneous orbital plane intersection with the mean ecliptic-of-date:

- Rahu = ascending true node longitude
- Ketu = Rahu + 180 degrees

These derived node values are reported with:

- `computation_meta.kernel = "derived_true_node_from_de440_moon"`
- `computation_meta.node_policy = "true"`
- `computation_meta.frame = "mean_ecliptic_of_date"`
- chart payload `node_policy = "true_node_mean_ecliptic_of_date"`

Observer metadata remains explicitly geocentric for Phase 1:

- `computation_meta.observer = "geocenter"`
- `computation_meta.topocentric_applied = false`

## Lahiri sidereal slice

The Lahiri sidereal functions use the same instant as the tropical pipeline input: TDB Julian Day derived from the requested UTC instant.

Algorithm ID:

- `lahiri_swe_zero_epoch_iau1976_v1`

Documented reference approach:

- Lahiri zero ayanamsa epoch: `285-03-22 17:54:02 TT`
- Reference family: Swiss Ephemeris Lahiri definition for the zero epoch
- Precession model used in code: IAU 1976 precession-in-longitude polynomial between the Lahiri zero epoch and the target TDB instant

This implementation is deterministic and auditable rather than table-interpolated. Any future numerical change to Lahiri outputs must bump `engine_semantic_version` and be recorded in [CHANGELOG.md](/Users/sanjeet/Documents/Playground/CHANGELOG.md).

Sidereal longitude is defined as:

`lambda_sidereal = normalize(lambda_tropical_apparent_ecliptic_of_date - lahiri_ayanamsa_tdb)`

This means the tropical input and Lahiri ayanamsa are both evaluated against the same TDB-derived instant.
