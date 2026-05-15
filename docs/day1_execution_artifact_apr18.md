# Day 1 Execution Artifact

Date: April 18, 2026
Sprint: Astro Engine Day 1
Scope: `daanyam-webapp` to `daanyam-astroengine` integration contract, plus sell-API MVP decisions

## 1. Route Mapping

This table is source-backed from `daanyam-webapp` and `astro-api` as they exist on April 18, 2026.

| Webapp surface | Current webapp route/file | Astro-engine endpoint | Mapping status | Notes |
| --- | --- | --- | --- | --- |
| Dasha calculator page | `/dasha-calculator` -> `POST /api/dasha` | `POST /dasha` | Day 2 | Primary user-facing dasha flow. |
| Kundli explainer page | `/kundli-explainer` -> `POST /api/dasha` + `POST /api/planet-positions` | `POST /dasha` + `POST /positions/sidereal` | Day 2-3 | `lagna` is optional in current webapp response, so page can survive without it initially. |
| Compatibility page | `/compatibility` -> `POST /api/dasha` for saved user profile only | `POST /dasha` | Day 2 | Indirect use only; the main compatibility scoring route is not engine-backed yet. |
| Profile page | `/profile` -> `POST /api/dasha` | `POST /dasha` | Day 2 | Same adapter as `/api/dasha`. |
| Dasha job route | `POST /api/dasha/job` | `POST /dasha` | Day 2 | Background route should reuse the same adapter as `/api/dasha`. |
| Planet positions route | `POST /api/planet-positions` | `POST /positions/sidereal` | Day 3 | Main consumer today is `/kundli-explainer`. |
| Kundli explainer API | `POST /api/kundli-explain` | none directly | No direct engine call | Depends on `/api/dasha` and `/api/planet-positions`, not on a separate engine endpoint. |
| Kundli matching | `POST /api/kundli-matching` | none this sprint | Deferred | Today it computes locally via `computeAshtakoota()`. Future engine path would be two birth-chart calls, but no direct mapping is required for Days 2-7. |
| Compatibility narrative | `POST /api/compatibility` | none | Out of scope | LLM-only narrative route. It consumes already-computed compatibility fields. |
| Panchang page | `/panchang` -> local `getCachedPanchang()` | no complete current endpoint | Deferred/partial | Current engine has no `/panchanga` route. `POST /positions/sidereal` can only cover graha positions, not tithi/yoga/karana/vara. |
| Muhurat page | `/muhurat` -> local `findMuhurats()` + local dasha calc | no complete current endpoint | Deferred/partial | Current engine cannot yet replace muhurat search. It can later help with birth dasha personalization, but not full muhurta scoring. |
| Vaishno Devi muhurat API | `POST /api/vaishno-devi/muhurat` | no complete current endpoint | Deferred/partial | Depends on local panchang plus dasha today. |
| Vaishno Devi panchang API | `GET /api/vaishno-devi/panchang` | no complete current endpoint | Deferred | Blocked on a dedicated engine panchanga route. |

### Exact route decisions for this sprint

1. `POST /api/dasha` maps to `POST /dasha`.
2. `POST /api/planet-positions` maps to `POST /positions/sidereal`.
3. No webapp route maps directly to `POST /chart/sidereal` today, but we should introduce one in Day 3 for chart-first consumers instead of overloading `/api/planet-positions`.
4. Panchang and muhurat stay on existing local code during this sprint, except for optional dasha personalization reuse.

### Recommended new internal route for Day 3

Add a webapp server route such as `POST /api/chart-sidereal` that maps 1:1 to `POST /chart/sidereal`.

Reason:

- `POST /chart/sidereal` returns `lagna`, `houses`, `grahas`, `d9_rashi`, and compact chart `dasha`.
- Those fields do not fit cleanly into the current `/api/planet-positions` contract.
- A dedicated chart route keeps the Day 3 integration explicit and avoids hidden shape creep.

## 2. Request and Response Differences We Must Adapt

## 2.1 `POST /api/dasha` -> `POST /dasha`

