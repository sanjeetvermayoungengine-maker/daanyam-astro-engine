//! Chandra-Mangal Yoga
//!
//! Classical: Moon and Mars conjunct in the same rashi. Associated with
//! financial mobility, entrepreneurial drive — but also temperamental
//! restlessness when not directed.

use super::{DetectedYoga, Yoga, YogaChartFacts};

pub struct ChandraMangal;

impl Yoga for ChandraMangal {
    fn key(&self) -> &'static str {
        "chandra_mangal"
    }

    fn detect(&self, facts: &YogaChartFacts) -> Option<DetectedYoga> {
        let moon_house = facts.planet_houses.moon?;
        let mars_house = facts.planet_houses.mars?;
        let moon_lon = facts.planet_longitudes.moon?;
        let mars_lon = facts.planet_longitudes.mars?;

        if moon_house != mars_house {
            return None;
        }

        // Same-rashi conjunction. Mark strength higher when the angular
        // separation is tight (<= 10°), lower when wider.
        let sep = (moon_lon - mars_lon).abs().min(360.0 - (moon_lon - mars_lon).abs());
        let strength = if sep <= 10.0 {
            1.0
        } else if sep <= 20.0 {
            0.75
        } else {
            0.5
        };

        Some(DetectedYoga {
            key: "chandra_mangal".to_string(),
            name: "Chandra-Mangal Yoga".to_string(),
            planets_involved: vec!["Moon".to_string(), "Mars".to_string()],
            houses_involved: vec![moon_house],
            strength,
            voice_line: "Moon and Mars meet in the same sign — the chart carries entrepreneurial fire; spend the heat on what is worth the burn.".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::{PlanetHouses, PlanetLongitudes, YogaChartFacts};
    use super::*;
    use crate::Rashi;

    fn facts_for_same_house(
        moon_house: u8,
        mars_house: u8,
        moon_lon: f64,
        mars_lon: f64,
    ) -> YogaChartFacts {
        YogaChartFacts {
            lagna_rashi: Rashi::Mesha,
            planet_longitudes: PlanetLongitudes {
                moon: Some(moon_lon),
                mars: Some(mars_lon),
                ..Default::default()
            },
            planet_houses: PlanetHouses {
                moon: Some(moon_house),
                mars: Some(mars_house),
                ..Default::default()
            },
        }
    }

    #[test]
    fn detects_tight_conjunction_at_full_strength() {
        let facts = facts_for_same_house(5, 5, 100.0, 103.0);
        let yoga = ChandraMangal.detect(&facts).expect("must detect");
        assert!((yoga.strength - 1.0).abs() < 1e-9);
    }

    #[test]
    fn detects_wider_conjunction_at_lower_strength() {
        let facts = facts_for_same_house(5, 5, 100.0, 115.0);
        let yoga = ChandraMangal.detect(&facts).expect("must detect");
        assert!((yoga.strength - 0.75).abs() < 1e-9);
    }

    #[test]
    fn rejects_when_planets_in_different_houses() {
        let facts = facts_for_same_house(5, 6, 100.0, 130.0);
        assert!(ChandraMangal.detect(&facts).is_none());
    }
}
