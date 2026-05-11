//! Gajakesari Yoga
//!
//! Classical: Moon and Jupiter in mutual kendra (1, 4, 7, or 10 from each
//! other). Bestows fame, intelligence, eloquence, longevity per Phaladeepika.
//!
//! Modern strength considerations (not all gating; we soften the voice via
//! `strength` instead of failing the detection):
//!   - Jupiter retrograde reduces strength slightly.
//!   - Moon waning (less than 90° from Sun on the dark side) reduces strength.
//!     (Phase 4 first iteration ignores this; Phase 4.1 may grade it in.)

use super::{is_mutual_kendra, DetectedYoga, Yoga, YogaChartFacts};

pub struct Gajakesari;

impl Yoga for Gajakesari {
    fn key(&self) -> &'static str {
        "gajakesari"
    }

    fn detect(&self, facts: &YogaChartFacts) -> Option<DetectedYoga> {
        let moon_house = facts.planet_houses.moon?;
        let jupiter_house = facts.planet_houses.jupiter?;

        if !is_mutual_kendra(moon_house, jupiter_house) {
            return None;
        }

        Some(DetectedYoga {
            key: "gajakesari".to_string(),
            name: "Gajakesari Yoga".to_string(),
            planets_involved: vec!["Moon".to_string(), "Jupiter".to_string()],
            houses_involved: vec![moon_house, jupiter_house],
            strength: 1.0,
            voice_line: "Moon and Jupiter sit in mutual kendra — the chart asks you to speak from a steady inner ground; the world tends to listen.".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::{PlanetHouses, PlanetLongitudes, YogaChartFacts};
    use super::*;
    use crate::Rashi;

    fn facts_with_moon_jupiter(moon_house: u8, jupiter_house: u8) -> YogaChartFacts {
        YogaChartFacts {
            lagna_rashi: Rashi::Mesha,
            planet_longitudes: PlanetLongitudes::default(),
            planet_houses: PlanetHouses {
                moon: Some(moon_house),
                jupiter: Some(jupiter_house),
                ..Default::default()
            },
        }
    }

    #[test]
    fn detects_when_moon_and_jupiter_are_in_kendra() {
        // Moon in 1st, Jupiter in 10th — classic Gajakesari
        let facts = facts_with_moon_jupiter(1, 10);
        let yoga = Gajakesari.detect(&facts).expect("must detect");
        assert_eq!(yoga.key, "gajakesari");
        assert_eq!(yoga.houses_involved, vec![1, 10]);
    }

    #[test]
    fn rejects_when_outside_kendra() {
        // Moon in 1st, Jupiter in 2nd — not a kendra distance
        let facts = facts_with_moon_jupiter(1, 2);
        assert!(Gajakesari.detect(&facts).is_none());
    }

    #[test]
    fn rejects_when_moon_or_jupiter_missing() {
        let mut facts = facts_with_moon_jupiter(1, 1);
        facts.planet_houses.jupiter = None;
        assert!(Gajakesari.detect(&facts).is_none());
    }
}
