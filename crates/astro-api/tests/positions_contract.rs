use astro_api::{
    app_router_with_api_keys, de440_state_from_env, demo_state, CHART_SIDEREAL_SCHEMA_VERSION,
};
use axum::{
    body::{to_bytes, Body},
    http::{Method, Request, StatusCode},
};
use serde_json::Value;
use tower::util::ServiceExt;

#[path = "../../../tests/support/de440_kernel.rs"]
mod de440_kernel;

const TEST_API_KEY: &str = "test-api-key";

fn authenticated_request(method: Method, uri: &str) -> axum::http::request::Builder {
    Request::builder().method(method).uri(uri).header("x-api-key", TEST_API_KEY)
}

#[tokio::test(flavor = "current_thread")]
async fn positions_contract_includes_per_body_computation_metadata() {
    if de440_kernel::require_de440_kernel().is_none() {
        return;
    }
    let app = app_router_with_api_keys(
        de440_state_from_env().expect("DE440 state must load"),
        [TEST_API_KEY],
    );

    let response = app
        .oneshot(
            authenticated_request(Method::POST, "/positions")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"julian_day":2451545.0,"bodies":["moon","sun","mercury"]}"#))
                .expect("request must build"),
        )
        .await
        .expect("response must succeed");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.expect("body must be readable");
    let json: Value = serde_json::from_slice(&body).expect("body must be valid json");

    let positions = json["data"]["positions"].as_array().expect("positions array must exist");
    assert_eq!(positions.len(), 3);

    for position in positions {
        assert!(position["position"]["longitude_deg"].is_number());
        assert!(position["position"]["latitude_deg"].is_number());
        assert_eq!(
            position["computation_meta"]["crate_version"].as_str(),
            Some(env!("CARGO_PKG_VERSION"))
        );
        assert!(position["computation_meta"]["kernel"]
            .as_str()
            .is_some_and(|kernel| kernel.starts_with("de440_")));
        assert_eq!(
            position["computation_meta"]["frame"].as_str(),
            Some("apparent_ecliptic_of_date")
        );
        assert_eq!(position["computation_meta"]["light_time"].as_bool(), Some(true));
        assert_eq!(position["computation_meta"]["stellar_aberration"].as_bool(), Some(true));
        assert_eq!(position["computation_meta"]["gravitational_deflection"].as_bool(), Some(true));
        assert_eq!(position["computation_meta"]["observer"].as_str(), Some("geocenter"));
        assert_eq!(position["computation_meta"]["topocentric_applied"].as_bool(), Some(false));
        assert!(position["computation_meta"]["kernel_notes"].is_string());
        assert!(position["computation_meta"]["motion_model"].is_null());
    }
}

#[tokio::test(flavor = "current_thread")]
async fn positions_compact_mode_omits_heavy_fields() {
    let app = app_router_with_api_keys(demo_state(), [TEST_API_KEY]);

    let response = app
        .oneshot(
            authenticated_request(Method::POST, "/positions")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"julian_day":2451545.0,"bodies":["moon"],"compact":true}"#))
                .expect("request must build"),
        )
        .await
        .expect("response must succeed");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.expect("body must be readable");
    let json: Value = serde_json::from_slice(&body).expect("body must be valid json");

    let position = &json["data"]["positions"][0];
    assert!(position["position"]["longitude_deg"].is_number());
    assert!(position["position"]["distance_au"].is_null());
    assert!(position["computation_meta"].is_null());
}

#[tokio::test(flavor = "current_thread")]
async fn sidereal_contract_includes_per_body_metadata() {
    // Motion sanity here is cross-checked vs JPL Horizons for the same observer/frame
    // settings as the engine: geocenter observer, apparent geocentric ecliptic quantities.
    // Horizons settings reference: https://ssd.jpl.nasa.gov/horizons/manual.html
    if de440_kernel::require_de440_kernel().is_none() {
        return;
    }
    let app = app_router_with_api_keys(
        de440_state_from_env().expect("DE440 state must load"),
        [TEST_API_KEY],
    );

    let response = app
        .oneshot(
            authenticated_request(Method::POST, "/positions/sidereal")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"datetime":{"kind":"utc","utc":"2000-01-01T12:00:00Z"},"geo":{"latitude_deg":12.9716,"longitude_deg":77.5946,"elevation_m":920.0},"ayanamsa":"lahiri","bodies":["moon","sun","mercury"],"gravitational_deflection":false}"#,
                ))
                .expect("request must build"),
        )
        .await
        .expect("response must succeed");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.expect("body must be readable");
    let json: Value = serde_json::from_slice(&body).expect("body must be valid json");

    let positions = json["data"]["positions"].as_array().expect("positions array must exist");
    assert_eq!(positions.len(), 3);

    let moon =
        positions.iter().find(|position| position["body"] == "moon").expect("moon must exist");
    for position in positions {
        assert!(position["tropical_longitude_deg"].is_number());
        assert!(position["sidereal_longitude_deg"].is_number());
        assert!(position["longitude_speed_deg_per_day"].is_number());
        assert!(position["retrograde"].is_boolean());
        assert_eq!(
            position["computation_meta"]["ayanamsa_algorithm"].as_str(),
            Some(astro_vedic::LAHIRI_ALGO_ID)
        );
        assert_eq!(position["computation_meta"]["observer"].as_str(), Some("geocenter"));
        assert_eq!(position["computation_meta"]["topocentric_applied"].as_bool(), Some(false));
        assert!(position["computation_meta"]["kernel_notes"].is_string());
        assert!(position["computation_meta"]["motion_model"].is_null());
        assert_eq!(position["computation_meta"]["gravitational_deflection"].as_bool(), Some(false));
    }

    assert!(positions[0]["moon_division"].is_object());
    assert!(positions[1]["moon_division"].is_null());

    let moon_speed =
        moon["longitude_speed_deg_per_day"].as_f64().expect("moon speed must be numeric");
    assert!(moon_speed > 10.0, "moon speed should be clearly non-zero in DE440 path");
    assert!(moon_speed < 16.0, "moon speed should stay within sane lunar bounds");
    let sun = positions.iter().find(|position| position["body"] == "sun").expect("sun must exist");
    let mercury = positions
        .iter()
        .find(|position| position["body"] == "mercury")
        .expect("mercury must exist");
    let sun_speed = sun["longitude_speed_deg_per_day"].as_f64().expect("sun speed must be numeric");
    assert!(sun_speed.abs() > 0.9, "sun speed should stay near one degree per day");
    assert!(sun_speed.abs() < 1.1, "sun speed should stay near one degree per day");
    let mercury_speed =
        mercury["longitude_speed_deg_per_day"].as_f64().expect("mercury speed must be numeric");
    assert!(
        mercury_speed.abs() > 0.5,
        "mercury speed should be meaningfully non-zero outside exact station checks"
    );
    assert!(mercury_speed.abs() < 2.5, "mercury speed should remain within a tolerant window");
}

