# Issue: Outer planet station-window regression (Uranus, Neptune, Pluto)

**Status:** Jupiter and Saturn **done** (Sprint 5); Uranus / Neptune / Pluto **blocked** — see [ADR 0004](../adr/0004-outer-planets-deferred.md)

**Source of truth:** [JPL Horizons](https://ssd.jpl.nasa.gov/horizons/)

## Completed (supported grahas)

| Body | Fixtures | Test | CI |
|------|----------|------|-----|
| Jupiter | `tests/golden/horizons_stations/jupiter.json` (5 retrograde entry + 5 direct, 2020–2030) | `crates/astro-core/tests/outer_planet_stations.rs` | `.github/workflows/horizons.yml` |
| Saturn | `tests/golden/horizons_stations/saturn.json` (5 + 5, 2020–2030) | same | same |

Speed-sign contract: `retrograde == true` iff `longitude_speed_deg_per_day < -1e-12` (`astro_core::motion`). Station windows assert sign flip within ±0.5 day of curated UTC.

## Remaining (not in engine)

For each of **Uranus**, **Neptune**, **Pluto** (not in `CelestialBody` today):

- 5 retrograde-entry station instants (2020–2030)
- 5 direct (end retrograde) station instants (2020–2030)

Each instant should be validated against Horizons apparent geocentric ecliptic longitude speed crossing zero within a ±0.5 day window.

**Blocked on:** [ADR 0004](../adr/0004-outer-planets-deferred.md) — kernel targets, API enums, 30 fixtures.

## Acceptance (outer three only)

- 30 assertions (3 planets × 10 instants) on DE440 when kernel is available
- No change to public HTTP JSON schemas for chart/positions endpoints
