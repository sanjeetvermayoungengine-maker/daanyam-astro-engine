# Deploying `astro-api`

## Runtime contract (server process)

Required for a real ephemeris-backed deployment:

| Variable | Purpose |
| --- | --- |
| `ASTRO_BACKEND` | `de440` for JPL DE440-backed responses, or `demo` for in-memory fixtures (never use `demo` in production without an explicit override; see below). |
| `ASTRO_EPHE_PATH` | Path to `de440.bsp` on disk, or `gs://…` URI resolved at startup. |
| `VALID_API_KEYS` | Comma-separated API keys for `POST` routes (and `GET` only where documented as protected). Empty means every authenticated request is rejected. |

Strongly recommended:

| Variable | Purpose |
| --- | --- |
| `ASTRO_EPHE_CACHE_DIR` | Writable directory when using `gs://` kernels or explicit cache. |
| `ENVIRONMENT` or `NODE_ENV` | Set to `production` in live environments so `ASTRO_BACKEND=demo` is refused unless `ALLOW_DEMO_BACKEND=true`. |
| `HOST` / `PORT` | Bind address (Cloud Run uses `0.0.0.0` and `3000`). |
| `RATE_LIMIT_RPM` | Optional per–API-key rolling 60s cap per process (`0` or unset disables). |

Optional observability:

| Variable | Purpose |
| --- | --- |
| `RUST_LOG` | Rust tracing/log filter (e.g. `info`, `warn`). |
| `METRICS_TOKEN` | Bearer secret for `GET /metrics` (Prometheus scrape). When unset, `/metrics` returns 503. |
| `GOOGLE_CLOUD_PROJECT`, `GCP_PROJECT`, or `GCLOUD_PROJECT` | When set, `x-cloud-trace-context` is mapped to Cloud Logging trace fields in stderr JSON logs. |

Configurable startup:

| Variable | Purpose |
| --- | --- |
| `ASTRO_EPHE_GCS_URI` | Alternative to `ASTRO_EPHE_PATH` for GCS resolution in some deployments. |
| `ALLOW_DEMO_BACKEND` | Must be `true` to allow `ASTRO_BACKEND=demo` when `ENVIRONMENT=production` or `NODE_ENV=production`. |

Recommended kernel mount:

- Mount the JPL kernel read-only at `/tmp/ephe/de440.bsp`
- Set `ASTRO_EPHE_PATH=/tmp/ephe/de440.bsp`
- Set `ASTRO_EPHE_CACHE_DIR=/tmp/ephe`
- Set `ASTRO_BACKEND=de440`
- Set `VALID_API_KEYS` to one or more comma-separated secrets for API clients
- Default Cloud Run recommendation for production stability: `min-instances=1`

Alternative startup path:

- Set `ASTRO_EPHE_PATH=gs://bucket/de440.bsp` and the runtime will download it into `ASTRO_EPHE_CACHE_DIR`
- Cloud Run service-to-service auth is attempted automatically through the metadata server before falling back to unauthenticated GCS access

## Demo backend guard

- Production-like environments must not start with `ASTRO_BACKEND=demo` by accident.
- If `ASTRO_BACKEND=demo` and either `ENVIRONMENT=production` or `NODE_ENV=production`, startup fails fast by default.
- Override only for intentional demos with `ALLOW_DEMO_BACKEND=true`.
- Recommended production setting:
  - `ASTRO_BACKEND=de440`
  - `VALID_API_KEYS=<long-random-key>[,<another-key>]`
  - leave `ALLOW_DEMO_BACKEND` unset or set it to `false`

## API authentication

- All routes require an API key except `GET /health`, `GET /openapi.json`, and `GET /docs` (Redoc UI; loads the same spec as `/openapi.json` in the browser).
- Clients may send either:
  - `x-api-key: <key>`
  - `Authorization: Bearer <key>`
- Missing keys return HTTP `401` with `{"error":"missing_api_key"}`.
- Invalid keys return HTTP `401` with `{"error":"invalid_api_key"}`.
- Store multiple live keys in `VALID_API_KEYS` as a comma-separated list to support rotation.

## Rate limiting and usage logging

- Optional `RATE_LIMIT_RPM` sets the maximum number of authenticated requests per API key in a rolling 60-second window (enforced in memory per server process). When unset or set to `0`, rate limiting is disabled.
- Public routes (`GET /health`, `GET /openapi.json`, `GET /docs`) are not counted. After authentication, excess traffic receives HTTP `429` with a `Retry-After` header (seconds) and body `{"error":"rate_limit_exceeded"}`.
- In horizontally scaled deployments, each instance applies its own counter; treat this as an MVP guardrail rather than a global quota.

Each request emits one structured JSON line on stderr with `message` set to `api_usage`, including `api_key_prefix` (at most the first eight characters of the key, never the full secret), `path`, `method`, `status`, `latency_ms`, `request_id`, and `request_body_hash` when a JSON request body was hashed (same rules as `POST`/`PUT`/`PATCH` with `Content-Type: application/json`).