#[tokio::test(flavor = "current_thread")]
async fn mercury_retrograde_regression_matches_cited_2024_window() {
    // Cross-checked vs JPL Horizons for the same observer/frame settings as the engine:
    // target=Mercury, center=500@399 (geocenter), apparent ecliptic-of-date observer quantities.
    // UTC instants asserted here:
    // - 2024-04-10T12:00:00Z => expected retrograde=true
    // - 2024-04-30T12:00:00Z => expected retrograde=false
    // Horizons settings reference: https://ssd.jpl.nasa.gov/horizons/manual.html
    if de440_kernel::require_de440_kernel().is_none() {
        return;
    }
    let app = app_router_with_api_keys(
        de440_state_from_env().expect("DE440 state must load"),
        [TEST_API_KEY],
    );

    let retrograde_response = app
        .clone()
        .oneshot(
            authenticated_request(Method::POST, "/positions/sidereal")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"datetime":{"kind":"utc","utc":"2024-04-10T12:00:00Z"},"geo":{"latitude_deg":0.0,"longitude_deg":0.0,"elevation_m":0.0},"ayanamsa":"lahiri","bodies":["mercury"]}"#,
                ))
                .expect("request must build"),
        )
        .await
        .expect("retrograde response must succeed");

    let direct_response = app
        .oneshot(
            authenticated_request(Method::POST, "/positions/sidereal")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"datetime":{"kind":"utc","utc":"2024-04-30T12:00:00Z"},"geo":{"latitude_deg":0.0,"longitude_deg":0.0,"elevation_m":0.0},"ayanamsa":"lahiri","bodies":["mercury"]}"#,
                ))
                .expect("request must build"),
        )
        .await
        .expect("direct response must succeed");

    let retrograde_body = to_bytes(retrograde_response.into_body(), usize::MAX)
        .await
        .expect("retrograde body must be readable");
    let direct_body = to_bytes(direct_response.into_body(), usize::MAX)
        .await
        .expect("direct body must be readable");
    let retrograde_json: Value =
        serde_json::from_slice(&retrograde_body).expect("retrograde json must be valid");
    let direct_json: Value =
        serde_json::from_slice(&direct_body).expect("direct json must be valid");

    let retrograde_position = &retrograde_json["data"]["positions"][0];
    let direct_position = &direct_json["data"]["positions"][0];
    let retrograde_speed = retrograde_position["longitude_speed_deg_per_day"]
        .as_f64()
        .expect("retrograde speed must be numeric");
    let direct_speed = direct_position["longitude_speed_deg_per_day"]
        .as_f64()
        .expect("direct speed must be numeric");

    assert_eq!(retrograde_position["retrograde"].as_bool(), Some(true));
    assert!(retrograde_speed < 0.0, "retrograde speed must be negative");
    assert_eq!(direct_position["retrograde"].as_bool(), Some(false));
    assert!(direct_speed > 0.0, "direct speed must be positive");
}

