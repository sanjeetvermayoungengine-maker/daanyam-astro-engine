# Topocentric Regression Scaffold

This directory is reserved for future topocentric regression fixtures.

Planned layout:

```text
tests/regression/topocentric/
  README.md
  topocentric_geocenter_baseline.json
  topocentric_observer_mumbai.json
```

Planned filename contract:

- `topocentric_geocenter_baseline.json`
  control fixture set matching the same UTC/body/frame inputs without topocentric corrections
- `topocentric_observer_mumbai.json`
  example observer-site fixture set for a concrete topocentric location

Required metadata fields per file:

- source provenance
- observer latitude/longitude/elevation
- ellipsoid identifier
- refraction policy
- frame and quantity metadata
- tolerance per metric
- body list and UTC timestamps
- whether the file is a control baseline or a true topocentric observer case

See [docs/topocentric_plan.md](/Users/sanjeet/Documents/Playground/docs/topocentric_plan.md) for the design plan.
