# Daanyam Astro Engine — 30-Sprint Codex Roadmap

**Starting version:** 0.17.0  
**Target:** World's best open Vedic + Western astronomical computation engine  
**Sprint cadence:** 1 week per sprint (~7.5 months total)  
**Working model:** OpenAI Codex drives implementation daily; golden fixture tests are the acceptance gate

---

## Deployment Architecture (Pre-Sprint: Week 0)

Before sprints begin, the following infra must be in place. This is the foundation every sprint builds on.

### Source of truth: GitHub

- Host the workspace at `github.com/daanyam/astro-engine` (private or public)
- All Codex PRs target `main` via short-lived feature branches
- Branch naming: `codex/sprint-N-short-description`
- Every PR requires CI green before merge — no exceptions

### CI: GitHub Actions

Three pipelines run on every PR:

```
1. test       → cargo test --workspace
2. bench      → cargo bench --workspace (with regression gate vs baseline)
3. build-push → docker build → push to GCP Artifact Registry (on merge to main only)
```

Store the DE440 kernel in GCS. The CI pipeline downloads it before running tests:
```yaml
- name: Fetch DE440 kernel
  run: |
    gcloud storage cp gs://daanyam-ephe/de440.bsp ./ephe/de440.bsp
  env:
    ASTRO_EPHE_PATH: ./ephe
```

### Runtime: GCP Cloud Run

- Deploy `astro-api` as a Cloud Run service (stateless, scales to zero)
- DE440 kernel served from a GCS bucket; downloaded to `/tmp/ephe/` on cold start
- Set `ASTRO_EPHE_PATH=/tmp/ephe` as a Cloud Run env var
- Mount the kernel via Cloud Run's built-in GCS volume mount (GA as of 2024) — no download latency
- Region: `asia-south1` (Mumbai) — closest to your users on daanyam.in

### Cost profile (rough)

| Resource | Estimated cost |
|---|---|
| Cloud Run (0.17 vCPU, 512MB) | ~$5–15/mo at moderate traffic |
| GCS bucket (DE440 kernel ~470MB) | ~$0.01/mo storage |
| Artifact Registry | ~$0.10/mo |
| GitHub Actions (public repo) | Free |

---

## Phase 1 — Infrastructure & Ayanamsa Completeness (Sprints 1–5)

### Sprint 1 — CI/CD pipeline + Cloud Run deployment

**Codex task:** Write the GitHub Actions workflow files and Cloud Run deployment config.

Deliverables:
- `.github/workflows/ci.yml` — test + bench on every PR
- `.github/workflows/deploy.yml` — docker build + push + Cloud Run deploy on `main` merge
- `cloudbuild.yaml` as fallback
- `Dockerfile` health check on `/health`

Acceptance gate: `GET /health` returns `200` on the deployed Cloud Run URL.

---

### Sprint 2 — DE440 kernel GCS integration + production config

**Codex task:** Implement a startup routine that resolves the DE440 kernel from GCS or local path, with proper error messaging and retry.

Deliverables:
- `crates/astro-core/src/kernel_resolver.rs` — reads `ASTRO_EPHE_PATH` or downloads from GCS URI
- Startup log line: `INFO de440 kernel loaded from gs://... in 120ms, 2024–2150 coverage confirmed`
- Integration test: boot with a real kernel path, call `/chart/sidereal`, assert no fallback to InMemory

Acceptance gate: Cloud Run cold start < 3s with GCS volume mount.

---

### Sprint 3 — Raman ayanamsa

**Codex task:** Implement Raman ayanamsa in `astro-vedic/src/ayanamsa.rs` following the same pattern as Lahiri.

Raman zero epoch: JD 2,396,758.0 (1827-01-01 TT), uses the same IAU 1976 precession polynomial.

Deliverables:
- `raman_ayanamsa_deg(jd_tdb: f64) -> f64`
- `RAMAN_ALGO_ID: &str = "raman_swe_zero_epoch_iau1976_v1"`
- Golden fixture table covering 1900–2100 (10 reference points, tolerance 1e-9)
- `POST /positions/sidereal` and `POST /chart/sidereal` accept `"ayanamsa": "raman"`

Acceptance gate: All golden fixtures pass. `/chart/sidereal` with Raman returns `ayanamsa_algorithm: "raman_swe_zero_epoch_iau1976_v1"` in `computation_meta`.

---

### Sprint 4 — Krishnamurti ayanamsa