#[tokio::test(flavor = "current_thread")]
async fn venus_retrograde_regression_matches_horizons_checked_dates() {
    // Cross-checked vs JPL Horizons for the same observer/frame settings as the engine:
    // target=Venus, center=500@399 (geocenter), apparent ecliptic-of-date observer quantities.
    // UTC instants asserted here:
    // - 2023-08-10T12:00:00Z => expected retrograde=true
    // - 2023-09-15T12:00:00Z => expected retrograde=false
    // Horizons settings reference: https://ssd.jpl.nasa.gov/horizons/manual.html
    if de440_kernel::require_de440_kernel().is_none() {
        return;
    }
    let app = app_router_with_api_keys(
        de440_state_from_env().expect("DE440 state must load"),
        [TEST_API_KEY],
    );

    let retrograde_response = app
        .clone()
        .oneshot(
            authenticated_request(Method::POST, "/positions/sidereal")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"datetime":{"kind":"utc","utc":"2023-08-10T12:00:00Z"},"geo":{"latitude_deg":0.0,"longitude_deg":0.0,"elevation_m":0.0},"ayanamsa":"lahiri","bodies":["venus"]}"#,
                ))
                .expect("request must build"),
        )
        .await
        .expect("retrograde response must succeed");

    let direct_response = app
        .oneshot(
            authenticated_request(Method::POST, "/positions/sidereal")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"datetime":{"kind":"utc","utc":"2023-09-15T12:00:00Z"},"geo":{"latitude_deg":0.0,"longitude_deg":0.0,"elevation_m":0.0},"ayanamsa":"lahiri","bodies":["venus"]}"#,
                ))
                .expect("request must build"),
        )
        .await
        .expect("direct response must succeed");

    let retrograde_body = to_bytes(retrograde_response.into_body(), usize::MAX)
        .await
        .expect("retrograde body must be readable");
    let direct_body = to_bytes(direct_response.into_body(), usize::MAX)
        .await
        .expect("direct body must be readable");
    let retrograde_json: Value =
        serde_json::from_slice(&retrograde_body).expect("retrograde json must be valid");
    let direct_json: Value =
        serde_json::from_slice(&direct_body).expect("direct json must be valid");

    let retrograde_position = &retrograde_json["data"]["positions"][0];
    let direct_position = &direct_json["data"]["positions"][0];
    let retrograde_speed = retrograde_position["longitude_speed_deg_per_day"]
        .as_f64()
        .expect("retrograde speed must be numeric");
    let direct_speed = direct_position["longitude_speed_deg_per_day"]
        .as_f64()
        .expect("direct speed must be numeric");

    assert_eq!(retrograde_position["retrograde"].as_bool(), Some(true));
    assert!(retrograde_speed < 0.0, "retrograde speed must be negative");
    assert_eq!(direct_position["retrograde"].as_bool(), Some(false));
    assert!(direct_speed > 0.0, "direct speed must be positive");
}

#[tokio::test(flavor = "current_thread")]
async fn mars_retrograde_regression_matches_horizons_checked_dates() {
    // Cross-checked vs JPL Horizons for the same observer/frame settings as the engine:
    // target=Mars, center=500@399 (geocenter), apparent ecliptic-of-date observer quantities.
    // UTC instants asserted here:
    // - 2022-11-15T12:00:00Z => expected retrograde=true
    // - 2023-01-20T12:00:00Z => expected retrograde=false
    // Horizons settings reference: https://ssd.jpl.nasa.gov/horizons/manual.html
    if de440_kernel::require_de440_kernel().is_none() {
        return;
    }
    let app = app_router_with_api_keys(
        de440_state_from_env().expect("DE440 state must load"),
        [TEST_API_KEY],
    );

    let retrograde_response = app
        .clone()
        .oneshot(
            authenticated_request(Method::POST, "/positions/sidereal")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"datetime":{"kind":"utc","utc":"2022-11-15T12:00:00Z"},"geo":{"latitude_deg":0.0,"longitude_deg":0.0,"elevation_m":0.0},"ayanamsa":"lahiri","bodies":["mars"]}"#,
                ))
                .expect("request must build"),
        )
        .await
        .expect("retrograde response must succeed");

    let direct_response = app
        .oneshot(
            authenticated_request(Method::POST, "/positions/sidereal")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"datetime":{"kind":"utc","utc":"2023-01-20T12:00:00Z"},"geo":{"latitude_deg":0.0,"longitude_deg":0.0,"elevation_m":0.0},"ayanamsa":"lahiri","bodies":["mars"]}"#,
                ))
                .expect("request must build"),
        )
        .await
        .expect("direct response must succeed");

    let retrograde_body = to_bytes(retrograde_response.into_body(), usize::MAX)
        .await
        .expect("retrograde body must be readable");
    let direct_body = to_bytes(direct_response.into_body(), usize::MAX)
        .await
        .expect("direct body must be readable");
    let retrograde_json: Value =
        serde_json::from_slice(&retrograde_body).expect("retrograde json must be valid");
    let direct_json: Value =
        serde_json::from_slice(&direct_body).expect("direct json must be valid");

    let retrograde_position = &retrograde_json["data"]["positions"][0];
    let direct_position = &direct_json["data"]["positions"][0];
    let retrograde_speed = retrograde_position["longitude_speed_deg_per_day"]
        .as_f64()
        .expect("retrograde speed must be numeric");
    let direct_speed = direct_position["longitude_speed_deg_per_day"]
        .as_f64()
        .expect("direct speed must be numeric");

    assert_eq!(retrograde_position["retrograde"].as_bool(), Some(true));
    assert!(retrograde_speed < 0.0, "retrograde speed must be negative");
    assert_eq!(direct_position["retrograde"].as_bool(), Some(false));
    assert!(direct_speed > 0.0, "direct speed must be positive");
}

