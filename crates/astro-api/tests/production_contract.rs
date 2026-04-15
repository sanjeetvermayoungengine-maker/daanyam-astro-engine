use reqwest::Client;
use serde_json::{json, Value};

fn deployed_base_url() -> Option<String> {
    std::env::var("ASTRO_API_BASE_URL")
        .ok()
        .map(|value| value.trim_end_matches('/').to_owned())
}

fn require_deployed_base_url() -> Option<String> {
    let Some(base_url) = deployed_base_url() else {
        eprintln!("skipping production contract test because ASTRO_API_BASE_URL is unset");
        return None;
    };
    Some(base_url)
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
    let body: Value = response.json().await.expect("health response must be valid json");
    assert_eq!(body["status"].as_str(), Some("ok"));
    assert!(body["version"].as_str().is_some_and(|version| !version.is_empty()));
}

#[tokio::test(flavor = "current_thread")]
async fn deployed_sidereal_positions_use_de440_contract() {
    let Some(base_url) = require_deployed_base_url() else {
        return;
    };

    let response = post_json(
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
    let Some(base_url) = require_deployed_base_url() else {
        return;
    };

    let response = post_json(
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
    )
    .await;

    assert!(response.status().is_success(), "sidereal chart must return 2xx");
    let body: Value = response.json().await.expect("sidereal chart must be valid json");

    assert_eq!(body["data"]["schema_version"].as_str(), Some("chart_sidereal_v1"));
    assert_eq!(body["data"]["house_system"].as_str(), Some("whole_sign"));
    assert_eq!(
        body["data"]["lahiri_algorithm"].as_str(),
        Some("lahiri_swe_zero_epoch_iau1976_v1")
    );
    assert!(body["data"]["summary"]["placement_table"].is_array());
    assert!(body["data"]["grahas"].as_array().is_some_and(|grahas| !grahas.is_empty()));
    assert!(body["data"]["houses"].is_null(), "compact chart should omit houses");
    assert!(body["data"]["dasha"].is_null(), "compact chart should omit dasha summary");
    assert!(body["data"]["lagna"]["sidereal_longitude_deg"].is_number());
}
