#!/usr/bin/env bash
# Load test: 50 RPS for 5 minutes against POST /chart/sidereal with rotating birth fixtures.
# Requires ASTRO_API_BASE_URL and ASTRO_API_KEY (see docs/deploy.md).

set -euo pipefail

BASE_URL="${ASTRO_API_BASE_URL:-}"
API_KEY="${ASTRO_API_KEY:-}"
DURATION="${LOAD_DURATION:-5m}"
RATE="${LOAD_RATE:-50}"

if [[ -z "${BASE_URL}" || -z "${API_KEY}" ]]; then
  cat <<'EOF'
Sprint 2 baseline load test — environment not configured.

Set:
  export ASTRO_API_BASE_URL="https://your-cloud-run-url"   # no trailing slash
  export ASTRO_API_KEY="your-valid-key"                    # from VALID_API_KEYS

Optional:
  export LOAD_RATE=50          # requests per second (default 50)
  export LOAD_DURATION=5m      # duration (default 5m)

Then re-run:
  ./scripts/load/baseline-chart-sidereal.sh

Do not run against production without explicit approval. Record results in docs/perf/baseline-w2.md.
EOF
  exit 0
fi

BASE_URL="${BASE_URL%/}"
TARGET="${BASE_URL}/chart/sidereal"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
FIXTURE_DIR="$(mktemp -d)"
trap 'rm -rf "${FIXTURE_DIR}"' EXIT

cat >"${FIXTURE_DIR}/delhi-1990.json" <<'JSON'
{"datetime":{"kind":"utc","utc":"1990-05-17T04:30:00Z"},"geo":{"latitude_deg":28.6139,"longitude_deg":77.2090,"elevation_m":216.0},"ayanamsa":"lahiri","compact":true,"projection":"sidereal_only"}
JSON

cat >"${FIXTURE_DIR}/bangalore-2000.json" <<'JSON'
{"datetime":{"kind":"utc","utc":"2000-01-01T12:00:00Z"},"geo":{"latitude_deg":12.9716,"longitude_deg":77.5946,"elevation_m":920.0},"ayanamsa":"lahiri","compact":true,"projection":"sidereal_only"}
JSON

cat >"${FIXTURE_DIR}/independence-1947.json" <<'JSON'
{"datetime":{"kind":"utc","utc":"1947-08-15T00:00:00Z"},"geo":{"latitude_deg":12.9716,"longitude_deg":77.5946,"elevation_m":920.0},"ayanamsa":"lahiri","compact":true,"projection":"sidereal_only"}
JSON

cat >"${FIXTURE_DIR}/bangalore-1995.json" <<'JSON'
{"datetime":{"kind":"utc","utc":"1995-08-12T09:00:00Z"},"geo":{"latitude_deg":12.9716,"longitude_deg":77.5946,"elevation_m":920.0},"ayanamsa":"lahiri","compact":true,"projection":"sidereal_only"}
JSON

echo "Target: POST ${TARGET}"
echo "Rate: ${RATE} RPS for ${DURATION}"
echo "Fixtures: ${FIXTURE_DIR}/*.json (rotating)"

if command -v oha >/dev/null 2>&1; then
  echo "Tool: oha"
  # oha does not rotate bodies natively; run one fixture per time slice (4 × 75s ≈ 5m at 50 RPS).
  SLICE_DURATION="${LOAD_SLICE_DURATION:-75s}"
  for fixture in "${FIXTURE_DIR}"/*.json; do
    name="$(basename "${fixture}" .json)"
    echo "--- ${name} (${SLICE_DURATION} @ ${RATE} RPS) ---"
    oha -n "${RATE}" -c 50 -z "${SLICE_DURATION}" \
      -m POST \
      -H "content-type: application/json" \
      -H "x-api-key: ${API_KEY}" \
      -d @"${fixture}" \
      "${TARGET}" || true
  done
  echo "Aggregate p50/p95/p99 from oha output above into docs/perf/baseline-w2.md"
  exit 0
fi

if command -v vegeta >/dev/null 2>&1; then
  echo "Tool: vegeta"
  TARGETS_FILE="${FIXTURE_DIR}/targets.txt"
  : >"${TARGETS_FILE}"
  for fixture in "${FIXTURE_DIR}"/*.json; do
    body="$(tr -d '\n' <"${fixture}")"
    printf 'POST %s\nContent-Type: application/json\nX-Api-Key: %s\n\n%s\n\n' \
      "${TARGET}" "${API_KEY}" "${body}" >>"${TARGETS_FILE}"
  done
  vegeta attack -duration="${DURATION}" -rate="${RATE}" -targets="${TARGETS_FILE}" \
    | tee "${FIXTURE_DIR}/vegeta.bin" \
    | vegeta report -type=text
  vegeta report -type="hist[0,50ms,100ms,200ms,500ms,1s,2s]" "${FIXTURE_DIR}/vegeta.bin"
  echo "Copy p50/p95/p99 and error rate into docs/perf/baseline-w2.md"
  exit 0
fi

cat <<'EOF'
Neither `oha` nor `vegeta` is installed.

Install one of:
  brew install oha
  brew install vegeta

Or run manually, for example with vegeta:
  export ASTRO_API_BASE_URL=...
  export ASTRO_API_KEY=...
  ./scripts/load/baseline-chart-sidereal.sh
EOF
exit 1
