-- Astro engine latency percentiles from Cloud Logging → BigQuery export.
-- Replace PROJECT_ID with your GCP project (see docs/runbooks/observability.md).
-- Table wildcard: astro_engine_logs.run_googleapis_com_stdout_*

-- Query 1: api_usage latency percentiles per path (last 24h)
SELECT
  jsonPayload.path AS path,
  COUNT(*) AS request_count,
  APPROX_QUANTILES(CAST(jsonPayload.latency_ms AS INT64), 100)[OFFSET(50)] AS p50_ms,
  APPROX_QUANTILES(CAST(jsonPayload.latency_ms AS INT64), 100)[OFFSET(95)] AS p95_ms,
  APPROX_QUANTILES(CAST(jsonPayload.latency_ms AS INT64), 100)[OFFSET(99)] AS p99_ms
FROM
  `PROJECT_ID.astro_engine_logs.run_googleapis_com_stdout_*`
WHERE
  timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), INTERVAL 24 HOUR)
  AND jsonPayload.message = 'api_usage'
  -- AND jsonPayload.path = '/chart/sidereal'
GROUP BY
  path
ORDER BY
  request_count DESC;

-- Query 2: slo_breach counts per path (last 24h)
SELECT
  jsonPayload.path AS path,
  COUNT(*) AS slo_breach_count
FROM
  `PROJECT_ID.astro_engine_logs.run_googleapis_com_stdout_*`
WHERE
  timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), INTERVAL 24 HOUR)
  AND jsonPayload.message = 'slo_breach'
  AND jsonPayload.slo_breach = TRUE
GROUP BY
  path
ORDER BY
  slo_breach_count DESC;
