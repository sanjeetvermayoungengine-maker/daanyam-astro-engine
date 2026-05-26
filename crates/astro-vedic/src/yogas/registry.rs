//! Yoga registry — collects every implemented yoga and runs them in a
//! stable order so API responses are deterministic.

use super::anapha::Anapha;
use super::budhaditya::Budhaditya;
use super::chandra_mangal::ChandraMangal;
use super::dhana::Dhana;
use super::gajakesari::Gajakesari;
use super::kemadruma::Kemadruma;
use super::mahapurusha::{Bhadra, Hamsa, Malavya, Ruchaka, Shasha};
use super::neecha_bhanga::NeechaBhanga;
use super::raja::Raja;
use super::sunapha::Sunapha;
use super::vipreet_raja::VipreetRaja;
use super::{DetectedYoga, Yoga, YogaChartFacts};

/// Stable list of all yoga keys recognised by this engine version.
pub fn all_yoga_keys() -> Vec<&'static str> {
    vec![
        "gajakesari",
        "chandra_mangal",
        "budhaditya",
        "neecha_bhanga",
        "vipreet_raja",
        "dhana",
        "raja",
        "kemadruma",
        "sunapha",
        "anapha",
        "ruchaka",
        "bhadra",
        "hamsa",
        "malavya",
        "shasha",
    ]
}

/// Run every detector against the chart facts. Returns only the detected
/// yogas (None results are dropped). Order is stable across requests.
pub fn detect_yogas(facts: &YogaChartFacts) -> Vec<DetectedYoga> {
    let detectors: Vec<Box<dyn Yoga>> = vec![
        Box::new(Gajakesari),
        Box::new(ChandraMangal),
        Box::new(Budhaditya),
        Box::new(NeechaBhanga),
        Box::new(VipreetRaja),
        Box::new(Dhana),
        Box::new(Raja),
        Box::new(Kemadruma),
        Box::new(Sunapha),
        Box::new(Anapha),
        Box::new(Ruchaka),
        Box::new(Bhadra),
        Box::new(Hamsa),
        Box::new(Malavya),
        Box::new(Shasha),
    ];

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
