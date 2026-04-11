# ADR 0003: Topocentric Observer Policy

## Status

Proposed

## Problem statement

The engine currently exposes geocentric results only, but the request and metadata surfaces already reserve space for future topocentric behavior. We need a documented policy for how topocentric support will enter the API and regression harness without creating ambiguity around observer mode, Earth model assumptions, or fixture provenance.

## Proposed decision

The detailed design remains in [docs/topocentric_plan.md](/Users/sanjeet/Documents/Playground/docs/topocentric_plan.md). This ADR records the current policy direction:

- topocentric requests will require explicit geolocation input,
- observer-mode metadata will evolve additively from the current geocentric contract,
- no silent fallback to approximate observer models will be allowed,
- regression fixtures will be organized under [tests/regression/topocentric/README.md](/Users/sanjeet/Documents/Playground/tests/regression/topocentric/README.md) before implementation ships.

## Non-goals for Phase 1

- implementing topocentric astronomy,
- adding atmospheric refraction behavior,
- changing current geocentric endpoints,
- introducing topocentric vectors before the fixture/provenance contract is finalized.

## References

- [docs/topocentric_plan.md](/Users/sanjeet/Documents/Playground/docs/topocentric_plan.md)
- [tests/regression/topocentric/README.md](/Users/sanjeet/Documents/Playground/tests/regression/topocentric/README.md)
