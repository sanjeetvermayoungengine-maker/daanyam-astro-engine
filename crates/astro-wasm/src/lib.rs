use astro_core::math::normalize_degrees;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub fn normalize_angle(angle_deg: f64) -> f64 {
    normalize_degrees(angle_deg)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartProjectionBinding {
    Full,
    SiderealOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiderealRequestOptions {
    pub compact: Option<bool>,
    pub projection: Option<ChartProjectionBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionsRequestOptions {
    pub compact: Option<bool>,
}

pub fn default_sidereal_request_options() -> SiderealRequestOptions {
    SiderealRequestOptions { compact: None, projection: Some(ChartProjectionBinding::Full) }
}

pub fn default_positions_request_options() -> PositionsRequestOptions {
    PositionsRequestOptions { compact: None }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum JsonProjection {
    Full,
    SiderealOnly,
}

#[derive(Serialize)]
struct JsonUtcDateTimeInput {
    kind: &'static str,
    utc: String,
}

#[derive(Serialize)]
struct JsonGeoInput {
    latitude_deg: f64,
    longitude_deg: f64,
    elevation_m: Option<f64>,
}

#[derive(Serialize)]
struct JsonSiderealPositionsRequest {
    datetime: JsonUtcDateTimeInput,
    geo: JsonGeoInput,
    ayanamsa: &'static str,
    bodies: Vec<String>,
    gravitational_deflection: Option<bool>,
    compact: Option<bool>,
    projection: Option<JsonProjection>,
}

#[derive(Serialize)]
struct JsonSiderealChartRequest {
    datetime: JsonUtcDateTimeInput,
    geo: JsonGeoInput,
    ayanamsa: &'static str,
    gravitational_deflection: Option<bool>,
    as_of: Option<JsonUtcDateTimeInput>,
    compact: Option<bool>,
    projection: Option<JsonProjection>,
}

#[derive(Serialize)]
struct JsonPositionsRequest {
    julian_day: f64,
    bodies: Vec<String>,
    compact: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMetadata {
    pub engine_mode: Option<String>,
    pub ayanamsa_used: Option<String>,
    pub house_system: Option<String>,
    pub gravitational_deflection: Option<bool>,
    pub engine_semantic_version: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TropicalPositionValue {
    pub body: String,
    pub longitude_deg: f64,
    pub latitude_deg: f64,
    pub distance_au: Option<f64>,
    pub frame: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TropicalPositionResponseEntry {
    pub position: TropicalPositionValue,
    pub computation_meta: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TropicalPositionsResponseData {
    pub positions: Vec<TropicalPositionResponseEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TropicalPositionsResponse {
    pub data: TropicalPositionsResponseData,
    pub metadata: ApiMetadata,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SiderealPositionResponseEntry {
    pub body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tropical_longitude_deg: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tropical_latitude_deg: Option<f64>,
    pub sidereal_longitude_deg: Option<f64>,
    pub longitude_speed_deg_per_day: Option<f64>,
    pub retrograde: Option<bool>,
    pub distance_au: Option<f64>,
    pub moon_division: Option<serde_json::Value>,
    pub computation_meta: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SiderealPositionsResponseData {
    pub positions: Vec<SiderealPositionResponseEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SiderealPositionsResponse {
    pub data: SiderealPositionsResponseData,
    pub metadata: ApiMetadata,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChartSummaryResponse {
    pub moon_rashi: Option<String>,
    pub lagna_rashi: Option<String>,
    pub lagna_lord: Option<String>,
    pub house_lords: Option<Vec<String>>,
    pub houses: Option<Vec<serde_json::Value>>,
    pub grahas_by_rashi: Option<serde_json::Map<String, serde_json::Value>>,
    pub dispositors: Option<Vec<serde_json::Value>>,
    pub placement_table: Option<Vec<serde_json::Value>>,
    pub motion: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChartResponseData {
    pub schema_version: String,
    pub extensions: serde_json::Map<String, serde_json::Value>,
    pub summary: ChartSummaryResponse,
    pub grahas: Vec<serde_json::Value>,
    pub lagna: serde_json::Value,
    pub houses: Option<Vec<serde_json::Value>>,
    pub house_system: String,
    pub moon_sidereal_longitude_deg: f64,
    pub moon_nakshatra: String,
    pub moon_pada: u8,
    pub dasha: Option<serde_json::Value>,
    pub node_policy: String,
    pub lahiri_algorithm: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChartSiderealResponse {
    pub data: ChartResponseData,
    pub metadata: ApiMetadata,
}

fn map_projection(projection: Option<ChartProjectionBinding>) -> Option<JsonProjection> {
    match projection {
        Some(ChartProjectionBinding::Full) => Some(JsonProjection::Full),
        Some(ChartProjectionBinding::SiderealOnly) => Some(JsonProjection::SiderealOnly),
        None => None,
    }
}

pub fn build_positions_sidereal_request_json(
    datetime_utc: &str,
    latitude_deg: f64,
    longitude_deg: f64,
    elevation_m: Option<f64>,
    bodies: Vec<String>,
    gravitational_deflection: Option<bool>,
    options: Option<SiderealRequestOptions>,
) -> String {
    let options = options.unwrap_or_else(default_sidereal_request_options);
    serde_json::to_string(&JsonSiderealPositionsRequest {
        datetime: JsonUtcDateTimeInput { kind: "utc", utc: datetime_utc.to_owned() },
        geo: JsonGeoInput { latitude_deg, longitude_deg, elevation_m },
        ayanamsa: "lahiri",
        bodies,
        gravitational_deflection,
        compact: options.compact,
        projection: map_projection(options.projection),
    })
    .expect("sidereal positions request json must serialize")
}

pub fn build_chart_sidereal_request_json(
    datetime_utc: &str,
    latitude_deg: f64,
    longitude_deg: f64,
    elevation_m: Option<f64>,
    gravitational_deflection: Option<bool>,
    as_of_utc: Option<&str>,
    options: Option<SiderealRequestOptions>,
) -> String {
    let options = options.unwrap_or_else(default_sidereal_request_options);
    serde_json::to_string(&JsonSiderealChartRequest {
        datetime: JsonUtcDateTimeInput { kind: "utc", utc: datetime_utc.to_owned() },
        geo: JsonGeoInput { latitude_deg, longitude_deg, elevation_m },
        ayanamsa: "lahiri",
        gravitational_deflection,
        as_of: as_of_utc.map(|utc| JsonUtcDateTimeInput { kind: "utc", utc: utc.to_owned() }),
        compact: options.compact,
        projection: map_projection(options.projection),
    })
    .expect("sidereal chart request json must serialize")
}

pub fn build_positions_request_json(
    julian_day: f64,
    bodies: Vec<String>,
    options: Option<PositionsRequestOptions>,
) -> String {
    let options = options.unwrap_or_else(default_positions_request_options);
    serde_json::to_string(&JsonPositionsRequest { julian_day, bodies, compact: options.compact })
        .expect("positions request json must serialize")
}

pub fn parse_positions_response_json(json: &str) -> String {
    let parsed: TropicalPositionsResponse =
        serde_json::from_str(json).expect("positions response json must deserialize");
    serde_json::to_string(&parsed).expect("positions response json must serialize")
}

pub fn parse_positions_sidereal_response_json(json: &str) -> String {
    let parsed: SiderealPositionsResponse =
        serde_json::from_str(json).expect("sidereal positions response json must deserialize");
    serde_json::to_string(&parsed).expect("sidereal positions response json must serialize")
}

pub fn parse_chart_sidereal_response_json(json: &str) -> String {
    let parsed: ChartSiderealResponse =
        serde_json::from_str(json).expect("sidereal chart response json must deserialize");
    serde_json::to_string(&parsed).expect("sidereal chart response json must serialize")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::path::PathBuf;

    fn example_json(path: &str) -> String {
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .canonicalize()
            .expect("crate manifest dir must resolve")
            .parent()
            .and_then(|path| path.parent())
            .map(PathBuf::from)
            .expect("workspace root must resolve");
        std::fs::read_to_string(workspace_root.join(path)).expect("example fixture must load")
    }

    #[test]
    fn wasm_surface_reuses_core_math() {
        assert_eq!(normalize_angle(361.0), 1.0);
    }

    #[test]
    fn wasm_surface_exposes_projection_and_compact_options() {
        let options = default_sidereal_request_options();
        assert_eq!(options.compact, None);
        assert!(matches!(options.projection, Some(ChartProjectionBinding::Full)));
        let positions = default_positions_request_options();
        assert_eq!(positions.compact, None);
    }

    #[test]
    fn wasm_positions_sidereal_request_helper_matches_http_shape() {
        let json = build_positions_sidereal_request_json(
            "2000-01-01T12:00:00Z",
            12.9716,
            77.5946,
            Some(920.0),
            vec!["moon".to_owned()],
            Some(false),
            Some(SiderealRequestOptions {
                compact: Some(true),
                projection: Some(ChartProjectionBinding::SiderealOnly),
            }),
        );
        let value: Value = serde_json::from_str(&json).expect("json must parse");
        assert_eq!(value["datetime"]["kind"], "utc");
        assert_eq!(value["projection"], "sidereal_only");
        assert_eq!(value["compact"], true);
    }

    #[test]
    fn wasm_chart_request_helper_matches_http_shape() {
        let json = build_chart_sidereal_request_json(
            "2000-01-01T12:00:00Z",
            12.9716,
            77.5946,
            Some(920.0),
            Some(false),
            Some("2000-01-02T00:00:00Z"),
            Some(SiderealRequestOptions {
                compact: Some(true),
                projection: Some(ChartProjectionBinding::SiderealOnly),
            }),
        );
        let value: Value = serde_json::from_str(&json).expect("json must parse");
        assert_eq!(value["datetime"]["kind"], "utc");
        assert_eq!(value["as_of"]["utc"], "2000-01-02T00:00:00Z");
        assert_eq!(value["projection"], "sidereal_only");
    }

    #[test]
    fn wasm_positions_tropical_request_helper_matches_http_shape() {
        let json = build_positions_request_json(
            2451545.0,
            vec!["moon".to_owned(), "sun".to_owned()],
            Some(PositionsRequestOptions { compact: Some(true) }),
        );
        let value: Value = serde_json::from_str(&json).expect("json must parse");
        assert_eq!(value["julian_day"], 2451545.0);
        assert_eq!(value["compact"], true);
        assert_eq!(value["bodies"][0], "moon");
    }

    #[test]
    fn wasm_positions_response_parser_round_trips_example_fixture() {
        let fixture = example_json("docs/examples/mobile_positions_tropical_compact.json");
        let round_trip = parse_positions_response_json(&fixture);
        let parsed_fixture: Value = serde_json::from_str(&fixture).expect("fixture json");
        let parsed_round_trip: Value = serde_json::from_str(&round_trip).expect("round trip json");
        assert_eq!(parsed_fixture, parsed_round_trip);
    }

    #[test]
    fn wasm_positions_sidereal_response_parser_round_trips_example_fixture() {
        let fixture =
            example_json("docs/examples/mobile_positions_sidereal_compact_sidereal_only.json");
        let round_trip = parse_positions_sidereal_response_json(&fixture);
        let parsed_fixture: Value = serde_json::from_str(&fixture).expect("fixture json");
        let parsed_round_trip: Value = serde_json::from_str(&round_trip).expect("round trip json");
        assert_eq!(parsed_fixture, parsed_round_trip);
    }

    #[test]
    fn wasm_chart_sidereal_response_parser_round_trips_example_fixture() {
        let fixture = example_json("docs/examples/mobile_chart_compact_sidereal_only.json");
        let round_trip = parse_chart_sidereal_response_json(&fixture);
        let parsed_fixture: Value = serde_json::from_str(&fixture).expect("fixture json");
        let parsed_round_trip: Value = serde_json::from_str(&round_trip).expect("round trip json");
        assert_eq!(parsed_fixture, parsed_round_trip);
    }
}