**Codex task:** Implement KP (Krishnamurti Paddhati) ayanamsa.

KP zero epoch: same Swiss Ephemeris convention as Lahiri but with a different base offset (~0.883° less than Lahiri at J2000).

Deliverables:
- `kp_ayanamsa_deg(jd_tdb: f64) -> f64`
- `KP_ALGO_ID: &str = "kp_swe_zero_epoch_iau1976_v1"`
- Golden fixtures (same 10 reference JDs as Lahiri, different expected values)
- KP sub-lord lookup table (249 sub-lords across 27 nakshatras) as a static data structure

Acceptance gate: Golden fixtures pass. KP sub-lord for a given Moon longitude returns deterministically.

---

### Sprint 5 — Outer planet station-window regression tests (Uranus, Neptune, Pluto)

**Codex task:** Curate JPL Horizons-vetted UTC instants for Uranus/Neptune/Pluto retrograde station windows and add regression tests.

Use Horizons data for:
- Uranus: 5 retrograde entry + 5 direct station instants (2020–2030)
- Neptune: same
- Pluto: same

Deliverables:
- `crates/astro-core/tests/outer_planet_stations.rs`
- Each test asserts `longitude_speed_deg_per_day` sign at the curated instant (within ±0.5 day window)
- Tolerance: speed sign must flip within the expected window

Acceptance gate: All 30 station assertions pass on the DE440 backend.

---

## Phase 2 — Divisional Charts (Sprints 6–11)

Divisional charts (Vargas) are the next highest-value Vedic feature for Kundali generation.

### Sprint 6 — D9 Navamsa

**Codex task:** Implement D9 Navamsa as a new module `crates/astro-vedic/src/vargas/navamsa.rs`.

Formula: each rashi (30°) is divided into 9 padas of 3°20'. The Navamsa sign is determined by the pada count and the elemental sequence (Fire → Earth → Air → Water).

Deliverables:
- `navamsa_sign(sidereal_longitude_deg: f64) -> Rashi`
- Extend `/chart/sidereal` response: each graha gains `"d9_rashi": "scorpio"` (or similar field)
- 27 golden fixture assertions (one per nakshatra boundary)

Acceptance gate: Navamsa sign matches Swiss Ephemeris output for the 27 test points.

---

### Sprint 7 — D10 Dashamsha

**Codex task:** Implement D10 Dashamsha in `crates/astro-vedic/src/vargas/dashamsha.rs`.

Formula: each rashi divided into 10 parts of 3°. Odd signs start from Aries, even signs from Capricorn.

Deliverables:
- `dashamsha_sign(sidereal_longitude_deg: f64) -> Rashi`
- Extend chart response graha object with `"d10_rashi"`
- 12 golden fixture assertions (one per rashi boundary)

Acceptance gate: D10 sign matches Swiss Ephemeris for all 12 test points.

---

### Sprint 8 — D3 Drekkana, D7 Saptamsha, D12 Dwadashamsha

**Codex task:** Implement the three most commonly used remaining Vargas.

- D3: each sign divided into 3 parts of 10°
- D7: each sign divided into 7 parts of ~4°17'
- D12: each sign divided into 12 parts of 2°30'

Deliverables:
- `crates/astro-vedic/src/vargas/drekkana.rs`, `saptamsha.rs`, `dwadashamsha.rs`
- Extend chart graha object with `d3_rashi`, `d7_rashi`, `d12_rashi`
- Golden fixture tables for each

---

### Sprint 9 — Varga framework + D1/D2/D4/D16/D20/D24/D30/D40/D45/D60

**Codex task:** Build a general Varga computation framework and implement the remaining standard Shodashavargas.

Deliverables:
- `crates/astro-vedic/src/vargas/mod.rs` — `VargaChart { division: u8, signs: Vec<Rashi> }` and a `compute_varga(longitude: f64, division: u8) -> Rashi` dispatcher
- All 16 Shodashavargas wired in
- New endpoint: `POST /chart/varga` — accepts `{ datetime, geo, ayanamsa, division: 9, bodies: [...] }` and returns the Varga chart
- OpenAPI spec updated

Acceptance gate: D60 (Shastiamsha) passes 3 reference point assertions.

---

### Sprint 10 — Divisional chart response design + `POST /chart/varga`

**Codex task:** Design and implement the full chart-varga API surface including compact mode and schema versioning.

