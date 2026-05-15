# Release Candidate Runbook

## v0.18.0 — Phase 1 close

| Step | Action |
|------|--------|
| Version | `ENGINE_SEMANTIC_VERSION = "0.18.0"` in `crates/astro-api/src/lib.rs` |
| Changelog | [CHANGELOG.md](CHANGELOG.md) `## 0.18.0` |
| Verify | `export ASTRO_EPHE_PATH=./ephe/de440.bsp && cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` |
| OpenAPI | `cargo run -p astro-api --bin write_openapi` → commit `dist/openapi.json` |
| Tag | `git tag -a v0.18.0 -m "Phase 1 close"` |
| Push tag | `git push origin v0.18.0` — triggers [.github/workflows/deploy.yml](.github/workflows/deploy.yml) |
| Smoke | `curl -sS https://<cloud-run>/provenance \| jq` — expect extended manifest + `Cache-Control` |

Docker build args (optional): `GIT_COMMIT`, `BUILD_DATE` (RFC3339 UTC) passed at image build for `/provenance` metadata.

Evidence: [docs/runbooks/phase1-checklist-evidence.md](docs/runbooks/phase1-checklist-evidence.md). Retrospective: [docs/retros/phase1.md](docs/retros/phase1.md).

## Local API run

1. Ensure the Rust toolchain is installed and available on `PATH`.
2. Place the JPL kernel file at a local path such as `/tmp/astro-ephe/de440.bsp` or any other path you control.
3. Export the kernel path:
   - `export ASTRO_EPHE_PATH=/tmp/astro-ephe/de440.bsp`
4. Select the runtime backend:
   - `export ASTRO_BACKEND=demo` for quick local startup
   - `export ASTRO_BACKEND=de440` for real ephemeris-backed runtime
   - if `ENVIRONMENT=production` or `NODE_ENV=production`, `ASTRO_BACKEND=demo` is rejected unless `ALLOW_DEMO_BACKEND=true`
5. Start the API locally from the workspace root:
   - `cargo run -p astro-api`
6. Verify the service:
   - `curl http://127.0.0.1:3000/health`
   - `curl http://127.0.0.1:3000/openapi.json`

## Required environment variables

- `ASTRO_EPHE_PATH`
  - Required for real DE440-backed astronomy paths.
  - Must point to a readable `de440.bsp` file outside the repo.

Optional runtime variables:

- `ASTRO_BACKEND`
  - `demo` for local quickstart
  - `de440` for real ephemeris-backed runtime
- `HOST`
  - Defaults to `127.0.0.1`
- `PORT`
  - Defaults to `3000`
- `ALLOW_DEMO_BACKEND`
  - Defaults to `false`
  - Set to `true` only for intentional demo deployments in production-like environments
- `ENVIRONMENT` / `NODE_ENV`
  - If either is `production`, startup rejects `ASTRO_BACKEND=demo` unless `ALLOW_DEMO_BACKEND=true`
- `RUST_LOG`
  - Optional logging level for local runs and containers

## Kernel file placement

- Do not commit `de440.bsp` into the repository.
- Recommended local placement:
  - `/tmp/astro-ephe/de440.bsp`
- Alternate placement:
  - any readable filesystem path exported through `ASTRO_EPHE_PATH`

## Release artifacts

- OpenAPI artifact:
  - `dist/openapi.json`
- OpenAPI checksum:
  - `dist/openapi.json.sha256`
- Linux x86_64 binary archive:
  - `dist/astro-api-x86_64-unknown-linux-gnu.tar.gz`
- Linux x86_64 binary checksum:
  - `dist/astro-api-x86_64-unknown-linux-gnu.tar.gz.sha256`
- Generate or refresh the artifact from the same source as `GET /openapi.json`:
  - `cargo run -p astro-api --bin write_openapi`
- Build the Linux x86_64 release binary locally:
  - `cargo build -p astro-api --release --target x86_64-unknown-linux-gnu`
- Package the Linux x86_64 release binary locally:
  - `tar -C target/x86_64-unknown-linux-gnu/release -czf dist/astro-api-x86_64-unknown-linux-gnu.tar.gz astro-api`

## Downloading artifacts

- Local RC consumers can copy artifacts directly from the workspace:
  - `dist/openapi.json`
  - `dist/openapi.json.sha256`
  - `dist/astro-api-x86_64-unknown-linux-gnu.tar.gz`
  - `dist/astro-api-x86_64-unknown-linux-gnu.tar.gz.sha256`
- When packaging a release bundle, include:
  - `dist/openapi.json`
  - `dist/openapi.json.sha256`
  - `dist/astro-api-x86_64-unknown-linux-gnu.tar.gz`
  - `dist/astro-api-x86_64-unknown-linux-gnu.tar.gz.sha256`

## Linux libc compatibility

- Tagged releases currently publish `x86_64-unknown-linux-gnu`, which links against glibc.
- That matches the current Debian-based container and typical Ubuntu/Debian server deployments.
- Alpine and other musl-based runtimes need either the container image or a separate musl build such as `x86_64-unknown-linux-musl`.

## Deterministic contract verification

- No external network is required.
- Contract checks read only local JSON artifacts and examples:
  - `cargo test -p astro-api --test contract_artifacts`

## Deployment guidance

- Container/runtime guidance lives in [docs/deploy.md](/Users/sanjeet/Documents/Playground/docs/deploy.md).
- Tag pushes matching `v*` publish `ghcr.io/<owner>/astro-api:<tag>` and attach:
  - `dist/openapi.json`
  - `dist/openapi.json.sha256`
  - `dist/astro-api-x86_64-unknown-linux-gnu.tar.gz`
  - `dist/astro-api-x86_64-unknown-linux-gnu.tar.gz.sha256`