### Webapp request today

```json
{
  "name": "optional",
  "date": "1990-05-15",
  "time": "14:30",
  "timezone": "Asia/Kolkata",
  "latitude": 28.6139,
  "longitude": 77.2090,
  "engine": "optional"
}
```

### Astro-engine request

```json
{
  "moon_sidereal_longitude_deg": 199.46019874126517,
  "birth_time_utc_rfc3339": "1990-05-15T09:00:00Z"
}
```

### Required request adaptation

1. Normalize webapp birth input to UTC.
2. Derive `moon_sidereal_longitude_deg` before the engine call.
3. Send the engine the reduced dasha request, not the raw birth payload.

### Important constraint

The current webapp `tryAstroApiDasha()` implementation assumes the engine returns a `DashaResult`-shaped payload. That is false.

### Webapp response today

```json
{
  "ok": true,
  "name": "optional",
  "data": {
    "input": {},
    "timeline": {
      "mahadashas": [],
      "antardashas": [],
      "pratyantars": []
    },
    "current": {
      "maha": {},
      "antar": {},
      "pratyantar": {}
    },
    "meta": {
      "moonSiderealLongitude": 0,
      "nakshatraName": "Swati",
      "rashiName": "Tula"
    },
    "summary": "..."
  },
  "engineUsed": "inhouse",
  "engineMode": "swiss",
  "fallbackReason": null,
  "warnings": []
}
```

### Astro-engine response

```json
{
  "data": {
    "dasha": {
      "maha": { "lord": "rahu", "start": "...", "end": "..." },
      "antar": { "lord": "jupiter", "start": "...", "end": "..." },
      "pratyantar": { "lord": "saturn", "start": "...", "end": "..." }
    }
  },
  "metadata": {
    "engine_semantic_version": "0.17.0",
    "ayanamsa_used": "lahiri"
  }
}
```

### Required response adaptation

1. Convert engine `lord` values to webapp `planet` enum values.
2. Preserve the existing webapp `DashaApiResponseSchema` for this sprint.
3. Reconstruct missing `timeline`, `meta`, `input`, and `summary` in the webapp adapter.
4. Inject webapp transport metadata: `engineUsed`, `engineMode`, `fallbackReason`, `warnings`.

### Decision

For Days 2-7, the webapp contract stays stable. We adapt engine output into the existing `DashaResult` shape instead of rewriting all dasha consumers.

## 2.2 `POST /api/planet-positions` -> `POST /positions/sidereal`

### Webapp request today

```json
{
  "date": "1990-05-15",
  "time": "14:30",
  "timezone": "Asia/Kolkata",
  "latitude": 28.6139,
  "longitude": 77.2090
}
```

### Astro-engine request

```json
{
  "datetime": {
    "kind": "utc",
    "utc": "1990-05-15T09:00:00Z"
  },
  "geo": {
    "latitude_deg": 28.6139,
    "longitude_deg": 77.2090,
    "elevation_m": 0
  },
  "ayanamsa": "lahiri",
  "bodies": ["sun", "moon", "mars", "mercury", "jupiter", "venus", "saturn", "rahu", "ketu"],
  "gravitational_deflection": false,
  "compact": false,
  "projection": "sidereal_only"
}
```

### Webapp response today

```json
{
  "ok": true,
  "planets": [
    {
      "planet": "Moon",
      "rashi": "Libra",
      "rashiSanskrit": "Tula",
      "isRetrograde": false
    }
  ],
  "lagna": "Sagittarius",
  "engineUsed": "inhouse",
  "engineMode": "swiss",
  "fallbackReason": null
}
```

### Astro-engine response

```json
{
  "data": {
    "positions": [
      {
        "body": "moon",
        "sidereal_longitude_deg": 199.46019874126517,
        "longitude_speed_deg_per_day": 13.176358,
        "retrograde": false
      }
    ]
  },
  "metadata": {
    "engine_semantic_version": "0.17.0",
    "ayanamsa_used": "lahiri"
  }
}
```

