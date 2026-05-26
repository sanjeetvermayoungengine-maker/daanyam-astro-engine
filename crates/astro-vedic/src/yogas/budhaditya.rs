//! Budhaditya Yoga — Sun and Mercury in the same sign.

use super::dignity::same_sign;
use super::{DetectedYoga, Yoga, YogaChartFacts};

pub struct Budhaditya;

impl Yoga for Budhaditya {
    fn key(&self) -> &'static str {
        "budhaditya"
    }

    fn detect(&self, facts: &YogaChartFacts) -> Option<DetectedYoga> {
        if !same_sign("sun", "mercury", facts) {
            return None;
        }
        let house = super::dignity::planet_house(facts, "sun")?;
        Some(DetectedYoga {
            key: "budhaditya".to_string(),
            name: "Budhaditya Yoga".to_string(),
            planets_involved: vec!["Sun".to_string(), "Mercury".to_string()],
            houses_involved: vec![house],
            strength: 1.0,
            voice_line: "Sun and Mercury share a sign — the chart favours clear speech, learning, and practical intelligence.".to_string(),
        })
    }
}