#[tokio::test(flavor = "current_thread")]
async fn jupiter_retrograde_regression_matches_horizons_checked_dates() {
    // Cross-checked vs JPL Horizons for the same observer/frame settings as the engine:
    // target=Jupiter barycenter (599), center=500@399 (geocenter), apparent ecliptic-of-date observer quantities.
    // UTC instants asserted here:
    // - 2023-10-15T12:00:00Z => expected retrograde=true
    // - 2024-02-15T12:00:00Z => expected retrograde=false
    // Horizons settings reference: https://ssd.jpl.nasa.gov/horizons/manual.html
    if de440_kernel::require_de440_kernel().is_none() {
        return;
    }
    let app = app_router_with_api_keys(
        de440_state_from_env().expect("DE440 state must load"),
        [TEST_API_KEY],
    );

    let retrograde_response = app
        .clone()
        .oneshot(
            authenticated_request(Method::POST, "/positions/sidereal")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"datetime":{"kind":"utc","utc":"2023-10-15T12:00:00Z"},"geo":{"latitude_deg":0.0,"longitude_deg":0.0,"elevation_m":0.0},"ayanamsa":"lahiri","bodies":["jupiter"]}"#,
                ))
                .expect("request must build"),
        )
        .await
        .expect("retrograde response must succeed");

    let direct_response = app
        .oneshot(
            authenticated_request(Method::POST, "/positions/sidereal")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"datetime":{"kind":"utc","utc":"2024-02-15T12:00:00Z"},"geo":{"latitude_deg":0.0,"longitude_deg":0.0,"elevation_m":0.0},"ayanamsa":"lahiri","bodies":["jupiter"]}"#,
                ))
                .expect("request must build"),
        )
        .await
        .expect("direct response must succeed");

    let retrograde_body = to_bytes(retrograde_response.into_body(), usize::MAX)
        .await
        .expect("retrograde body must be readable");
    let direct_body = to_bytes(direct_response.into_body(), usize::MAX)
        .await
        .expect("direct body must be readable");
    let retrograde_json: Value =
        serde_json::from_slice(&retrograde_body).expect("retrograde json must be valid");
    let direct_json: Value =
        serde_json::from_slice(&direct_body).expect("direct json must be valid");

    let retrograde_position = &retrograde_json["data"]["positions"][0];
    let direct_position = &direct_json["data"]["positions"][0];
    let retrograde_speed = retrograde_position["longitude_speed_deg_per_day"]
        .as_f64()
        .expect("retrograde speed must be numeric");
    let direct_speed = direct_position["longitude_speed_deg_per_day"]
        .as_f64()
        .expect("direct speed must be numeric");

    assert_eq!(retrograde_position["retrograde"].as_bool(), Some(true));
    assert!(retrograde_speed < 0.0, "retrograde speed must be negative");
    assert!(retrograde_speed.abs() > 0.005, "jupiter retrograde speed should be non-trivial");
    assert!(
        retrograde_speed.abs() < 0.3,
        "jupiter speed should remain in a sane outer-planet range"
    );
    assert_eq!(direct_position["retrograde"].as_bool(), Some(false));
    assert!(direct_speed > 0.0, "direct speed must be positive");
    assert!(direct_speed.abs() > 0.005, "jupiter direct speed should be non-trivial");
    assert!(direct_speed.abs() < 0.3, "jupiter speed should remain in a sane outer-planet range");
}

#[tokio::test(flavor = "current_thread")]
async fn saturn_retrograde_regression_matches_horizons_checked_dates() {
    // Cross-checked vs JPL Horizons for the same observer/frame settings as the engine:
    // target=Saturn barycenter (699), center=500@399 (geocenter), apparent ecliptic-of-date observer quantities.
    // UTC instants asserted here:
    // - 2023-08-15T12:00:00Z => expected retrograde=true
    // - 2024-01-15T12:00:00Z => expected retrograde=false
    // Horizons settings reference: https://ssd.jpl.nasa.gov/horizons/manual.html
    if de440_kernel::require_de440_kernel().is_none() {
        return;
    }
    let app = app_router_with_api_keys(
        de440_state_from_env().expect("DE440 state must load"),
        [TEST_API_KEY],
    );

    let retrograde_response = app
        .clone()
        .oneshot(
            authenticated_request(Method::POST, "/positions/sidereal")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"datetime":{"kind":"utc","utc":"2023-08-15T12:00:00Z"},"geo":{"latitude_deg":0.0,"longitude_deg":0.0,"elevation_m":0.0},"ayanamsa":"lahiri","bodies":["saturn"]}"#,
                ))
                .expect("request must build"),
        )
        .await
        .expect("retrograde response must succeed");

    let direct_response = app
        .oneshot(
            authenticated_request(Method::POST, "/positions/sidereal")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"datetime":{"kind":"utc","utc":"2024-01-15T12:00:00Z"},"geo":{"latitude_deg":0.0,"longitude_deg":0.0,"elevation_m":0.0},"ayanamsa":"lahiri","bodies":["saturn"]}"#,
                ))
                .expect("request must build"),
        )
        .await
        .expect("direct response must succeed");

    let retrograde_body = to_bytes(retrograde_response.into_body(), usize::MAX)
        .await
        .expect("retrograde body must be readable");
    let direct_body = to_bytes(direct_response.into_body(), usize::MAX)
        .await
        .expect("direct body must be readable");
    let retrograde_json: Value =
        serde_json::from_slice(&retrograde_body).expect("retrograde json must be valid");
    let direct_json: Value =
        serde_json::from_slice(&direct_body).expect("direct json must be valid");

    let retrograde_position = &retrograde_json["data"]["positions"][0];
    let direct_position = &direct_json["data"]["positions"][0];
    let retrograde_speed = retrograde_position["longitude_speed_deg_per_day"]
        .as_f64()
        .expect("retrograde speed must be numeric");
    let direct_speed = direct_position["longitude_speed_deg_per_day"]
        .as_f64()
        .expect("direct speed must be numeric");

    assert_eq!(retrograde_position["retrograde"].as_bool(), Some(true));
    assert!(retrograde_speed < 0.0, "retrograde speed must be negative");
    assert!(retrograde_speed.abs() > 0.002, "saturn retrograde speed should be non-trivial");
    assert!(
        retrograde_speed.abs() < 0.2,
        "saturn speed should remain in a sane outer-planet range"
    );
    assert_eq!(direct_position["retrograde"].as_bool(), Some(false));
    assert!(direct_speed > 0.0, "direct speed must be positive");
    assert!(direct_speed.abs() > 0.002, "saturn direct speed should be non-trivial");
    assert!(direct_speed.abs() < 0.2, "saturn speed should remain in a sane outer-planet range");
}