Deliverables:
- `schema_version: "chart_varga_v1"` in response
- Projection support: `"projection": "sidereal_only"` suppresses tropical coordinate fields
- Compact mode: omits speed and latitude fields
- OpenAPI 3.1 schema for the new route
- `engine_semantic_version` bumped to `0.18.0`

---

### Sprint 11 — Panchanga (Tithi, Nakshatra, Yoga, Karana, Vara)

**Codex task:** Implement the Panchanga (Hindu almanac) five limbs as a new module.

Deliverables:
- `crates/astro-vedic/src/panchanga.rs`
- `tithi(sun_lon: f64, moon_lon: f64) -> Tithi` (1–30)
- `yoga(sun_lon: f64, moon_lon: f64) -> Yoga` (1–27)
- `karana(sun_lon: f64, moon_lon: f64) -> Karana` (1–11, repeating)
- `vara(weekday: Weekday) -> Vara`
- New endpoint: `POST /panchanga` — accepts `{ datetime, geo, ayanamsa }`, returns all five limbs
- Golden fixtures for 5 reference datetimes

---

## Phase 3 — Vedic Strength & Yoga Systems (Sprints 12–17)

### Sprint 12 — Graha aspects (Drishti)

**Codex task:** Implement Parashari graha aspects including special aspects for Mars (4th/8th), Jupiter (5th/9th), Saturn (3rd/10th), and Rahu/Ketu.

Deliverables:
- `crates/astro-vedic/src/aspects.rs`
- `graha_aspects(chart: &SiderealChart) -> Vec<AspectPair>` where each pair has `(aspecting, aspected, strength: f64)`
- Full aspect (1.0), three-quarter (0.75), half (0.5), quarter (0.25)
- Extend `/chart/sidereal` with `summary.aspects: [...]`

---

### Sprint 13 — Ashtakavarga (basic Sarvashtakavarga)

**Codex task:** Implement the 8-source Ashtakavarga point system.

Each of 8 grahas (Sun, Moon, Mars, Mercury, Jupiter, Venus, Saturn, Ascendant) contributes a Bhinnashtakavarga table of 8 points per rashi. Sarvashtakavarga = sum across all 8.

Deliverables:
- `crates/astro-vedic/src/ashtakavarga.rs`
- `bhinnashtakavarga(source: Graha, chart: &SiderealChart) -> [u8; 12]`
- `sarvashtakavarga(chart: &SiderealChart) -> [u8; 12]`
- New endpoint: `POST /chart/ashtakavarga`
- 12 golden assertions per source graha against reference tables

---

### Sprint 14 — Shadbala (Positional + Directional + Temporal strength)

**Codex task:** Implement the first 3 of 6 Shadbala components: Sthana Bala, Dig Bala, Kala Bala.

Deliverables:
- `crates/astro-vedic/src/shadbala/sthana.rs` — exaltation, own sign, Moolatrikona, friendly/enemy placement
- `crates/astro-vedic/src/shadbala/dig.rs` — directional strength based on house placement
- `crates/astro-vedic/src/shadbala/kala.rs` — day/night, hora, paksha, masa, varsha strength components

---

### Sprint 15 — Shadbala completion (Cheshta, Naisargika, Drik Bala)

**Codex task:** Complete the remaining 3 Shadbala components.

Deliverables:
- `crates/astro-vedic/src/shadbala/cheshta.rs` — motional strength (retrograde speed relative to mean motion)
- `crates/astro-vedic/src/shadbala/naisargika.rs` — natural strength (fixed weights: Sun > Moon > Venus > Jupiter > Mercury > Mars > Saturn)
- `crates/astro-vedic/src/shadbala/drik.rs` — aspectual strength from Ashtakavarga
- New endpoint: `POST /chart/shadbala` — returns Rupas (total) and Virupas (components) per graha
- Ishta/Kashta Phala derived from Shadbala

---

### Sprint 16 — Yoga detection (Raj, Dhana, Duryoga)

**Codex task:** Implement a rule-based Yoga detection system starting with the 20 most important Yogas.

Deliverables:
- `crates/astro-vedic/src/yogas/mod.rs`
- Detected Yogas per chart: `Vec<Yoga { name: &str, grahas: Vec<Graha>, strength: f64, description: &str }>`
- Implemented: Gajakesari, Budha-Aditya, Pancha Mahapurusha (5 yogas), Dhana Yogas (5), Raj Yogas (5), Kemadruma, Veshi/Voshi
- Extend `/chart/sidereal` with `summary.yogas`

---

