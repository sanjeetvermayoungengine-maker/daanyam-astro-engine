# Sprint 7 reconciliation (Phase 2 entry)

**Plan reference:** `AstroEngine_Sprint_Prompts.md` Sprint 7 — D9 Navamsha  
**Status:** 2026-05 — reconcile before starting new D9 work

## Engine (`daanyam-astroengine`)

| Sprint 7 ask | Current state | Action |
|--------------|---------------|--------|
| `divisional/navamsha.rs` BPHS rules | [`crates/astro-vedic/src/vargas/navamsa.rs`](../crates/astro-vedic/src/vargas/navamsa.rs) exists | Verify movable/fixed/dual mapping vs BPHS; add fixtures if gaps |
| `compute_navamsha` API | D9 on chart grahas (`d9_rashi` in `/chart/sidereal`) | Sprint 10 per prompts: dedicated `POST /chart/varga` |
| 10 golden fixtures | [`tests/golden/navamsa_reference_points.json`](../tests/golden/navamsa_reference_points.json), [`navamsa_golden.rs`](../crates/astro-vedic/tests/navamsa_golden.rs) | Extend to 10 Parashara + cusp edge cases if &lt; 10 |
| `feature = "varga-d9"` | Not feature-flagged | Optional: add feature flag only if build time matters |
| &lt;2ms bench | Not documented | Add `benches/divisional.rs` smoke when touching D9 |
| API endpoint | Deferred to Sprint 10 | No change in Sprint 7 |

**Recommendation:** Treat engine Sprint 7 as **hardening + fixtures**, not greenfield implementation.

## Webapp (`daanyam-webapp`)

| Sprint 7 ask | Current state | Action |
|--------------|---------------|--------|
| `DivisionalChartView` extract | Rashi chart still monolithic | New task: extract from `components/kundli/` (or equivalent) |
| D9 UI | Not wired | After engine exposes varga API (Sprint 10) |

**Recommendation:** Webapp Sprint 7 can proceed **in parallel** with production flip; does not block Phase 1 close.

## Gate

Do not start Phase 2 feature sprints until:

- `USE_ASTRO_API_V2=true` in production (see webapp `docs/qa/phase0-w2.md`)
- PostHog `inhouse` ≥ 95% on dasha/planet paths (24h window)