#[tokio::test(flavor = "current_thread")]
async fn sidereal_positions_compact_mode_omits_heavy_fields() {
    let app = app_router_with_api_keys(demo_state(), [TEST_API_KEY]);

    let response = app
        .oneshot(
            authenticated_request(Method::POST, "/positions/sidereal")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"datetime":{"kind":"utc","utc":"2000-01-01T12:00:00Z"},"geo":{"latitude_deg":12.9716,"longitude_deg":77.5946,"elevation_m":920.0},"ayanamsa":"lahiri","bodies":["moon"],"compact":true}"#,
                ))
                .expect("request must build"),
        )
        .await
        .expect("response must succeed");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.expect("body must be readable");
    let json: Value = serde_json::from_slice(&body).expect("body must be valid json");

    let position = &json["data"]["positions"][0];
    assert!(position["sidereal_longitude_deg"].is_number());
    assert!(position["longitude_speed_deg_per_day"].is_number());
    assert!(position["distance_au"].is_null());
    assert!(position["moon_division"].is_null());
    assert!(position["computation_meta"].is_null());
}

#[tokio::test(flavor = "current_thread")]
async fn sidereal_positions_sidereal_only_projection_omits_tropical_fields() {
    let app = app_router_with_api_keys(demo_state(), [TEST_API_KEY]);

    let response = app
        .oneshot(
            authenticated_request(Method::POST, "/positions/sidereal")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"datetime":{"kind":"utc","utc":"2000-01-01T12:00:00Z"},"geo":{"latitude_deg":12.9716,"longitude_deg":77.5946,"elevation_m":920.0},"ayanamsa":"lahiri","bodies":["moon"],"compact":true,"projection":"sidereal_only"}"#,
                ))
                .expect("request must build"),
        )
        .await
        .expect("response must succeed");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.expect("body must be readable");
    let json: Value = serde_json::from_slice(&body).expect("body must be valid json");

    let position = &json["data"]["positions"][0];
    assert!(position["sidereal_longitude_deg"].is_number());
    assert!(position["tropical_longitude_deg"].is_null());
    assert!(position["tropical_latitude_deg"].is_null());
    assert!(position["distance_au"].is_null());
    assert!(position["moon_division"].is_null());
    assert!(position["computation_meta"].is_null());
}

