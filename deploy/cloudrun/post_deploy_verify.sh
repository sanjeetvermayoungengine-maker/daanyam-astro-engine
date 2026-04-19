#!/usr/bin/env bash
set -euo pipefail

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required command: $1" >&2
    exit 1
  fi
}

require_command gcloud
require_command curl
require_command cargo

PROJECT_ID="${PROJECT_ID:-$(gcloud config get-value project 2>/dev/null || true)}"
SERVICE_NAME="${SERVICE_NAME:-astro-api}"
REGION="${REGION:-asia-south1}"
SERVICE_URL="${SERVICE_URL:-}"

if [[ -z "${PROJECT_ID}" ]]; then
  echo "set PROJECT_ID or configure a default gcloud project" >&2
  exit 1
fi

if [[ -z "${SERVICE_URL}" ]]; then
  SERVICE_URL="$(gcloud run services describe "${SERVICE_NAME}" \
    --project "${PROJECT_ID}" \
    --region "${REGION}" \
    --format='value(status.url)')"
fi

if [[ -z "${SERVICE_URL}" ]]; then
  echo "unable to resolve Cloud Run URL; set SERVICE_URL explicitly" >&2
  exit 1
fi

UPTIME_DISPLAY_NAME="${UPTIME_DISPLAY_NAME:-${SERVICE_NAME} health}"
UPTIME_POLICY_DISPLAY_NAME="${UPTIME_POLICY_DISPLAY_NAME:-${SERVICE_NAME} uptime failure}"
ERROR_POLICY_DISPLAY_NAME="${ERROR_POLICY_DISPLAY_NAME:-${SERVICE_NAME} 5xx count}"
LATENCY_POLICY_DISPLAY_NAME="${LATENCY_POLICY_DISPLAY_NAME:-${SERVICE_NAME} p95 latency}"

echo "Verifying deployed health endpoint"
curl --fail --show-error --silent "${SERVICE_URL}/health"
echo

echo "Running deployed contract suite"
ASTRO_API_BASE_URL="${SERVICE_URL}" cargo test -p astro-api --test production_contract

echo "Checking uptime check presence"
UPTIME_CHECK_NAME="$(gcloud monitoring uptime list-configs \
  --project "${PROJECT_ID}" \
  --filter="displayName=\"${UPTIME_DISPLAY_NAME}\"" \
  --format='value(name)' | head -n1)"
if [[ -z "${UPTIME_CHECK_NAME}" ]]; then
  echo "missing uptime check: ${UPTIME_DISPLAY_NAME}" >&2
  exit 1
fi

echo "Checking alert policies"
for display_name in \
  "${UPTIME_POLICY_DISPLAY_NAME}" \
  "${ERROR_POLICY_DISPLAY_NAME}" \
  "${LATENCY_POLICY_DISPLAY_NAME}"; do
  policy_name="$(gcloud monitoring policies list \
    --project "${PROJECT_ID}" \
    --filter="displayName=\"${display_name}\"" \
    --format='value(name)' | head -n1)"
  if [[ -z "${policy_name}" ]]; then
    echo "missing alert policy: ${display_name}" >&2
    exit 1
  fi
  echo "- ${display_name}: ${policy_name}"
done

echo "Verification passed for ${SERVICE_URL}"
