//! Yoga registry — collects every implemented yoga and runs them in a
//! stable order so API responses are deterministic.

use super::{chandra_mangal::ChandraMangal, gajakesari::Gajakesari};
use super::{DetectedYoga, Yoga, YogaChartFacts};

/// Stable list of all yoga keys recognised by this engine version.
/// Used in the OpenAPI schema and for client-side filter/sort.
pub fn all_yoga_keys() -> Vec<&'static str> {
    vec![
        Gajakesari.key(),
        ChandraMangal.key(),
        // 28+ more yoga keys go here as detectors are added — see
        // docs/engine/yogas-roadmap.md for the priority-ordered list.
    ]
}

/// Run every detector against the chart facts. Returns only the detected
/// yogas (None results are dropped). Order is stable across requests.
pub fn detect_yogas(facts: &YogaChartFacts) -> Vec<DetectedYoga> {
    let detectors: Vec<Box<dyn Yoga>> = vec![Box::new(Gajakesari), Box::new(ChandraMangal)];

    detectors.into_iter().filter_map(|d| d.detect(facts)).collect()
}

#[cfg(test)]
mod tests {
    use super::super::{PlanetHouses, PlanetLongitudes, YogaChartFacts};
    use super::*;
    use crate::Rashi;

    #[test]
    fn registry_returns_only_detected_yogas() {
        let facts = YogaChartFacts {
            lagna_rashi: Rashi::Karka,
            planet_longitudes: PlanetLongitudes {
                moon: Some(100.0),
                mars: Some(102.0),
                jupiter: Some(280.0),
                ..Default::default()
            },
            planet_houses: PlanetHouses {
                moon: Some(1),
                mars: Some(1),
                jupiter: Some(7),
                ..Default::default()
            },
        };
        let yogas = detect_yogas(&facts);
        // Moon in 1, Jupiter in 7 → Gajakesari; Moon+Mars in same house → Chandra-Mangal.
        let keys: Vec<&str> = yogas.iter().map(|y| y.key.as_str()).collect();
        assert!(keys.contains(&"gajakesari"));
        assert!(keys.contains(&"chandra_mangal"));
    }

    #[test]
    fn registry_returns_empty_when_no_yogas_apply() {
        let facts = YogaChartFacts {
            lagna_rashi: Rashi::Mesha,
            planet_longitudes: PlanetLongitudes::default(),
            planet_houses: PlanetHouses::default(),
        };
        let yogas = detect_yogas(&facts);
        assert!(yogas.is_empty());
    }
}