#[tokio::test(flavor = "current_thread")]
async fn sidereal_chart_contract_includes_chart_metadata() {
    let app = app_router_with_api_keys(demo_state(), [TEST_API_KEY]);

    let response = app
        .oneshot(
            authenticated_request(Method::POST, "/chart/sidereal")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"datetime":{"kind":"utc","utc":"2000-01-01T12:00:00Z"},"geo":{"latitude_deg":12.9716,"longitude_deg":77.5946,"elevation_m":920.0},"ayanamsa":"lahiri","gravitational_deflection":false}"#,
                ))
                .expect("request must build"),
        )
        .await
        .expect("response must succeed");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.expect("body must be readable");
    let json: Value = serde_json::from_slice(&body).expect("body must be valid json");

    let grahas = json["data"]["grahas"].as_array().expect("grahas array must exist");
    assert_eq!(grahas.len(), 9);
    assert_eq!(
        json["data"]["schema_version"].as_str(),
        Some(astro_api::CHART_SIDEREAL_SCHEMA_VERSION)
    );
    assert_eq!(
        json["data"]["extensions"],
        serde_json::json!({
            "yogas": [
                {
                    "houses_involved": [5, 2],
                    "key": "gajakesari",
                    "name": "Gajakesari Yoga",
                    "planets_involved": ["Moon", "Jupiter"],
                    "strength": 1.0,
                    "voice_line": "Moon and Jupiter sit in mutual kendra — the chart asks you to speak from a steady inner ground; the world tends to listen."
                },
                {
                    "houses_involved": [12, 12],
                    "key": "vipreet_raja",
                    "name": "Vipreet Raja Yoga",
                    "planets_involved": ["Saturn"],
                    "strength": 0.8,
                    "voice_line": "A dusthana lord sits in adversity and turns it into quiet advantage — gains through reversal."
                },
                {
                    "houses_involved": [5, 6],
                    "key": "sunapha",
                    "name": "Sunapha Yoga",
                    "planets_involved": ["Moon"],
                    "strength": 0.85,
                    "voice_line": "Planets flank Moon from the 2nd — self-made prosperity is emphasised when you act on your own initiative."
                }
            ]
        })
    );
    assert!(json["data"]["lagna"]["sidereal_longitude_deg"].is_number());
    assert!(json["data"]["lagna"]["rashi"].is_string());
    let houses = json["data"]["houses"].as_array().expect("houses array must exist");
    assert_eq!(houses.len(), 12);
    assert_eq!(json["data"]["house_system"].as_str(), Some("whole_sign"));
    assert_eq!(houses[0]["house"].as_u64(), Some(1));
    assert_eq!(houses[0]["rashi"], json["data"]["lagna"]["rashi"]);
    assert!(houses[0]["cusp_sidereal_longitude_deg"].is_number());
    for graha in grahas {
        assert!(graha["sidereal_rashi"].is_string());
        assert!(graha["whole_sign_house"].is_u64());
        assert!(graha["longitude_speed_deg_per_day"].is_number());
        assert!(graha["retrograde"].is_boolean());
    }
    let sun = grahas.iter().find(|graha| graha["body"] == "sun").expect("sun must exist");
    let moon = grahas.iter().find(|graha| graha["body"] == "moon").expect("moon must exist");
    assert_eq!(sun["sidereal_rashi"].as_str(), Some("dhanu"));
    assert_eq!(sun["d3_rashi"].as_str(), Some("mesha"));
    assert_eq!(sun["d9_rashi"].as_str(), Some("simha"));
    assert_eq!(sun["whole_sign_house"].as_u64(), Some(10));
    assert_eq!(sun["house_context"]["whole_sign_house"].as_u64(), Some(10));
    assert_eq!(sun["house_context"]["house_lord"].as_str(), Some("jupiter"));
    assert_eq!(sun["longitude_speed_deg_per_day"].as_f64(), Some(0.0));
    assert_eq!(sun["retrograde"].as_bool(), Some(false));
    assert_eq!(moon["sidereal_rashi"].as_str(), Some("karka"));
    assert_eq!(moon["d3_rashi"].as_str(), Some("karka"));
    assert_eq!(moon["d9_rashi"].as_str(), Some("kanya"));
    assert_eq!(moon["whole_sign_house"].as_u64(), Some(5));
    assert_eq!(moon["house_context"]["whole_sign_house"].as_u64(), Some(5));
    assert_eq!(moon["house_context"]["house_lord"].as_str(), Some("moon"));
    assert_eq!(moon["longitude_speed_deg_per_day"].as_f64(), Some(0.0));
    assert_eq!(moon["retrograde"].as_bool(), Some(false));
    assert_eq!(json["data"]["summary"]["moon_rashi"].as_str(), Some("karka"));
    assert_eq!(json["data"]["summary"]["lagna_rashi"].as_str(), Some("meena"));
    assert_eq!(json["data"]["summary"]["lagna_lord"].as_str(), Some("jupiter"));
    assert_eq!(json["data"]["summary"]["grahas_by_rashi"]["dhanu"], serde_json::json!(["sun"]));
    assert_eq!(json["data"]["summary"]["grahas_by_rashi"]["karka"], serde_json::json!(["moon"]));
    assert_eq!(json["data"]["summary"]["grahas_by_rashi"]["simha"], serde_json::json!(["mercury"]));
    assert_eq!(json["data"]["summary"]["grahas_by_rashi"]["meena"], serde_json::json!(["rahu"]));
    let dispositors =
        json["data"]["summary"]["dispositors"].as_array().expect("dispositors array must exist");
    let sun_dispositor =
        dispositors.iter().find(|entry| entry["body"] == "sun").expect("sun dispositor must exist");
    let moon_dispositor = dispositors
        .iter()
        .find(|entry| entry["body"] == "moon")
        .expect("moon dispositor must exist");
    assert_eq!(sun_dispositor["occupied_rashi"].as_str(), Some("dhanu"));
    assert_eq!(sun_dispositor["dispositor"].as_str(), Some("jupiter"));
    assert_eq!(moon_dispositor["occupied_rashi"].as_str(), Some("karka"));
    assert_eq!(moon_dispositor["dispositor"].as_str(), Some("moon"));
    let placement_table = json["data"]["summary"]["placement_table"]
        .as_array()
        .expect("placement_table array must exist");
    let sun_placement = placement_table
        .iter()
        .find(|entry| entry["body"] == "sun")
        .expect("sun placement must exist");
    assert_eq!(sun_placement["sidereal_rashi"].as_str(), Some("dhanu"));
    assert_eq!(sun_placement["d3_rashi"].as_str(), Some("mesha"));
    assert_eq!(sun_placement["d9_rashi"].as_str(), Some("simha"));
    assert_eq!(sun_placement["whole_sign_house"].as_u64(), Some(10));
    assert_eq!(sun_placement["sign_lord"].as_str(), Some("jupiter"));
    assert_eq!(sun_placement["house_context"]["house_lord"].as_str(), Some("jupiter"));
    let house_occupancy =
        json["data"]["summary"]["houses"].as_array().expect("summary.houses array must exist");
    assert_eq!(house_occupancy.len(), 12);
    assert_eq!(house_occupancy[0]["house"].as_u64(), Some(1));
    assert_eq!(house_occupancy[0]["occupants"], serde_json::json!(["rahu"]));
    assert_eq!(house_occupancy[1]["occupants"], serde_json::json!(["jupiter"]));
    assert_eq!(house_occupancy[4]["occupants"], serde_json::json!(["moon"]));
    assert_eq!(house_occupancy[5]["occupants"], serde_json::json!(["mercury"]));
    assert_eq!(house_occupancy[6]["occupants"], serde_json::json!(["ketu"]));
    assert_eq!(house_occupancy[7]["occupants"], serde_json::json!(["mars"]));
    assert_eq!(house_occupancy[8]["occupants"], serde_json::json!(["venus"]));
    assert_eq!(house_occupancy[9]["occupants"], serde_json::json!(["sun"]));
    assert_eq!(house_occupancy[11]["occupants"], serde_json::json!(["saturn"]));
    let retrograde_bodies = json["data"]["summary"]["motion"]["retrograde_bodies"]
        .as_array()
        .expect("retrograde_bodies array must exist");
    assert!(retrograde_bodies.is_empty());
    assert_eq!(json["data"]["summary"]["motion"]["fastest"]["body"].as_str(), Some("sun"));
    assert_eq!(
        json["data"]["summary"]["motion"]["fastest"]["longitude_speed_deg_per_day"].as_f64(),
        Some(0.0)
    );
    let house_lords =
        json["data"]["summary"]["house_lords"].as_array().expect("house_lords array must exist");
    assert_eq!(house_lords.len(), 12);
    assert_eq!(house_lords[0].as_str(), Some("jupiter"));
    assert_eq!(house_lords[1].as_str(), Some("mars"));
    assert_eq!(json["data"]["dasha"]["as_of_utc"].as_str(), Some("2000-01-01T12:00:00Z"));
    assert_eq!(json["data"]["dasha"]["birth_nakshatra"].as_str(), Some("pushya"));
    assert_eq!(json["data"]["dasha"]["birth_pada"].as_u64(), Some(2));
    assert_eq!(json["data"]["dasha"]["current"]["maha"]["lord"].as_str(), Some("saturn"));
    assert_eq!(json["data"]["dasha"]["current"]["antar"]["lord"].as_str(), Some("saturn"));
    assert_eq!(json["data"]["dasha"]["current"]["pratyantar"]["lord"].as_str(), Some("saturn"));
    assert_eq!(json["data"]["node_policy"].as_str(), Some(astro_api::NODE_POLICY_ID));
    assert_eq!(json["data"]["lahiri_algorithm"].as_str(), Some(astro_vedic::LAHIRI_ALGO_ID));
    assert!(json["data"]["moon_sidereal_longitude_deg"].is_number());
    assert!(json["data"]["moon_nakshatra"].is_string());
    assert!(json["data"]["moon_pada"].is_number());
}

