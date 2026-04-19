#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEMPLATE_DIR="${SCRIPT_DIR}/alert-policies"

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required command: $1" >&2
    exit 1
  fi
}

require_command gcloud
require_command sed
require_command mktemp

PROJECT_ID="${PROJECT_ID:-$(gcloud config get-value project 2>/dev/null || true)}"
SERVICE_NAME="${SERVICE_NAME:-astro-api}"
REGION="${REGION:-asia-south1}"
SERVICE_URL="${SERVICE_URL:-}"
SERVICE_HOST="${SERVICE_HOST:-}"
UPTIME_CHECK_PERIOD="${UPTIME_CHECK_PERIOD:-1}"
ERROR_COUNT_THRESHOLD="${ERROR_COUNT_THRESHOLD:-5}"
LATENCY_THRESHOLD_MS="${LATENCY_THRESHOLD_MS:-1500}"
NOTIFICATION_CHANNELS="${NOTIFICATION_CHANNELS:-}"

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

if [[ -z "${SERVICE_URL}" && -z "${SERVICE_HOST}" ]]; then
  echo "unable to resolve Cloud Run URL; set SERVICE_URL or SERVICE_HOST explicitly" >&2
  exit 1
fi

if [[ -z "${SERVICE_HOST}" ]]; then
  SERVICE_HOST="${SERVICE_URL#https://}"
  SERVICE_HOST="${SERVICE_HOST#http://}"
  SERVICE_HOST="${SERVICE_HOST%%/*}"
fi

UPTIME_DISPLAY_NAME="${UPTIME_DISPLAY_NAME:-${SERVICE_NAME} health}"
UPTIME_POLICY_DISPLAY_NAME="${UPTIME_POLICY_DISPLAY_NAME:-${SERVICE_NAME} uptime failure}"
ERROR_POLICY_DISPLAY_NAME="${ERROR_POLICY_DISPLAY_NAME:-${SERVICE_NAME} 5xx count}"
LATENCY_POLICY_DISPLAY_NAME="${LATENCY_POLICY_DISPLAY_NAME:-${SERVICE_NAME} p95 latency}"

json_escape() {
  printf '%s' "$1" | sed 's/[\/&]/\\&/g'
}

