use reqwest::Client;
use serde_json::{json, Value};

/// Base URL of the deployed service (no trailing slash), for example `https://astro-api-xxxxx.run.app`.
fn deployed_base_url() -> Option<String> {
    std::env::var("ASTRO_API_BASE_URL").ok().map(|value| value.trim_end_matches('/').to_owned())
}

/// API key that is configured in the deployment's `VALID_API_KEYS`. Used for authenticated contract checks.
fn deployed_api_key() -> Option<String> {
    std::env::var("ASTRO_API_KEY").ok().map(|value| value.trim().to_owned()).filter(|value| !value.is_empty())
}

fn require_deployed_base_url() -> Option<String> {
    let Some(base_url) = deployed_base_url() else {
        eprintln!("skipping production contract test because ASTRO_API_BASE_URL is unset");
        return None;
    };
    Some(base_url)
}

fn require_deployed_base_url_and_api_key() -> Option<(String, String)> {
    let Some(base_url) = require_deployed_base_url() else {
        return None;
    };
    let Some(api_key) = deployed_api_key() else {
        eprintln!("skipping authenticated production contract test because ASTRO_API_KEY is unset");
        return None;
    };
    Some((base_url, api_key))
}

fn http_client() -> Client {
    Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("http client must build")
}

async fn post_json(client: &Client, url: &str, payload: Value) -> reqwest::Response {
    client
        .post(url)
        .header("content-type", "application/json")
        .json(&payload)
        .send()
        .await
        .expect("request must succeed")
}

async fn post_json_with_api_key(client: &Client, url: &str, payload: Value, api_key: &str) -> reqwest::Response {
    client
        .post(url)
        .header("content-type", "application/json")
        .header("x-api-key", api_key)
        .json(&payload)
        .send()
        .await
        .expect("request must succeed")
}

#[tokio::test(flavor = "current_thread")]
async fn deployed_health_endpoint_reports_version() {
    let Some(base_url) = require_deployed_base_url() else {
        return;
    };

    let response = http_client()
        .get(format!("{base_url}/health"))
        .send()
        .await
        .expect("health request must succeed");

    assert!(response.status().is_success(), "health must return 2xx");
    assert!(
        response
            .headers()
            .get("x-request-id")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| !value.is_empty()),
        "health response must include x-request-id for request correlation"
    );
    let body: Value = response.json().await.expect("health response must be valid json");
    assert_eq!(body["status"].as_str(), Some("ok"));
    assert!(body["version"].as_str().is_some_and(|version| !version.is_empty()));
}

