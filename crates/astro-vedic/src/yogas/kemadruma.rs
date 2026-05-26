//! Kemadruma Yoga — Moon without grahas in the 2nd or 12th houses from it.

use super::dignity::{adjacent_house, has_planet_in_house};
use super::{DetectedYoga, Yoga, YogaChartFacts};

pub struct Kemadruma;

impl Yoga for Kemadruma {
    fn key(&self) -> &'static str {
        "kemadruma"
    }

    fn detect(&self, facts: &YogaChartFacts) -> Option<DetectedYoga> {
        let moon_house = facts.planet_houses.moon?;
        let second = adjacent_house(moon_house, 1);
        let twelfth = adjacent_house(moon_house, -1);
        let isolated = !has_planet_in_house(facts, second, &["moon"])
            && !has_planet_in_house(facts, twelfth, &["moon"]);
        if !isolated {
            return None;
        }
        Some(DetectedYoga {
            key: "kemadruma".to_string(),
            name: "Kemadruma Yoga".to_string(),
            planets_involved: vec!["Moon".to_string()],
            houses_involved: vec![moon_house, second, twelfth],
            strength: 0.75,
            voice_line: "Moon stands without neighbours — inner weather may swing until you anchor daily rhythm and trusted company.".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::{PlanetHouses, PlanetLongitudes, YogaChartFacts};
    use super::*;
    use crate::Rashi;

    #[test]
    fn detects_isolated_moon() {
        let facts = YogaChartFacts {
            lagna_rashi: Rashi::Mesha,
            planet_longitudes: PlanetLongitudes::default(),
            planet_houses: PlanetHouses {
                moon: Some(5),
                sun: Some(1),
                ..Default::default()
            },
        };
        assert!(Kemadruma.detect(&facts).is_some());
    }
}