notification_channels_json() {
  if [[ -z "${NOTIFICATION_CHANNELS}" ]]; then
    printf '[]'
    return
  fi

  local channel
  local first=1
  printf '['
  IFS=',' read -r -a channels <<< "${NOTIFICATION_CHANNELS}"
  for channel in "${channels[@]}"; do
    channel="${channel#"${channel%%[![:space:]]*}"}"
    channel="${channel%"${channel##*[![:space:]]}"}"
    [[ -z "${channel}" ]] && continue
    if [[ ${first} -eq 0 ]]; then
      printf ','
    fi
    first=0
    printf '"%s"' "${channel}"
  done
  printf ']'
}

lookup_uptime_check_name() {
  gcloud monitoring uptime list-configs \
    --project "${PROJECT_ID}" \
    --filter="displayName=\"${UPTIME_DISPLAY_NAME}\"" \
    --format='value(name)'
}

ensure_uptime_check() {
  local existing_name
  existing_name="$(lookup_uptime_check_name | head -n1)"

  if [[ -z "${existing_name}" ]]; then
    gcloud monitoring uptime create "${UPTIME_DISPLAY_NAME}" \
      --project "${PROJECT_ID}" \
      --resource-type=uptime-url \
      --resource-labels="host=${SERVICE_HOST},project_id=${PROJECT_ID}" \
      --path=/health \
      --protocol=https \
      --request-method=get \
      --status-classes=2xx \
      --period="${UPTIME_CHECK_PERIOD}" \
      --timeout=10 \
      --regions=usa-iowa,usa-oregon,usa-virginia \
      --validate-ssl=true
    existing_name="$(lookup_uptime_check_name | head -n1)"
  else
    gcloud monitoring uptime update "${existing_name}" \
      --project "${PROJECT_ID}" \
      --display-name="${UPTIME_DISPLAY_NAME}" \
      --path=/health \
      --set-status-classes=2xx \
      --period="${UPTIME_CHECK_PERIOD}" \
      --timeout=10 \
      --set-regions=usa-iowa,usa-oregon,usa-virginia \
      --clear-status-codes=true \
      --validate-ssl=true
  fi

  if [[ -z "${existing_name}" ]]; then
    echo "failed to create or resolve uptime check ${UPTIME_DISPLAY_NAME}" >&2
    exit 1
  fi

  printf '%s\n' "${existing_name}"
}

render_policy() {
  local template_path="$1"
  local output_path="$2"
  local policy_display_name="$3"
  local uptime_check_id="$4"
  local channels_json
  channels_json="$(notification_channels_json)"

  sed \
    -e "s|__POLICY_DISPLAY_NAME__|$(json_escape "${policy_display_name}")|g" \
    -e "s|__SERVICE_NAME__|$(json_escape "${SERVICE_NAME}")|g" \
    -e "s|__REGION__|$(json_escape "${REGION}")|g" \
    -e "s|__UPTIME_CHECK_ID__|$(json_escape "${uptime_check_id}")|g" \
    -e "s|__ERROR_COUNT_THRESHOLD__|${ERROR_COUNT_THRESHOLD}|g" \
    -e "s|__LATENCY_THRESHOLD_MS__|${LATENCY_THRESHOLD_MS}|g" \
    -e "s|__NOTIFICATION_CHANNELS__|$(json_escape "${channels_json}")|g" \
    "${template_path}" > "${output_path}"
}

lookup_policy_name() {
  local display_name="$1"
  gcloud monitoring policies list \
    --project "${PROJECT_ID}" \
    --filter="displayName=\"${display_name}\"" \
    --format='value(name)' | head -n1
}

apply_policy() {
  local display_name="$1"
  local template_file="$2"
  local uptime_check_id="$3"
  local tmp_file
  local existing_policy

  tmp_file="$(mktemp)"
  render_policy "${TEMPLATE_DIR}/${template_file}" "${tmp_file}" "${display_name}" "${uptime_check_id}"
  existing_policy="$(lookup_policy_name "${display_name}")"

  if [[ -z "${existing_policy}" ]]; then
    gcloud monitoring policies create \
      --project "${PROJECT_ID}" \
      --policy-from-file="${tmp_file}"
  else
    gcloud monitoring policies update "${existing_policy}" \
      --project "${PROJECT_ID}" \
      --policy-from-file="${tmp_file}"
  fi

  rm -f "${tmp_file}"
}

echo "Using project: ${PROJECT_ID}"
echo "Using service: ${SERVICE_NAME}"
echo "Using region: ${REGION}"
echo "Using host: ${SERVICE_HOST}"

UPTIME_CHECK_NAME="$(ensure_uptime_check)"
UPTIME_CHECK_ID="${UPTIME_CHECK_NAME##*/}"

apply_policy "${UPTIME_POLICY_DISPLAY_NAME}" "uptime_failure.json.tmpl" "${UPTIME_CHECK_ID}"
apply_policy "${ERROR_POLICY_DISPLAY_NAME}" "5xx_count.json.tmpl" "${UPTIME_CHECK_ID}"
apply_policy "${LATENCY_POLICY_DISPLAY_NAME}" "p95_latency.json.tmpl" "${UPTIME_CHECK_ID}"

echo "Created or updated:"
echo "- uptime check: ${UPTIME_CHECK_NAME}"
echo "- alert policy: ${UPTIME_POLICY_DISPLAY_NAME}"
echo "- alert policy: ${ERROR_POLICY_DISPLAY_NAME}"
echo "- alert policy: ${LATENCY_POLICY_DISPLAY_NAME}"
