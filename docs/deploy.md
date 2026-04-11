# Deploying `astro-api`

## Runtime contract

- Required environment variable:
  - `ASTRO_EPHE_PATH`
- Configurable environment variables:
  - `ASTRO_BACKEND`
  - `ALLOW_DEMO_BACKEND`
  - `ENVIRONMENT`
  - `NODE_ENV`
  - `HOST`
  - `PORT`
  - `RUST_LOG` (optional)

Recommended kernel mount:

- Mount the JPL kernel read-only at `/ephe/de440.bsp`
- Set `ASTRO_EPHE_PATH=/ephe/de440.bsp`
- Set `ASTRO_BACKEND=de440` for real ephemeris-backed runtime

## Demo backend guard

- Production-like environments must not start with `ASTRO_BACKEND=demo` by accident.
- If `ASTRO_BACKEND=demo` and either `ENVIRONMENT=production` or `NODE_ENV=production`, startup fails fast by default.
- Override only for intentional demos with `ALLOW_DEMO_BACKEND=true`.
- Recommended production setting:
  - `ASTRO_BACKEND=de440`
  - leave `ALLOW_DEMO_BACKEND` unset or set it to `false`

## Container notes

- Dockerfile: [Dockerfile](/Users/sanjeet/Desktop/daanyam%20-%20astro%20engine/Dockerfile)
- Docker Compose sample: [deploy/docker-compose.yml](/Users/sanjeet/Desktop/daanyam%20-%20astro%20engine/deploy/docker-compose.yml)
- Kubernetes samples:
  - [deploy/k8s/deployment.yaml](/Users/sanjeet/Desktop/daanyam%20-%20astro%20engine/deploy/k8s/deployment.yaml)
  - [deploy/k8s/service.yaml](/Users/sanjeet/Desktop/daanyam%20-%20astro%20engine/deploy/k8s/service.yaml)
- Cloud Run manifest: [deploy/cloudrun/service.yaml](/Users/sanjeet/Desktop/daanyam%20-%20astro%20engine/deploy/cloudrun/service.yaml)
- GitHub Actions:
  - [`.github/workflows/ci.yml`](/Users/sanjeet/Desktop/daanyam%20-%20astro%20engine/.github/workflows/ci.yml)
  - [`.github/workflows/deploy.yml`](/Users/sanjeet/Desktop/daanyam%20-%20astro%20engine/.github/workflows/deploy.yml)
- Cloud Build fallback: [cloudbuild.yaml](/Users/sanjeet/Desktop/daanyam%20-%20astro%20engine/cloudbuild.yaml)
- Default container port: `3000`
- Health endpoint: `GET /health`

Example container run:

```bash
docker build -t daanyam-astro-api:rc .
docker run --rm \
  -p 3000:3000 \
  --mount type=bind,source=/tmp/astro-ephe/de440.bsp,target=/ephe/de440.bsp,readonly \
  -e ASTRO_BACKEND=de440 \
  -e ENVIRONMENT=production \
  -e HOST=0.0.0.0 \
  -e PORT=3000 \
  -e ASTRO_EPHE_PATH=/ephe/de440.bsp \
  daanyam-astro-api:rc
```

## Readiness and alerts

Readiness:

- treat the service as ready when `GET /health` returns HTTP `200`

Recommended alerting:

- sustained 5xx error rate above normal baseline
- p95 latency regression on `POST /positions`, `POST /positions/sidereal`, or `POST /chart/sidereal`
- repeated container restarts or healthcheck failures

## Rough minimum sizing

Starting point for a single small instance:

- CPU: `1 vCPU`
- Memory: `512 MiB`

Increase memory headroom if colocating additional services or if future ephemeris caching expands.

## GitHub Actions to Cloud Run

The default Sprint 1 deployment path is:

1. open a PR and let [`.github/workflows/ci.yml`](/Users/sanjeet/Desktop/daanyam%20-%20astro%20engine/.github/workflows/ci.yml) pass
2. merge to `main`
3. let [`.github/workflows/deploy.yml`](/Users/sanjeet/Desktop/daanyam%20-%20astro%20engine/.github/workflows/deploy.yml) build and push the image to Artifact Registry
4. deploy the rendered [deploy/cloudrun/service.yaml](/Users/sanjeet/Desktop/daanyam%20-%20astro%20engine/deploy/cloudrun/service.yaml) manifest to Cloud Run
5. grant unauthenticated invoker access so `GET /health` can be probed without a signed request
6. smoke test `GET /health` on the deployed service URL

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

The Cloud Run service manifest mounts the configured GCS bucket read-only at `/ephe` using the Cloud Run GCS Fuse CSI driver and sets `ASTRO_EPHE_PATH=/ephe/de440.bsp`.

## Cloud Build fallback

If GitHub Actions is unavailable, [cloudbuild.yaml](/Users/sanjeet/Desktop/daanyam%20-%20astro%20engine/cloudbuild.yaml) provides the same build, push, deploy, and `/health` smoke-test path inside GCP.

## Environment templates

- Staging: [docs/env.staging.example](/Users/sanjeet/Desktop/daanyam%20-%20astro%20engine/docs/env.staging.example)
- Production: [docs/env.production.example](/Users/sanjeet/Desktop/daanyam%20-%20astro%20engine/docs/env.production.example)
