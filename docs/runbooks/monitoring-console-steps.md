# Monitoring setup — Console and gcloud

Use when [`setup_monitoring.sh`](../../deploy/cloudrun/setup_monitoring.sh) cannot be run from CI, or to verify resources created by the script.

## Multi-region uptime check (`GET /health`)

**Goal:** HTTPS probe every **60s** from three synthetic regions; alert after **2 consecutive** failures.

| Sprint region | Cloud Monitoring probe id | Notes |
| --- | --- | --- |
| asia-south1 (service region) | `asia-southeast1` | Nearest supported probe to Mumbai |
| us-central1 | `usa-iowa` | Maps to `us-central1` |
| europe-west1 | `europe-west1` | EU probe |

### Console

1. **Monitoring → Uptime checks → Create**
2. Target: **URL**, protocol **HTTPS**, path `/health`, host = Cloud Run hostname (no path prefix).
3. Check frequency: **1 minute** (60s).
4. Select regions: **asia-southeast1**, **usa-iowa**, **europe-west1** (minimum 3).
5. Response validation: status class **2xx**.
6. Display name: `astro-api health (multi-region)` (or `${CLOUD_RUN_SERVICE} health (multi-region)`).

### gcloud

```bash
export PROJECT_ID="your-gcp-project"
export SERVICE_HOST="astro-api-xxxxx.asia-south1.run.app"   # no https://

gcloud monitoring uptime create "${SERVICE_NAME:-astro-api} health (multi-region)" \
  --project="${PROJECT_ID}" \
  --resource-type=uptime-url \
  --resource-labels="host=${SERVICE_HOST},project_id=${PROJECT_ID}" \
  --path=/health \
  --protocol=https \
  --request-method=get \
  --status-classes=2xx \
  --period=60s \
  --timeout=10 \
  --regions=asia-southeast1,usa-iowa,europe-west1 \
  --validate-ssl=true
```

### Uptime alert (2 consecutive failures)

1. **Monitoring → Alerting → Create policy**
2. Condition: metric `monitoring.googleapis.com/uptime_check/check_passed`, filter by uptime check id, threshold **> 1** failed checks over **60s** alignment, trigger **count = 2** (or duration covering two failed periods).
3. Notification channel: set `NOTIFICATION_CHANNELS` to your PagerDuty/email channel resource name, e.g. `projects/PROJECT/notificationChannels/PAGERDUTY_CHANNEL_ID`.
4. Display name: `astro-api uptime failure`.

Repo template: [`deploy/cloudrun/alert-policies/uptime_failure.json.tmpl`](../../deploy/cloudrun/alert-policies/uptime_failure.json.tmpl) (rendered by `setup_monitoring.sh`).

## Synthetic chart monitor

Cloud Monitoring URL checks cannot easily assert JSON body fields. Use:

- **Cloud Scheduler** → HTTP target or Cloud Run job running [`scripts/monitoring/synthetic-chart-sidereal.sh`](../../scripts/monitoring/synthetic-chart-sidereal.sh) every 5 minutes; alert on job failure / non-zero exit.
- Or CI/cron on a trusted runner with `ASTRO_API_BASE_URL` + `ASTRO_API_KEY`.

Golden expected lagna: **275.1573701670353°** (±1e-6°), fixture [`tests/golden/synthetic/delhi-1990-chart.json`](../../tests/golden/synthetic/delhi-1990-chart.json).

## One-command setup

```bash
PROJECT_ID="..." \
SERVICE_NAME="astro-api" \
REGION="asia-south1" \
NOTIFICATION_CHANNELS="projects/.../notificationChannels/..." \
bash deploy/cloudrun/setup_monitoring.sh
```

Post-deploy: `bash deploy/cloudrun/post_deploy_verify.sh`