#[tokio::test(flavor = "current_thread")]
async fn sidereal_chart_with_de440_kernel_reports_real_backend_metadata() {
    if de440_kernel::require_de440_kernel().is_none() {
        return;
    }
    let app = app_router_with_api_keys(
        de440_state_from_env().expect("DE440 state must load"),
        [TEST_API_KEY],
    );

    let response = app
        .oneshot(
            authenticated_request(Method::POST, "/chart/sidereal")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"datetime":{"kind":"utc","utc":"2024-01-01T00:00:00Z"},"geo":{"latitude_deg":12.9716,"longitude_deg":77.5946,"elevation_m":920.0},"ayanamsa":"lahiri","gravitational_deflection":false}"#,
                ))
                .expect("request must build"),
        )
        .await
        .expect("response must succeed");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.expect("body must be readable");
    let json: Value = serde_json::from_slice(&body).expect("body must be valid json");

    let grahas = json["data"]["grahas"].as_array().expect("grahas array must exist");
    assert!(!grahas.is_empty());
    for graha in grahas {
        let kernel = graha["computation_meta"]["kernel"].as_str().expect("kernel must exist");
        assert!(kernel.starts_with("de440_"), "expected DE440 kernel metadata, got {kernel}");
        assert_ne!(kernel, "in_memory");
    }
}

#[tokio::test(flavor = "current_thread")]
async fn sidereal_chart_sidereal_only_projection_omits_tropical_fields() {
    let app = app_router_with_api_keys(demo_state(), [TEST_API_KEY]);

    let response = app
        .oneshot(
            authenticated_request(Method::POST, "/chart/sidereal")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"datetime":{"kind":"utc","utc":"2000-01-01T12:00:00Z"},"geo":{"latitude_deg":12.9716,"longitude_deg":77.5946,"elevation_m":920.0},"ayanamsa":"lahiri","projection":"sidereal_only","compact":true}"#,
                ))
                .expect("request must build"),
        )
        .await
        .expect("response must succeed");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.expect("body must be readable");
    let json: Value = serde_json::from_slice(&body).expect("body must be valid json");

    let graha = &json["data"]["grahas"][0];
    assert!(graha["sidereal_longitude_deg"].is_number());
    assert!(graha["tropical_longitude_deg"].is_null());
    assert!(graha["tropical_latitude_deg"].is_null());
    assert!(json["data"]["summary"]["houses"].is_array());
    assert!(json["data"]["houses"].is_null());
    assert!(json["data"]["dasha"].is_null());
}