### Required response adaptation

1. Map engine `body` to title-case `planet`.
2. Derive English and Sanskrit rashi names from `sidereal_longitude_deg`.
3. Map `retrograde` to `isRetrograde`.
4. Add webapp transport fields: `ok`, `engineUsed`, `engineMode`, `fallbackReason`.
5. Set `lagna` to `undefined` in the first pass, or switch this route to chart-backed mode later.

### Important constraint

`POST /positions/sidereal` does not return `lagna`. That is acceptable for `/kundli-explainer` because `lagna` is already optional there, but it means `/api/planet-positions` cannot remain a full drop-in replacement for the current in-house output if some future caller requires ascendant data.

## 2.3 `POST /chart/sidereal`

### What it gives us

- `lagna`
- `houses`
- `grahas`
- `moon_sidereal_longitude_deg`
- `moon_nakshatra`
- `moon_pada`
- chart summary blocks
- compact current dasha
- `d9_rashi`

### What it does not give us

- full Vimshottari timeline arrays
- current webapp `DashaResult` envelope
- full panchang limbs

### Sprint use

Use this route for Day 3 chart-first work, not for Day 2 dasha migration.

## 2.4 Cross-cutting transport differences

1. Astro-engine always returns a `{ data, metadata }` envelope.
2. Webapp public routes currently return route-specific envelopes such as `{ ok, data, engineUsed, ... }`.
3. Astro-engine uses lowercase snake_case enum values such as `moon`, `rahu`, `ketu`, `tula`, `swati`.
4. Webapp consumers often expect title-case planet labels and English/Sanskrit display strings.
5. Astro-engine currently supports `ayanamsa = "lahiri"` only.
6. Astro-engine geolocation is parsed but still geocentric in Phase 1. We should not claim topocentric accuracy in webapp copy yet.

## 2.5 Proxy behavior we must change

The current webapp helper `tryProxyAstroApiRequest()` is not sufficient for integration because it only forwards raw request bodies and pathnames. That works only when route shapes already match.

It does not work for:

- `/api/dasha`, because engine request and response shapes differ.
- `/api/planet-positions`, because the engine route is `/positions/sidereal`, not `/planet-positions`, and the response shape differs.

Decision:

Use a typed `astroEngineClient.ts` plus per-route adapters, not a blind proxy, for Day 2 and Day 3.

## 3. Sprint Decisions

These are the Day 1 decisions for the Apr 18-24 sprint.

### Auth

- Engine auth will be header-based API key auth using `x-api-key`.
- Public unauthenticated routes remain `GET /health` and `GET /openapi.json`.
- All mutating or billable engine routes stay protected: `/dasha`, `/positions`, `/positions/sidereal`, `/chart/sidereal`.
- We will not depend on Cloud Run IAM for app-to-engine auth in this sprint because the webapp needs a simple server-side secret path that also works in local and staging.

### Key storage

- Sprint MVP: engine reads allowed keys from env var `VALID_API_KEYS`.
- Webapp stores its internal service key in server env var `ASTRO_ENGINE_SERVICE_KEY`.
- Webapp must inject `x-api-key` on every engine request from server code only.
- Hashed DB-backed key storage is deferred until after the sprint.

### Rate limiting

- Enforce per-key RPM in the engine middleware.
- Internal webapp service key gets a high limit: `1000 RPM`.
- External customer keys get `60 RPM` default for MVP.
- Daily quota is measured in logs during this sprint but not enforced yet.

Reason:

- RPM enforcement is enough to protect the service this week.
- Daily quota enforcement requires persistent storage and customer state we are not building in Days 2-7.

### Billing scope

- No automated billing integration in this sprint.
- This sprint includes only key auth, usage logging, hosted docs, and a request-access flow.
- Commercial rollout is manual pilot billing, not self-serve usage billing.
- Packaging decision: flat monthly pilot tiers, not per-request billing yet.

Reason:

- We need real usage data before locking a per-request pricing model.
- Tiered pilots let us sell immediately after the sprint without adding Stripe/Razorpay complexity.

