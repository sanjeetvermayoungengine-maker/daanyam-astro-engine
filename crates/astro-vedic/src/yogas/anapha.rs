//! Anapha Yoga — grahas (other than Sun) in the 12th house from Moon.

use super::dignity::{adjacent_house, has_planet_in_house};
use super::{DetectedYoga, Yoga, YogaChartFacts};

pub struct Anapha;

impl Yoga for Anapha {
    fn key(&self) -> &'static str {
        "anapha"
    }

    fn detect(&self, facts: &YogaChartFacts) -> Option<DetectedYoga> {
        let moon_house = facts.planet_houses.moon?;
        let twelfth = adjacent_house(moon_house, -1);
        if !has_planet_in_house(facts, twelfth, &["moon", "sun"]) {
            return None;
        }
        Some(DetectedYoga {
            key: "anapha".to_string(),
            name: "Anapha Yoga".to_string(),
            planets_involved: vec!["Moon".to_string()],
            houses_involved: vec![moon_house, twelfth],
            strength: 0.85,
            voice_line: "Planets sit in the 12th from Moon — comfort, polish, and social grace tend to follow when you receive well.".to_string(),
        })
    }
}