### Sprint 17 — Transit engine (`POST /transits`)

**Codex task:** Implement a transit computation engine that finds when a graha transits a given sidereal sign or crosses a specific longitude.

Deliverables:
- New endpoint: `POST /transits` — accepts `{ from_datetime, to_datetime, geo, ayanamsa, bodies, event_type: "sign_ingress" | "longitude_crossing" | "retrograde_station" }`
- Iterative bisection search over the DE440 backend (step down from 1 day → 1 hour → 1 minute)
- Returns: `[{ body, event_type, datetime_utc, sidereal_longitude_deg, sign }]`
- Max window: 2 years (enforced server-side)

---

## Phase 4 — Western Astrology (Sprints 18–22)

### Sprint 18 — Western tropical positions + full body set

**Codex task:** Complete `astro-western` — it is currently a stub. Implement tropical position output for all 10 classical bodies plus Chiron.

Deliverables:
- `crates/astro-western/src/positions.rs`
- `POST /positions/western` — returns tropical longitude/latitude/speed/retrograde for `Sun, Moon, Mercury, Venus, Mars, Jupiter, Saturn, Uranus, Neptune, Pluto, Chiron`
- Chiron requires a separate small kernel or polynomial approximation (document the approach)
- `zodiac_sign` derived from tropical longitude (simple 30° division)

---

### Sprint 19 — Aspect calculations (Western)

**Codex task:** Implement Western aspect detection between any two bodies.

Deliverables:
- `crates/astro-western/src/aspects.rs`
- Major aspects: Conjunction (0°), Opposition (180°), Trine (120°), Square (90°), Sextile (60°) with configurable orbs
- Minor aspects: Quincunx (150°), Semi-square (45°), Sesquiquadrate (135°)
- `POST /aspects/western` — accepts `{ datetime, geo, bodies, orbs: { conjunction: 8.0, ... } }`
- Returns applying/separating flag and exact aspect datetime (via bisection)

---

### Sprint 20 — Placidus house system

**Codex task:** Implement Placidus house cusps using the iterative convergence algorithm.

Deliverables:
- `crates/astro-core/src/houses/placidus.rs`
- `placidus_cusps(jd: f64, lat_deg: f64, lon_deg: f64) -> [f64; 12]`
- Handle polar regions gracefully (fall back to Equal houses above 66°N/S with a warning flag)
- Wire into `HouseSystem::Placidus` in the backend trait

---

### Sprint 21 — Equal house system + house system parity

**Codex task:** Implement Equal houses and ensure all three house systems return consistent schema.

Deliverables:
- `crates/astro-core/src/houses/equal.rs`
- All house systems return `{ house_system, cusps: [f64; 12], ascendant_deg, mc_deg }`
- `/chart/western` endpoint: tropical chart with Placidus/Equal/WholeSign support

---

### Sprint 22 — Western chart endpoint (`POST /chart/western`)

**Codex task:** Wire everything together into a unified Western chart endpoint.

Deliverables:
- `POST /chart/western` — accepts `{ datetime, geo, house_system, bodies, orbs }`
- Returns: tropical positions, house placements, aspect grid, chart ruler
- `schema_version: "chart_western_v1"`
- OpenAPI 3.1 spec updated
- `engine_semantic_version: "0.19.0"`

---

## Phase 5 — SDK Surface & Event Finding (Sprints 23–26)

### Sprint 23 — WASM full surface

**Codex task:** Mirror the complete NAPI request/response surface into `astro-wasm`.

Currently only `normalize_angle` is exposed. The NAPI crate has typed helpers for all three request families.

Deliverables:
- `wasm_chart_sidereal(request_json: &str) -> Result<String, JsError>`
- `wasm_positions_sidereal(request_json: &str) -> Result<String, JsError>`
- `wasm_positions_tropical(request_json: &str) -> Result<String, JsError>`
- All three backed by InMemory backend (WASM cannot do file I/O for DE440); document this clearly
- WASM build target: `wasm32-unknown-unknown` via `wasm-pack`
- Published as `@daanyam/astro-engine` NPM package (build script only, not auto-published)

---

### Sprint 24 — NAPI surface parity + TypeScript types

**Codex task:** Complete NAPI bindings and generate TypeScript `.d.ts` declarations.

Deliverables:
- All chart + positions + panchanga + varga routes exposed via NAPI
- `napi-build` generates `index.d.ts` with full type coverage
- Integration test: call `chartSidereal({...})` from a Node.js test script, assert on `schema_version`