#[tokio::test(flavor = "current_thread")]
async fn deployed_sidereal_positions_use_de440_contract() {
    let Some((base_url, api_key)) = require_deployed_base_url_and_api_key() else {
        return;
    };

    let response = post_json_with_api_key(
        &http_client(),
        &format!("{base_url}/positions/sidereal"),
        json!({
            "datetime": { "kind": "utc", "utc": "2000-01-01T12:00:00Z" },
            "geo": {
                "latitude_deg": 12.9716,
                "longitude_deg": 77.5946,
                "elevation_m": 920.0
            },
            "ayanamsa": "lahiri",
            "bodies": ["moon", "sun", "mercury"],
            "gravitational_deflection": false
        }),
        &api_key,
    )
    .await;

    assert!(response.status().is_success(), "sidereal positions must return 2xx");
    let body: Value = response.json().await.expect("sidereal positions must be valid json");

    assert_eq!(body["metadata"]["engine_mode"].as_str(), Some("vedic"));
    assert_eq!(body["metadata"]["ayanamsa_used"].as_str(), Some("lahiri"));
    assert_eq!(body["metadata"]["gravitational_deflection"].as_bool(), Some(false));

    let positions = body["data"]["positions"].as_array().expect("positions array must exist");
    assert_eq!(positions.len(), 3);

    let moon =
        positions.iter().find(|position| position["body"] == "moon").expect("moon must exist");
    let sun = positions.iter().find(|position| position["body"] == "sun").expect("sun must exist");
    let mercury = positions
        .iter()
        .find(|position| position["body"] == "mercury")
        .expect("mercury must exist");

    for position in positions {
        assert!(position["sidereal_longitude_deg"].is_number());
        assert!(position["longitude_speed_deg_per_day"].is_number());
        assert!(position["retrograde"].is_boolean());
        assert_eq!(position["computation_meta"]["observer"].as_str(), Some("geocenter"));
        assert_eq!(position["computation_meta"]["topocentric_applied"].as_bool(), Some(false));
        assert_eq!(position["computation_meta"]["gravitational_deflection"].as_bool(), Some(false));
        assert!(position["computation_meta"]["kernel"]
            .as_str()
            .is_some_and(|kernel| kernel.starts_with("de440_")));
        assert_eq!(
            position["computation_meta"]["ayanamsa_algorithm"].as_str(),
            Some("lahiri_swe_zero_epoch_iau1976_v1")
        );
    }

    let moon_speed =
        moon["longitude_speed_deg_per_day"].as_f64().expect("moon speed must be numeric");
    assert!(moon_speed > 10.0);
    assert!(moon_speed < 16.0);

    let sun_speed = sun["longitude_speed_deg_per_day"].as_f64().expect("sun speed must be numeric");
    assert!(sun_speed.abs() > 0.9);
    assert!(sun_speed.abs() < 1.1);

    let mercury_speed =
        mercury["longitude_speed_deg_per_day"].as_f64().expect("mercury speed must be numeric");
    assert!(mercury_speed.abs() > 0.5);
    assert!(mercury_speed.abs() < 2.5);
}

#[tokio::test(flavor = "current_thread")]
async fn deployed_sidereal_chart_compact_projection_matches_mobile_contract() {
    let Some((base_url, api_key)) = require_deployed_base_url_and_api_key() else {
        return;
    };

    let response = post_json_with_api_key(
        &http_client(),
        &format!("{base_url}/chart/sidereal"),
        json!({
            "datetime": { "kind": "utc", "utc": "1990-05-17T04:30:00Z" },
            "geo": {
                "latitude_deg": 28.6139,
                "longitude_deg": 77.2090,
                "elevation_m": 216.0
            },
            "ayanamsa": "lahiri",
            "compact": true,
            "projection": "sidereal_only"
        }),
        &api_key,
    )
    .await;

    assert!(response.status().is_success(), "sidereal chart must return 2xx");
    let body: Value = response.json().await.expect("sidereal chart must be valid json");

    assert_eq!(body["data"]["schema_version"].as_str(), Some("chart_sidereal_v1"));
    assert_eq!(body["data"]["house_system"].as_str(), Some("whole_sign"));
    assert_eq!(body["data"]["lahiri_algorithm"].as_str(), Some("lahiri_swe_zero_epoch_iau1976_v1"));
    assert!(body["data"]["summary"]["placement_table"].is_array());
    assert!(body["data"]["grahas"].as_array().is_some_and(|grahas| !grahas.is_empty()));
    assert!(body["data"]["houses"].is_null(), "compact chart should omit houses");
    assert!(body["data"]["dasha"].is_null(), "compact chart should omit dasha summary");
    assert!(body["data"]["lagna"]["sidereal_longitude_deg"].is_number());
}

fn minimal_sidereal_payload() -> Value {
    json!({
        "datetime": { "kind": "utc", "utc": "2000-01-01T12:00:00Z" },
        "geo": { "latitude_deg": 12.9716, "longitude_deg": 77.5946, "elevation_m": 920.0 },
        "ayanamsa": "lahiri",
        "bodies": ["moon"],
        "gravitational_deflection": false
    })
}

