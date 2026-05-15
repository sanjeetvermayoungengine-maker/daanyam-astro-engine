//! Horizons-vetted Jupiter and Saturn station-window regressions (DE440).

use astro_core::{
    motion::{longitude_motion, LONGITUDE_SPEED_DELTA_DAYS},
    time::julian_day,
    CelestialBody, De440Backend, EngineConfig,
};
use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;
use std::path::Path;

#[path = "../../../tests/support/de440_kernel.rs"]
mod de440_kernel;

#[derive(Debug, Deserialize)]
struct StationCatalog {
    body: String,
    #[allow(dead_code)]
    horizons_target: String,
    fixtures: Vec<StationFixture>,
}

#[derive(Debug, Deserialize)]
struct StationFixture {
    utc: String,
    #[allow(dead_code)]
    horizons_url: String,
    expected_speed_sign: String,
    station_kind: String,
    #[allow(dead_code)]
    tolerance_deg: f64,
}

const STATION_WINDOW_DAYS: f64 = 0.5;

#[test]
fn outer_planet_station_fixtures_match_speed_sign_contract() {
    let Some(path) = de440_kernel::require_de440_kernel() else {
        return;
    };
    let backend = De440Backend::from_path(&path).expect("DE440 backend must load");
    let config = EngineConfig::default();

    for catalog_path in [
        "../../tests/golden/horizons_stations/jupiter.json",
        "../../tests/golden/horizons_stations/saturn.json",
    ] {
        let catalog = load_catalog(catalog_path);
        let body = parse_body(&catalog.body);

        for fixture in &catalog.fixtures {
            let jd = rfc3339_to_julian_day(&fixture.utc);
            let motion = longitude_motion(&backend, body, jd, &config).unwrap_or_else(|err| {
                panic!("{} {} motion failed: {err}", catalog.body, fixture.utc)
            });

            let expected_negative = fixture.expected_speed_sign == "negative";
            assert_eq!(
                motion.retrograde, expected_negative,
                "{} {} retrograde flag must match expected_speed_sign={}",
                catalog.body, fixture.utc, fixture.expected_speed_sign
            );
            if expected_negative {
                assert!(
                    motion.longitude_speed_deg_per_day < 0.0,
                    "{} {} speed must be negative, got {}",
                    catalog.body,
                    fixture.utc,
                    motion.longitude_speed_deg_per_day
                );
            } else {
                assert!(
                    motion.longitude_speed_deg_per_day > 0.0,
                    "{} {} speed must be positive, got {}",
                    catalog.body,
                    fixture.utc,
                    motion.longitude_speed_deg_per_day
                );
            }

            assert_station_sign_flip_within_window(
                &backend,
                body,
                jd,
                &fixture.station_kind,
                &config,
                &catalog.body,
                &fixture.utc,
            );
        }
    }
}

fn assert_station_sign_flip_within_window(
    backend: &De440Backend,
    body: CelestialBody,
    jd: f64,
    station_kind: &str,
    config: &EngineConfig,
    body_name: &str,
    utc: &str,
) {
    let before = longitude_motion(backend, body, jd - STATION_WINDOW_DAYS, config)
        .expect("before-window motion");
    let after = longitude_motion(backend, body, jd + STATION_WINDOW_DAYS, config)
        .expect("after-window motion");

    match station_kind {
        "retrograde_entry" => {
            assert!(
                before.longitude_speed_deg_per_day > 0.0 && after.longitude_speed_deg_per_day < 0.0,
                "{body_name} {utc} retrograde_entry: speed must flip + to - within ±{STATION_WINDOW_DAYS}d (before={}, after={})",
                before.longitude_speed_deg_per_day,
                after.longitude_speed_deg_per_day
            );
        }
        "direct" => {
            assert!(
                before.longitude_speed_deg_per_day < 0.0 && after.longitude_speed_deg_per_day > 0.0,
                "{body_name} {utc} direct: speed must flip - to + within ±{STATION_WINDOW_DAYS}d (before={}, after={})",
                before.longitude_speed_deg_per_day,
                after.longitude_speed_deg_per_day
            );
        }
        other => panic!("unsupported station_kind: {other}"),
    }

    assert!(
        (STATION_WINDOW_DAYS - LONGITUDE_SPEED_DELTA_DAYS).abs() < 1e-9,
        "station window must match motion delta for flip semantics"
    );
}

fn load_catalog(path: &str) -> StationCatalog {
    let raw =
        std::fs::read_to_string(path).unwrap_or_else(|err| panic!("{path} must exist: {err}"));
    serde_json::from_str(&raw).unwrap_or_else(|err| panic!("{path} must parse: {err}"))
}

fn parse_body(body: &str) -> CelestialBody {
    match body {
        "jupiter" => CelestialBody::Jupiter,
        "saturn" => CelestialBody::Saturn,
        other => panic!("unsupported station catalog body: {other}"),
    }
}

fn rfc3339_to_julian_day(timestamp: &str) -> f64 {
    let datetime: DateTime<Utc> = timestamp.parse().expect("timestamp must parse");
    julian_day(datetime)
}

#[test]
fn horizons_station_fixture_files_stay_small() {
    let root = Path::new("../../tests/golden/horizons_stations");
    let mut total = 0u64;
    for name in ["jupiter.json", "saturn.json"] {
        let path = root.join(name);
        let len = std::fs::metadata(&path)
            .unwrap_or_else(|err| panic!("{} must exist: {err}", path.display()))
            .len();
        total += len;
    }
    assert!(
        total < 50 * 1024,
        "horizons_stations fixtures must stay under 50KB total, got {total} bytes"
    );
}

#[test]
fn station_fixture_utc_range_is_2020_through_2030() {
    let start = Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
    let end = Utc.with_ymd_and_hms(2030, 12, 31, 23, 59, 59).unwrap();

    for catalog_path in [
        "../../tests/golden/horizons_stations/jupiter.json",
        "../../tests/golden/horizons_stations/saturn.json",
    ] {
        let catalog = load_catalog(catalog_path);
        for fixture in &catalog.fixtures {
            let instant: DateTime<Utc> = fixture.utc.parse().expect("utc must parse");
            assert!(instant >= start, "{} before 2020", fixture.utc);
            assert!(instant <= end, "{} after 2030", fixture.utc);
        }
    }
}