## 4. Prioritized Task Board for Days 2-7

## P0

| Day | Task | Depends on | Definition of done |
| --- | --- | --- | --- |
| Day 2 | Build `astroEngineClient.ts` with typed request builders for `/dasha`, `/positions/sidereal`, `/chart/sidereal` | Day 1 contract | Webapp has one server-only client with base URL, timeout, request-id propagation, `x-api-key` support, and normalized engine errors. |
| Day 2 | Implement `/api/dasha` adapter on top of engine `/dasha` | `astroEngineClient.ts` | `/api/dasha` returns the current `DashaApiResponseSchema` when `USE_ASTRO_API_V2=true`, with no consumer changes required. |
| Day 2 | Reuse the same adapter in `/api/dasha/job` | `/api/dasha` adapter | Job route stores a payload identical to the interactive dasha route payload. |
| Day 3 | Implement `/api/planet-positions` adapter on top of `/positions/sidereal` | `astroEngineClient.ts` | `/api/planet-positions` returns the existing `PlanetPositionsResponse` contract using astro-engine data. |
| Day 3 | Introduce a new webapp chart route backed by `/chart/sidereal` | `astroEngineClient.ts` | There is one server route that exposes `lagna`, `houses`, `grahas`, and compact chart dasha without overloading `/api/planet-positions`. |
| Day 4 | Add engine auth middleware and service key injection from webapp | Day 2-3 engine client | Engine returns `401` for missing/invalid keys, and webapp requests succeed end to end in local and staging. |

## P1

| Day | Task | Depends on | Definition of done |
| --- | --- | --- | --- |
| Day 5 | Add per-key rate limiting middleware | Engine auth middleware | Internal service key stays under high RPM, external keys are limited, and `429` includes `Retry-After`. |
| Day 5 | Add structured usage logs by key prefix and endpoint | Engine auth middleware | Every protected request emits endpoint, status, latency, and key-prefix usage data into logs. |
| Day 6 | Serve hosted docs at `/docs` and publish API auth scheme in OpenAPI | Engine auth middleware | `/docs` renders working Redoc against the live OpenAPI spec and documents `x-api-key`. |
| Day 6 | Publish `/api-access` page in webapp | Pricing decision | Page is live with endpoints overview, pilot pricing copy, and request-access flow. |

## P2

| Day | Task | Depends on | Definition of done |
| --- | --- | --- | --- |
| Day 7 | End-to-end regression pack | Days 2-6 | Tests cover dasha, planet positions, chart route, `401`, and `429` against a deployed engine. |
| Day 7 | Monitoring and alerting setup | Usage logs in place | Dashboard shows request rate, latency, error rate, and Cloud Run health. |
| Day 7 | Fallback verification | Day 2-3 adapters | With `USE_ASTRO_API_V2=false`, existing webapp fallback paths still work. |
| Day 7 | Deploy and env doc updates | Auth and rate-limit env settled | Both repos document required vars, staging is green, and production rollout is unblocked. |

## Dependency order

1. Typed client and adapters.
2. Dasha migration.
3. Planet positions migration.
4. Chart route introduction.
5. Auth.
6. Rate limits plus usage logs.
7. Docs and access page.
8. E2E verification and monitoring.

## 5. Explicit Non-Goals for This Sprint

- No engine-backed full panchang replacement.
- No engine-backed muhurat search replacement.
- No DB-backed API key issuance or revocation UI.
- No Stripe or Razorpay billing integration.
- No topocentric accuracy claims in product copy.

## 6. Immediate Implementation Guidance for Day 2

1. Do not use `tryProxyAstroApiRequest()` for dasha or planet positions.
2. Preserve the current webapp response schemas and adapt engine output into them.
3. Add `ASTRO_ENGINE_SERVICE_KEY` support to webapp server request headers from the start, even if engine auth lands on Day 4.
4. Keep `USE_ASTRO_API_V2` as the rollback switch for all migrated routes.
