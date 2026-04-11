use astro_core::{CelestialBody, CoordinateFrame, De440Backend, EngineConfig, EphemerisBackend};
use std::collections::HashMap;

use serde::Deserialize;

#[path = "../../../tests/support/de440_kernel.rs"]
mod de440_kernel;

#[derive(Debug, Deserialize)]
struct RegressionManifest {
    groups: Vec<RegressionGroup>,
}

#[derive(Debug, Deserialize)]
struct RegressionGroup {
    name: String,
    source_type: String,
    kernel_file: String,
    tolerance_deg: f64,
    horizons_params: HorizonsParams,
    vectors: Vec<RegressionVector>,
}

#[derive(Debug, Deserialize)]
struct HorizonsParams {
    quantities: String,
    apparent: String,
    center: String,
    ephem_type: String,
    ang_format: String,
    ref_system: String,
    atmospheric_refraction: String,
    gravitational_deflection: bool,
    node_policy: Option<String>,
    notes: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RegressionVector {
    body: String,
    utc: String,
    observer: String,
    frame: String,
    expected_longitude_deg: f64,
    expected_latitude_deg: f64,
}

#[test]
fn de440_horizons_regression_vectors_match() {
    let manifest = std::fs::read_to_string("../../tests/regression/manifest.json")
        .expect("regression manifest must exist");
    let manifest: RegressionManifest =
        serde_json::from_str(&manifest).expect("regression manifest must parse");
    let Some(path) = de440_kernel::require_de440_kernel() else {
        return;
    };
    let backend = De440Backend::from_path(&path).expect("DE440 backend must load");

    for group in manifest.groups {
        assert_eq!(group.horizons_params.quantities, "31");
        assert_eq!(group.horizons_params.apparent, "airless");
        assert_eq!(group.horizons_params.center, "500@399");
        assert_eq!(group.horizons_params.ephem_type, "observer");
        assert_eq!(group.horizons_params.ang_format, "deg");
        assert_eq!(group.horizons_params.ref_system, "icrf");
        assert_eq!(group.horizons_params.atmospheric_refraction, "no");
        assert_eq!(group.kernel_file, "de440.bsp");
        assert!(
            matches!(group.source_type.as_str(), "jpl_horizons" | "manual_reference"),
            "unexpected source type for {}: {}",
            group.name,
            group.source_type
        );
        assert!(group.tolerance_deg > 0.0, "group tolerance must be positive");

        if let Some(node_policy) = &group.horizons_params.node_policy {
            assert!(matches!(node_policy.as_str(), "true"));
        }
        if let Some(notes) = &group.horizons_params.notes {
            assert!(!notes.trim().is_empty(), "group notes must not be empty");
        }

        let config = EngineConfig {
            gravitational_deflection: group.horizons_params.gravitational_deflection,
            node_mode: parse_node_mode(group.horizons_params.node_policy.as_deref()),
            ..EngineConfig::default()
        };

        for vector in group.vectors {
            assert_eq!(vector.observer, "geocenter");

            let jd = rfc3339_to_julian_day(&vector.utc);
            let position = backend
                .position(
                    parse_body(&vector.body),
                    jd,
                    CoordinateFrame::EclipticGeocentric,
                    None,
                    &config,
                )
                .unwrap_or_else(|err| {
                    panic!(
                        "{} {} in group {} failed to compute: {err}",
                        vector.body, vector.utc, group.name
                    )
                });

            let longitude_error = circular_difference_deg(
                position.position.longitude_deg,
                vector.expected_longitude_deg,
            );
            let latitude_error = position.position.latitude_deg - vector.expected_latitude_deg;

            assert_eq!(
                position.computation_meta.gravitational_deflection,
                group.horizons_params.gravitational_deflection
            );
            if let Some(node_policy) = group.horizons_params.node_policy.as_deref() {
                assert_eq!(
                    position.computation_meta.node_policy,
                    Some(parse_node_mode(Some(node_policy)))
                );
            }
            assert_eq!(position.computation_meta.frame, vector.frame);
            assert!(
                longitude_error.abs() <= group.tolerance_deg,
                "{} {} [{}] longitude mismatch: actual={}, expected={}, error={}",
                vector.body,
                vector.utc,
                group.name,
                position.position.longitude_deg,
                vector.expected_longitude_deg,
                longitude_error
            );
            assert!(
                latitude_error.abs() <= group.tolerance_deg,
                "{} {} [{}] latitude mismatch: actual={}, expected={}, error={}",
                vector.body,
                vector.utc,
                group.name,
                position.position.latitude_deg,
                vector.expected_latitude_deg,
                latitude_error
            );
        }
    }
}

#[test]
fn deflection_fixture_groups_are_distinct_and_auditable() {
    let manifest = std::fs::read_to_string("../../tests/regression/manifest.json")
        .expect("regression manifest must exist");
    let manifest: RegressionManifest =
        serde_json::from_str(&manifest).expect("regression manifest must parse");
    let Some(path) = de440_kernel::require_de440_kernel() else {
        return;
    };
    let backend = De440Backend::from_path(&path).expect("DE440 backend must load");

    let on_group = manifest
        .groups
        .iter()
        .find(|group| group.name == "horizons_deflection_on")
        .expect("deflection-on group must exist");
    let off_group = manifest
        .groups
        .iter()
        .find(|group| group.name == "horizons_deflection_off")
        .expect("deflection-off group must exist");

    assert!(on_group.horizons_params.gravitational_deflection);
    assert!(!off_group.horizons_params.gravitational_deflection);

    let off_lookup = off_group
        .vectors
        .iter()
        .map(|vector| ((vector.body.as_str(), vector.utc.as_str()), vector))
        .collect::<HashMap<_, _>>();

    for on_vector in &on_group.vectors {
        let off_vector = off_lookup
            .get(&(on_vector.body.as_str(), on_vector.utc.as_str()))
            .expect("deflection-off vector must pair with deflection-on vector");

        let longitude_shift = circular_difference_deg(
            on_vector.expected_longitude_deg,
            off_vector.expected_longitude_deg,
        )
        .abs();
        let latitude_shift =
            (on_vector.expected_latitude_deg - off_vector.expected_latitude_deg).abs();
        let jd = rfc3339_to_julian_day(&on_vector.utc);
        let on_position = backend
            .position(
                parse_body(&on_vector.body),
                jd,
                CoordinateFrame::EclipticGeocentric,
                None,
                &EngineConfig { gravitational_deflection: true, ..EngineConfig::default() },
            )
            .expect("deflection-on position must compute");
        let off_position = backend
            .position(
                parse_body(&off_vector.body),
                jd,
                CoordinateFrame::EclipticGeocentric,
                None,
                &EngineConfig { gravitational_deflection: false, ..EngineConfig::default() },
            )
            .expect("deflection-off position must compute");

        assert!(
            longitude_shift > 1.0e-9 || latitude_shift > 1.0e-12,
            "{} {} must change when deflection toggles",
            on_vector.body,
            on_vector.utc
        );
        assert!(on_position.computation_meta.gravitational_deflection);
        assert!(!off_position.computation_meta.gravitational_deflection);
        assert!(
            circular_difference_deg(
                on_position.position.longitude_deg,
                on_vector.expected_longitude_deg
            )
            .abs()
                <= on_group.tolerance_deg
        );
        assert!(
            circular_difference_deg(
                off_position.position.longitude_deg,
                off_vector.expected_longitude_deg
            )
            .abs()
                <= off_group.tolerance_deg
        );
    }
}

fn parse_body(body: &str) -> CelestialBody {
    match body {
        "moon" => CelestialBody::Moon,
        "sun" => CelestialBody::Sun,
        "mercury" => CelestialBody::Mercury,
        "venus" => CelestialBody::Venus,
        "mars" => CelestialBody::Mars,
        "jupiter" => CelestialBody::Jupiter,
        "saturn" => CelestialBody::Saturn,
        "rahu" => CelestialBody::Rahu,
        "ketu" => CelestialBody::Ketu,
        other => panic!("unsupported regression body: {other}"),
    }
}

fn parse_node_mode(node_policy: Option<&str>) -> astro_core::NodeMode {
    match node_policy.unwrap_or("true") {
        "true" => astro_core::NodeMode::True,
        "mean" => astro_core::NodeMode::Mean,
        other => panic!("unsupported node policy: {other}"),
    }
}

fn rfc3339_to_julian_day(timestamp: &str) -> f64 {
    let datetime = chrono::DateTime::parse_from_rfc3339(timestamp)
        .expect("timestamp must parse")
        .with_timezone(&chrono::Utc);
    astro_core::time::julian_day(datetime)
}

fn circular_difference_deg(actual_deg: f64, expected_deg: f64) -> f64 {
    (actual_deg - expected_deg + 180.0).rem_euclid(360.0) - 180.0
}
