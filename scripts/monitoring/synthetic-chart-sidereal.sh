#!/usr/bin/env bash
# Synthetic transaction: POST /chart/sidereal (Delhi 1990) and assert lagna longitude.
# Suitable for cron, Cloud Scheduler → Cloud Run job, or CI smoke against staging/production.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
FIXTURE="${REPO_ROOT}/tests/golden/synthetic/delhi-1990-chart.json"

BASE_URL="${ASTRO_API_BASE_URL:-}"
API_KEY="${ASTRO_API_KEY:-}"
TOLERANCE="${SYNTHETIC_TOLERANCE_DEG:-1e-6}"

if [[ -z "${BASE_URL}" || -z "${API_KEY}" ]]; then
  cat <<'EOF'
Synthetic chart monitor — environment not configured.

Set:
  export ASTRO_API_BASE_URL="https://your-cloud-run-url"
  export ASTRO_API_KEY="your-valid-key"

Optional:
  export SYNTHETIC_LAGNA_EXPECTED_DEG=275.1573701670353
  export SYNTHETIC_TOLERANCE_DEG=1e-6

Then re-run:
  ./scripts/monitoring/synthetic-chart-sidereal.sh
EOF
  exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required" >&2
  exit 1
fi

if [[ ! -f "${FIXTURE}" ]]; then
  echo "missing fixture: ${FIXTURE}" >&2
  exit 1
fi

BASE_URL="${BASE_URL%/}"
TARGET="${BASE_URL}/chart/sidereal"
BODY="$(jq -c '.request' "${FIXTURE}")"
EXPECTED="${SYNTHETIC_LAGNA_EXPECTED_DEG:-$(jq -r '.expected_lagna_sidereal_longitude_deg' "${FIXTURE}")}"
EXPECTED_RASHI="$(jq -r '.expected_lagna_rashi' "${FIXTURE}")"

RESPONSE="$(mktemp)"
trap 'rm -f "${RESPONSE}"' EXIT

HTTP_CODE="$(curl -sS -o "${RESPONSE}" -w '%{http_code}' \
  -X POST "${TARGET}" \
  -H "content-type: application/json" \
  -H "x-api-key: ${API_KEY}" \
  -d "${BODY}")"

if [[ "${HTTP_CODE}" != "200" ]]; then
  echo "synthetic chart: HTTP ${HTTP_CODE}" >&2
  head -c 500 "${RESPONSE}" >&2 || true
  exit 1
fi

ACTUAL="$(jq -r '.data.lagna.sidereal_longitude_deg' "${RESPONSE}")"
RASHI="$(jq -r '.data.lagna.rashi' "${RESPONSE}")"

if [[ "${RASHI}" != "${EXPECTED_RASHI}" ]]; then
  echo "synthetic chart: lagna rashi mismatch (got ${RASHI}, expected ${EXPECTED_RASHI})" >&2
  exit 1
fi

DELTA="$(awk -v a="${ACTUAL}" -v e="${EXPECTED}" -v t="${TOLERANCE}" \
  'BEGIN { d = (a - e); if (d < 0) d = -d; if (d > t) exit 1; print d }')" || {
  echo "synthetic chart: lagna longitude mismatch (got ${ACTUAL}, expected ${EXPECTED}, tolerance ${TOLERANCE})" >&2
  exit 1
}

echo "synthetic chart ok: lagna_sidereal_longitude_deg=${ACTUAL} (delta=${DELTA} <= ${TOLERANCE}), rashi=${RASHI}"
