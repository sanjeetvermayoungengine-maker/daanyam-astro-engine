#!/usr/bin/env bash
# Idempotent Cloud Logging → BigQuery sink for astro-api Cloud Run logs.
# Requires: gcloud, bq; authenticated with permissions to create sinks and datasets.
set -euo pipefail

: "${GCP_PROJECT:?Set GCP_PROJECT to your GCP project id}"

BQ_DATASET="${BQ_DATASET:-astro_engine_logs}"
BQ_LOCATION="${BQ_LOCATION:-asia-south1}"
CLOUD_RUN_SERVICE="${CLOUD_RUN_SERVICE:-astro-api}"
SINK_ID="${SINK_ID:-astro-api-logs-to-bq}"

LOG_FILTER=$'resource.type="cloud_run_revision"\nresource.labels.service_name="'"${CLOUD_RUN_SERVICE}"'"'

echo "Project: ${GCP_PROJECT}"
echo "Dataset: ${BQ_DATASET} (${BQ_LOCATION})"
echo "Service filter: ${CLOUD_RUN_SERVICE}"
echo "Sink: ${SINK_ID}"

if ! bq show --project_id="${GCP_PROJECT}" "${BQ_DATASET}" >/dev/null 2>&1; then
  echo "Creating BigQuery dataset ${BQ_DATASET}..."
  bq --location="${BQ_LOCATION}" mk \
    --dataset \
    --description="Astro engine Cloud Run structured logs" \
    "${GCP_PROJECT}:${BQ_DATASET}"
else
  echo "Dataset ${BQ_DATASET} already exists."
fi

DESTINATION="bigquery.googleapis.com/projects/${GCP_PROJECT}/datasets/${BQ_DATASET}"

if gcloud logging sinks describe "${SINK_ID}" --project="${GCP_PROJECT}" >/dev/null 2>&1; then
  echo "Updating existing sink ${SINK_ID}..."
  gcloud logging sinks update "${SINK_ID}" \
    --project="${GCP_PROJECT}" \
    --log-filter="${LOG_FILTER}"
else
  echo "Creating sink ${SINK_ID}..."
  gcloud logging sinks create "${SINK_ID}" "${DESTINATION}" \
    --project="${GCP_PROJECT}" \
    --log-filter="${LOG_FILTER}" \
    --use-partitioned-tables
fi

SINK_WRITER="$(gcloud logging sinks describe "${SINK_ID}" \
  --project="${GCP_PROJECT}" \
  --format='value(writerIdentity)')"
echo ""
echo "Sink writer identity: ${SINK_WRITER}"
echo "Grant roles/bigquery.dataEditor on dataset ${BQ_DATASET} if not already granted."
echo ""
echo "Query table prefix:"
echo "  ${GCP_PROJECT}.${BQ_DATASET}.run_googleapis_com_stdout_*"
