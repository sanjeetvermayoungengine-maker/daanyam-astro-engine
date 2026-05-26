//! Sunapha Yoga — grahas (other than Sun) in the 2nd house from Moon.

use super::dignity::{adjacent_house, has_planet_in_house};
use super::{DetectedYoga, Yoga, YogaChartFacts};

pub struct Sunapha;

impl Yoga for Sunapha {
    fn key(&self) -> &'static str {
        "sunapha"
    }

    fn detect(&self, facts: &YogaChartFacts) -> Option<DetectedYoga> {
        let moon_house = facts.planet_houses.moon?;
        let second = adjacent_house(moon_house, 1);
        if !has_planet_in_house(facts, second, &["moon", "sun"]) {
            return None;
        }
        Some(DetectedYoga {
            key: "sunapha".to_string(),
            name: "Sunapha Yoga".to_string(),
            planets_involved: vec!["Moon".to_string()],
            houses_involved: vec![moon_house, second],
            strength: 0.85,
            voice_line: "Planets flank Moon from the 2nd — self-made prosperity is emphasised when you act on your own initiative.".to_string(),
        })
    }
}