fn minimal_chart_payload() -> Value {
    json!({
        "datetime": { "kind": "utc", "utc": "1990-05-17T04:30:00Z" },
        "geo": { "latitude_deg": 28.6139, "longitude_deg": 77.2090, "elevation_m": 216.0 },
        "ayanamsa": "lahiri",
        "compact": true,
        "projection": "sidereal_only"
    })
}

#[tokio::test(flavor = "current_thread")]
async fn deployed_protected_routes_return_401_without_api_key() {
    let Some(base_url) = require_deployed_base_url() else {
        return;
    };

    let client = http_client();
    let sidereal = post_json(&client, &format!("{base_url}/positions/sidereal"), minimal_sidereal_payload()).await;
    assert_eq!(sidereal.status(), reqwest::StatusCode::UNAUTHORIZED);
    let body: Value = sidereal.json().await.expect("sidereal error body must be json");
    assert_eq!(body["error"].as_str(), Some("missing_api_key"));

    let chart = post_json(&client, &format!("{base_url}/chart/sidereal"), minimal_chart_payload()).await;
    assert_eq!(chart.status(), reqwest::StatusCode::UNAUTHORIZED);
    let body: Value = chart.json().await.expect("chart error body must be json");
    assert_eq!(body["error"].as_str(), Some("missing_api_key"));
}

#[tokio::test(flavor = "current_thread")]
async fn deployed_protected_routes_return_401_for_invalid_api_key() {
    let Some(base_url) = require_deployed_base_url() else {
        return;
    };

    let client = http_client();
    let sidereal = client
        .post(format!("{base_url}/positions/sidereal"))
        .header("content-type", "application/json")
        .header("x-api-key", "astro-contract-invalid-key")
        .json(&minimal_sidereal_payload())
        .send()
        .await
        .expect("request must succeed");
    assert_eq!(sidereal.status(), reqwest::StatusCode::UNAUTHORIZED);
    let body: Value = sidereal.json().await.expect("sidereal error body must be json");
    assert_eq!(body["error"].as_str(), Some("invalid_api_key"));
}

/// When `ASTRO_CONTRACT_ASSERT_RATE_LIMIT=1`, expects the deployment to use `RATE_LIMIT_RPM=2` (or lower)
/// so three authenticated requests to `/positions/sidereal` within a short window yield HTTP 429 on the third.
#[tokio::test(flavor = "current_thread")]
async fn deployed_rate_limit_may_return_429_when_configured_for_contract() {
    let Some((base_url, api_key)) = require_deployed_base_url_and_api_key() else {
        return;
    };

    let assert_rate = std::env::var("ASTRO_CONTRACT_ASSERT_RATE_LIMIT")
        .ok()
        .is_some_and(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "YES"));

    if !assert_rate {
        eprintln!("skipping rate-limit contract (set ASTRO_CONTRACT_ASSERT_RATE_LIMIT=1 and configure service RATE_LIMIT_RPM=2 for this check)");
        return;
    }

    let client = http_client();
    for _ in 0..2 {
        let response = post_json_with_api_key(
            &client,
            &format!("{base_url}/positions/sidereal"),
            minimal_sidereal_payload(),
            &api_key,
        )
        .await;
        assert!(
            response.status().is_success(),
            "first two authenticated requests must succeed when rate limit is enabled"
        );
    }

    let limited = post_json_with_api_key(
        &client,
        &format!("{base_url}/positions/sidereal"),
        minimal_sidereal_payload(),
        &api_key,
    )
    .await;

    assert_eq!(limited.status(), reqwest::StatusCode::TOO_MANY_REQUESTS);
    let retry_after = limited
        .headers()
        .get("retry-after")
        .and_then(|value| value.to_str().ok())
        .expect("429 must include retry-after");
    assert!(retry_after.parse::<u64>().is_ok());
    let body: Value = limited.json().await.expect("429 body must be json");
    assert_eq!(body["error"].as_str(), Some("rate_limit_exceeded"));
}