## Container notes

- Dockerfile: [Dockerfile](../Dockerfile)
- Docker Compose sample: [deploy/docker-compose.yml](../deploy/docker-compose.yml)
- Kubernetes samples:
  - [deploy/k8s/deployment.yaml](../deploy/k8s/deployment.yaml)
  - [deploy/k8s/service.yaml](../deploy/k8s/service.yaml)
- Cloud Run manifest (**canonical** for CI/CD): [deploy/cloudrun/service.yaml](../deploy/cloudrun/service.yaml)
- Alternate Cloud Run template (hand deploy / `astro-engine` naming): [deploy/cloud_run.yaml](../deploy/cloud_run.yaml) — same `minScale: "1"` policy; not used by `deploy.yml` / `cloudbuild.yaml`
- GitHub Actions:
  - [`.github/workflows/ci.yml`](../.github/workflows/ci.yml)
  - [`.github/workflows/deploy.yml`](../.github/workflows/deploy.yml)
- Cloud Build fallback: [cloudbuild.yaml](../cloudbuild.yaml)
- Default container port: `3000`
- Health endpoint: `GET /health`

Example container run:

```bash
docker build -t daanyam-astro-api:rc .
docker run --rm \
  -p 3000:3000 \
  --mount type=bind,source=/tmp/astro-ephe/de440.bsp,target=/ephe/de440.bsp,readonly \
  -e ASTRO_BACKEND=de440 \
  -e VALID_API_KEYS=replace-with-long-random-key \
  -e ENVIRONMENT=production \
  -e HOST=0.0.0.0 \
  -e PORT=3000 \
  -e ASTRO_EPHE_PATH=/ephe/de440.bsp \
  -e ASTRO_EPHE_CACHE_DIR=/tmp/ephe \
  daanyam-astro-api:rc
```

## Readiness and alerts

Readiness:

- treat the service as ready when `GET /health` returns HTTP `200`

Recommended alerting:

- sustained 5xx error rate above normal baseline
- p95 latency regression on `POST /positions`, `POST /positions/sidereal`, or `POST /chart/sidereal`
- repeated container restarts or healthcheck failures

Monitoring readiness:

- `astro-api` emits one structured request log line per request with `request_id`, `request_body_hash`, `latency_ms`, HTTP method/path/status, and Cloud Run trace linkage when `x-cloud-trace-context` is present
- every response includes `x-request-id`; callers may send `x-request-id` or `x-correlation-id` to preserve an upstream identifier end to end
- Cloud Run uptime checks can target unauthenticated `GET /health`
- Recommended first alert policies:
  - uptime-check failure on `/health`
  - 5xx count or error-rate alert on the Cloud Run service
  - p95 latency alert from Cloud Run `request_latencies`
- Reproducible setup and verification: [docs/monitoring.md](monitoring.md)
  - setup helper: [deploy/cloudrun/setup_monitoring.sh](../deploy/cloudrun/setup_monitoring.sh)
  - verification helper: [deploy/cloudrun/post_deploy_verify.sh](../deploy/cloudrun/post_deploy_verify.sh)

## Rough minimum sizing

Starting point for a single small instance:

- CPU: `1 vCPU`
- Memory: `512 MiB`

Increase memory headroom if colocating additional services or if future ephemeris caching expands.

## GitHub Actions to Cloud Run

The default deployment path is:

1. Open a PR and let [`.github/workflows/ci.yml`](../.github/workflows/ci.yml) pass.
2. Merge to `main`.
3. Let [`.github/workflows/deploy.yml`](../.github/workflows/deploy.yml) build and push the image to Artifact Registry.
4. Deploy the rendered [deploy/cloudrun/service.yaml](../deploy/cloudrun/service.yaml) manifest to Cloud Run (inject image URI, service account, and bucket placeholders).
5. Configure **secrets** for `VALID_API_KEYS` (and optional `RATE_LIMIT_RPM`) in Cloud Run or your secret store; reference them in the service manifest if you use Secret Manager.
6. Grant **unauthenticated** invoker access only if you need public `GET /health` without API keys (typical for probes). `POST` routes still require `VALID_API_KEYS`.
7. Smoke test `GET /health` on the deployed URL.
8. Run the deployed contract suite (see below).

Repository configuration expected by `deploy.yml`:

- GitHub repository variables:
  - `GCP_PROJECT_ID`
  - `GCP_REGION` (optional, defaults to `asia-south1`)
  - `GAR_REPOSITORY` (optional, defaults to `astro-engine`)
  - `CLOUD_RUN_SERVICE` (optional, defaults to `astro-api`)
  - `CLOUD_RUN_EPHEMERIS_BUCKET` (optional, defaults to `daanyam-ephe`)
  - `CLOUD_RUN_SERVICE_ACCOUNT`