---

### Sprint 25 — Event finder (`POST /events`)

**Codex task:** Build a general astronomical event search endpoint over the transit engine.

Deliverables:
- `POST /events` — accepts `{ from_datetime, to_datetime, geo, ayanamsa, event_types: [...] }`
- Supported event types: `graha_retrograde_entry`, `graha_retrograde_exit`, `sign_ingress`, `nakshatra_ingress`, `tithi_change`, `lunar_eclipse`, `solar_eclipse`
- Lunar/solar eclipse detection via Sun-Moon-Earth alignment threshold (angular separation < 1.5° for lunar, < 0.5° for solar)
- Returns a timeline: `[{ event_type, body, datetime_utc, description }]`

---

### Sprint 26 — Synastry + composite chart

**Codex task:** Implement bi-wheel synastry and composite chart (midpoint method).

Deliverables:
- `POST /chart/synastry` — accepts two birth data objects, returns inter-aspects and compatibility summary
- `POST /chart/composite` — returns midpoint composite chart
- Extend `/chart/western` to accept an optional `partner` object for synastry overlay

---

## Phase 6 — Jaimini, Muhurta & Production Hardening (Sprints 27–30)

### Sprint 27 — Jaimini Chara Dasha

**Codex task:** Implement Jaimini Chara Dasha system as an alternative to Vimshottari.

Deliverables:
- `crates/astro-vedic/src/jaimini/chara_dasha.rs`
- Atmakaraka computation (planet with highest degree in any sign)
- Chara Dasha sequence derived from Atmakaraka sign placement
- Extend `/chart/sidereal` with optional `"dasha_system": "chara"` request parameter

---

### Sprint 28 — Muhurta engine (`POST /muhurta`)

**Codex task:** Build a Muhurta (electional astrology) engine that scores time windows.

Deliverables:
- `POST /muhurta` — accepts `{ from_datetime, to_datetime, geo, ayanamsa, purpose: "marriage" | "business" | "travel" | "medical" | "general" }`
- Scoring: Panchanga quality (Tithi, Vara, Nakshatra, Yoga, Karana) × graha strength × absence of Duryoga
- Returns: ranked windows `[{ from, to, score, notes }]`
- Built on top of the Transit + Panchanga engines from Sprints 17 + 11

---

### Sprint 29 — API auth, rate limiting, observability

**Codex task:** Harden the production API.

Deliverables:
- API key authentication middleware (static keys via env var; no DB required for v1)
- Rate limiting: 100 req/min per key via `tower-governor` or equivalent
- Structured JSON logging with `tracing-subscriber` in Cloud Run format
- `GET /metrics` endpoint: request count, p50/p95 latency, backend type (de440 vs inmemory)
- Dockerfile updated: non-root user, read-only filesystem

---

### Sprint 30 — Performance audit + regression gate + v1.0 changelog

**Codex task:** Run the benchmark suite, establish hard performance targets, and write the v1.0 milestone changelog.

Deliverables:
- Benchmark targets (enforced in CI via `criterion` regression gate):
  - `POST /positions` (InMemory): < 1ms p99
  - `POST /chart/sidereal` (DE440): < 50ms p99
  - `POST /events` 1-year window: < 500ms p99
- `CHANGELOG.md` entry for `1.0.0` — full feature summary
- `engine_semantic_version: "1.0.0"` in all responses
- `schema_version` audit: confirm `chart_sidereal_v1`, `chart_varga_v1`, `chart_western_v1` are stable
- OpenAPI spec published to `dist/openapi.json` and `dist/openapi.yaml`

---

## Summary

| Phase | Sprints | Focus |
|---|---|---|
| Infra | 1–2 | GitHub CI/CD, Cloud Run, DE440 on GCS |
| Ayanamsa | 3–5 | Raman, KP, outer planet stations |
| Divisional charts | 6–11 | D1–D60 Vargas, Panchanga |
| Strength & Yogas | 12–17 | Ashtakavarga, Shadbala, Yogas, Transits |
| Western | 18–22 | Tropical chart, Placidus, aspects |
| SDK & Events | 23–26 | WASM, NAPI, event finder, synastry |
| Jaimini & Hardening | 27–30 | Chara Dasha, Muhurta, auth, v1.0 |

**Engine version trajectory:** 0.17.0 → 0.18.0 (Vargas) → 0.19.0 (Western) → 1.0.0 (complete)
