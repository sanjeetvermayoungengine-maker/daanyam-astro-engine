# Regression Harness

This directory stores source-attributed regression fixtures for the DE440-backed apparent-position pipeline.

## Current Horizons fixture settings

- Source: JPL Horizons observer table
- Quantity: `31` (`ObsEcLon`, `ObsEcLat`)
- Observer: Earth geocenter (`500@399`)
- Frame: observer-centered apparent ecliptic-of-date
- Atmosphere: airless
- Time tags: UTC/UT as returned by Horizons
- Tolerance: `0.05 deg` for longitude and latitude
- Default engine behavior: `gravitational_deflection = true`, which matches the current Horizons baseline used in fixtures

## Fixture groups

- `horizons_deflection_on`
  Uses the current baseline settings including gravitational deflection enabled and full Horizons parameter metadata in the manifest.
- `horizons_deflection_off`
  Uses the same observer-table parameter family with gravitational deflection disabled in the local DE440 pipeline. Horizons does not expose a standalone observer-table deflection toggle, so these audit fixtures are engine-derived and documented as such in the manifest metadata.
- `true_node_manual_reference`
  Uses the documented DE440 true-node derivation policy in `docs/astronomy.md`. The frame is `mean_ecliptic_of_date`, deflection is disabled, and the fixture metadata records the node policy explicitly because Horizons does not expose Rahu/Ketu as direct bodies.

## Future planning references

- Topocentric design plan: [docs/topocentric_plan.md](/Users/sanjeet/Documents/Playground/docs/topocentric_plan.md)
- Topocentric regression scaffold: [tests/regression/topocentric/README.md](/Users/sanjeet/Documents/Playground/tests/regression/topocentric/README.md)
- Mean-node policy status: [docs/mean_nodes.md](/Users/sanjeet/Documents/Playground/docs/mean_nodes.md)

## Frame correction note

Earlier placeholder Moon expectations in this repo were labeled as J2000 ecliptic apparent longitudes, but the actual validated Horizons comparison for quantity `31` is ecliptic-of-date apparent longitude/latitude. The regression fixtures now use the official Horizons quantity `31` values for that documented frame.
