use astro_api::{app_router_with_api_keys, de440_state_from_env};
use axum::{
    body::{to_bytes, Body},
    http::{Method, Request, StatusCode},
};
use serde::Deserialize;
use serde_json::Value;
use tower::util::ServiceExt;

#[path = "../../../tests/support/de440_kernel.rs"]
mod de440_kernel;

const TEST_API_KEY: &str = "test-api-key";

#[derive(Debug, Deserialize)]
struct SyntheticGoldenFixture {
    request: Value,
    expected_lagna_sidereal_longitude_deg: f64,
    expected_lagna_rashi: String,
    tolerance_deg: f64,
}

fn load_fixture() -> SyntheticGoldenFixture {
    let raw = include_str!("../../../tests/golden/synthetic/delhi-1990-chart.json");
    serde_json::from_str(raw).expect("synthetic golden fixture must parse")
}

fn authenticated_request(method: Method, uri: &str) -> axum::http::request::Builder {
    Request::builder().method(method).uri(uri).header("x-api-key", TEST_API_KEY)
}

#[tokio::test(flavor = "current_thread")]
async fn delhi_1990_chart_lagna_matches_de440_golden() {
    if de440_kernel::require_de440_kernel().is_none() {
        return;
    }

    let fixture = load_fixture();
    let app = app_router_with_api_keys(
        de440_state_from_env().expect("DE440 state must load"),
        [TEST_API_KEY],
    );

    let response = app
        .oneshot(
            authenticated_request(Method::POST, "/chart/sidereal")
                .header("content-type", "application/json")
                .body(Body::from(fixture.request.to_string()))
                .expect("request must build"),
        )
        .await
        .expect("response must succeed");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.expect("body must be readable");
    let json: Value = serde_json::from_slice(&body).expect("body must be valid json");

    let lagna_deg = json["data"]["lagna"]["sidereal_longitude_deg"]
        .as_f64()
        .expect("lagna sidereal longitude must be numeric");
    let lagna_rashi = json["data"]["lagna"]["rashi"].as_str().expect("lagna rashi must be present");

    assert_eq!(lagna_rashi, fixture.expected_lagna_rashi);
    let delta = (lagna_deg - fixture.expected_lagna_sidereal_longitude_deg).abs();
    assert!(
        delta <= fixture.tolerance_deg,
        "lagna longitude drift: got {lagna_deg}, expected {}, delta {delta} > tolerance {}",
        fixture.expected_lagna_sidereal_longitude_deg,
        fixture.tolerance_deg
    );
}