#[tokio::test(flavor = "current_thread")]
async fn protected_post_routes_return_401_without_api_key() {
    let app = app_router_with_api_keys(demo_state(), [TEST_API_KEY]);

    for (method, uri, body) in [
        (Method::POST, "/positions", r#"{"julian_day":2451545.0,"bodies":["moon"]}"#),
        (
            Method::POST,
            "/positions/sidereal",
            r#"{"datetime":{"kind":"utc","utc":"2000-01-01T12:00:00Z"},"geo":{"latitude_deg":12.97,"longitude_deg":77.59,"elevation_m":920.0},"ayanamsa":"lahiri","bodies":["moon"]}"#,
        ),
        (
            Method::POST,
            "/chart/sidereal",
            r#"{"datetime":{"kind":"utc","utc":"2000-01-01T12:00:00Z"},"geo":{"latitude_deg":12.97,"longitude_deg":77.59,"elevation_m":920.0},"ayanamsa":"lahiri"}"#,
        ),
        (
            Method::POST,
            "/dasha",
            r#"{"moon_sidereal_longitude_deg":15.0,"birth_time_utc_rfc3339":"2024-01-01T00:00:00Z"}"#,
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .expect("request must build"),
            )
            .await
            .expect("response must succeed");

        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "expected 401 for {uri} without credentials"
        );
        let bytes =
            to_bytes(response.into_body(), usize::MAX).await.expect("body must be readable");
        let json: Value = serde_json::from_slice(&bytes).expect("body must be json");
        assert_eq!(json["error"].as_str(), Some("missing_api_key"));
    }
}

#[tokio::test(flavor = "current_thread")]
async fn protected_post_routes_return_401_for_invalid_api_key() {
    let app = app_router_with_api_keys(demo_state(), [TEST_API_KEY]);

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/positions/sidereal")
                .header("x-api-key", "wrong-key")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"datetime":{"kind":"utc","utc":"2000-01-01T12:00:00Z"},"geo":{"latitude_deg":12.97,"longitude_deg":77.59,"elevation_m":920.0},"ayanamsa":"lahiri","bodies":["moon"]}"#,
                ))
                .expect("request must build"),
        )
        .await
        .expect("response must succeed");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("body must be readable");
    let json: Value = serde_json::from_slice(&bytes).expect("body must be json");
    assert_eq!(json["error"].as_str(), Some("invalid_api_key"));
}

#[tokio::test(flavor = "current_thread")]
async fn authenticated_sidereal_and_chart_routes_match_expected_contract_shapes() {
    let app = app_router_with_api_keys(demo_state(), [TEST_API_KEY]);

    let sidereal = app
        .clone()
        .oneshot(
            authenticated_request(Method::POST, "/positions/sidereal")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"datetime":{"kind":"utc","utc":"2000-01-01T12:00:00Z"},"geo":{"latitude_deg":12.9716,"longitude_deg":77.5946,"elevation_m":920.0},"ayanamsa":"lahiri","bodies":["moon","sun"],"compact":true,"projection":"sidereal_only"}"#,
                ))
                .expect("request must build"),
        )
        .await
        .expect("sidereal response");

    assert_eq!(sidereal.status(), StatusCode::OK);
    let body = to_bytes(sidereal.into_body(), usize::MAX).await.expect("body must be readable");
    let sidereal_json: Value = serde_json::from_slice(&body).expect("sidereal json");
    assert_eq!(sidereal_json["metadata"]["engine_mode"].as_str(), Some("vedic"));
    assert!(sidereal_json["data"]["positions"].is_array());

    let chart = app
        .oneshot(
            authenticated_request(Method::POST, "/chart/sidereal")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"datetime":{"kind":"utc","utc":"1990-05-17T04:30:00Z"},"geo":{"latitude_deg":28.6139,"longitude_deg":77.2090,"elevation_m":216.0},"ayanamsa":"lahiri","compact":true,"projection":"sidereal_only"}"#,
                ))
                .expect("request must build"),
        )
        .await
        .expect("chart response");

    assert_eq!(chart.status(), StatusCode::OK);
    let body = to_bytes(chart.into_body(), usize::MAX).await.expect("body must be readable");
    let chart_json: Value = serde_json::from_slice(&body).expect("chart json");
    assert_eq!(chart_json["data"]["schema_version"].as_str(), Some(CHART_SIDEREAL_SCHEMA_VERSION));
    assert_eq!(chart_json["data"]["house_system"].as_str(), Some("whole_sign"));
    assert!(chart_json["data"]["summary"]["placement_table"].is_array());
    assert!(chart_json["data"]["grahas"].as_array().is_some_and(|rows| !rows.is_empty()));
    assert!(chart_json["data"]["houses"].is_null());
    assert!(chart_json["data"]["dasha"].is_null());
    assert!(chart_json["data"]["lagna"]["sidereal_longitude_deg"].is_number());
}