- GitHub repository secrets:
  - `GCP_WORKLOAD_IDENTITY_PROVIDER`
  - `GCP_DEPLOY_SERVICE_ACCOUNT`

The Cloud Run service manifest mounts the configured GCS bucket read-only at `/tmp/ephe` using the Cloud Run GCS Fuse CSI driver, sets `ASTRO_EPHE_PATH=/tmp/ephe/de440.bsp`, and keeps `min-instances=1` as the production default to reduce cold-start impact.

## Contract tests (local shell; not server env vars)

These are **only for developers/CI** hitting a running base URL. They are not read by the `astro-api` binary:

| Variable | Purpose |
| --- | --- |
| `ASTRO_API_BASE_URL` | HTTPS origin of the deployed service (no trailing slash). When unset, `production_contract` integration tests skip. |
| `ASTRO_API_KEY` | One of the keys from `VALID_API_KEYS` on that deployment. Required for authenticated contract tests (sidereal + chart). |
| `ASTRO_CONTRACT_ASSERT_RATE_LIMIT` | Set to `1` to run the optional deployed rate-limit assertion. The service must use a low `RATE_LIMIT_RPM` (for example `2`) for that run; otherwise the test will fail. |

Run locally:

```bash
ASTRO_API_BASE_URL="https://your-cloud-run-url" \
ASTRO_API_KEY="your-valid-key" \
cargo test -p astro-api --test production_contract
```

In CI against staging, store the URL and a staging key as protected variables and export them before the test step.

## Rollout notes

1. **Config**: Set `VALID_API_KEYS` before sending production traffic; rotate keys via comma-separated list. Align `RATE_LIMIT_RPM` with product expectations (or leave unset while tuning).
2. **Image**: Deploy the new revision; confirm startup logs show DE440 loaded when using `ASTRO_BACKEND=de440` (and coverage log line from `main.rs` when applicable).
3. **Smoke**: `GET /health` returns `200` and JSON `status=ok` with a non-empty `version`.
4. **Auth**: Unauthenticated `POST /positions/sidereal` returns `401` and `missing_api_key`; wrong key returns `invalid_api_key`.
5. **Contracts**: Run `cargo test -p astro-api --test production_contract` with `ASTRO_API_BASE_URL` and `ASTRO_API_KEY` (see [Environment templates](#environment-templates)).
6. **Optional rate limit**: For a dedicated check, temporarily set `RATE_LIMIT_RPM=2` on one revision, export `ASTRO_CONTRACT_ASSERT_RATE_LIMIT=1`, run the suite, then restore production limits.

## Staging vs production verification checklist

Use the same checklist in both; only credentials and URLs differ.

### Staging

1. **Reachability**: `curl -sS "$ASTRO_API_BASE_URL/health"` → HTTP 200, JSON includes `"status":"ok"` and `"version"`.
2. **Correlation**: Response includes `x-request-id` (optionally send `x-correlation-id` and confirm it echoes as `x-request-id`).
3. **Auth (negative)**: `POST /positions/sidereal` with no `x-api-key` / Bearer → 401, body `{"error":"missing_api_key"}`.
4. **Auth (negative)**: Same with a wrong key → 401, `{"error":"invalid_api_key"}`.
5. **Auth (positive)**: Same payload with a valid staging key from `VALID_API_KEYS` → 200 and `metadata.engine_mode` present.
6. **Chart contract**: `POST /chart/sidereal` with compact `projection=sidereal_only` (valid key) → `data.schema_version` is `chart_sidereal_v1`, `data.houses` and `data.dasha` null when `compact=true`.
7. **OpenAPI / docs**: `GET /openapi.json` and browser `GET /docs` load without a key.
8. **Automated**: `cargo test -p astro-api --test production_contract` with `ASTRO_API_BASE_URL` and `ASTRO_API_KEY` for staging.
9. **Rate limit (optional)**: With `RATE_LIMIT_RPM` set low on staging, third authenticated request within the window returns 429 with `retry-after` and `rate_limit_exceeded`, or run unit tests (`cargo test -p astro-api rate_limit_returns_429`).

### Production

1. Same as staging steps 1–7 using the production URL and a production-issued key (never reuse staging keys).
2. Confirm alert policies from [docs/monitoring.md](monitoring.md) are enabled and green after deploy.
3. Confirm Cloud Run revision has expected env: `ASTRO_BACKEND=de440`, `ENVIRONMENT=production`, `ASTRO_EPHE_PATH` pointing at the mounted kernel, `VALID_API_KEYS` from Secret Manager or equivalent.
4. **Automated**: Run the same `production_contract` command against the production URL with a production key in a secure CI context only.

## Cloud Build fallback

If GitHub Actions is unavailable, [cloudbuild.yaml](../cloudbuild.yaml) provides the same build, push, deploy, and `/health` smoke-test path inside GCP.

## Environment templates

- Staging: [docs/env.staging.example](env.staging.example)
- Production: [docs/env.production.example](env.production.example)
