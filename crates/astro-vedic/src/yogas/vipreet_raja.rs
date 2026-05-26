//! Vipreet Raja Yoga — lords of dusthana houses (6, 8, 12) placed in dusthana.

use super::dignity::{house_lord_for, is_dusthana_house, planet_house};
use super::{DetectedYoga, Yoga, YogaChartFacts};

pub struct VipreetRaja;

impl Yoga for VipreetRaja {
    fn key(&self) -> &'static str {
        "vipreet_raja"
    }

    fn detect(&self, facts: &YogaChartFacts) -> Option<DetectedYoga> {
        let mut matched = Vec::new();
        for house in [6u8, 8, 12] {
            let lord = house_lord_for(house, facts.lagna_rashi);
            let lord_house = planet_house(facts, lord)?;
            if is_dusthana_house(lord_house) {
                matched.push((house, lord, lord_house));
            }
        }
        if matched.is_empty() {
            return None;
        }
        let (dusthana, lord, lord_house) = matched[0];
        Some(DetectedYoga {
            key: "vipreet_raja".to_string(),
            name: "Vipreet Raja Yoga".to_string(),
            planets_involved: vec![lord_display(lord)],
            houses_involved: vec![dusthana, lord_house],
            strength: if matched.len() >= 2 { 1.0 } else { 0.8 },
            voice_line: "A dusthana lord sits in adversity and turns it into quiet advantage — gains through reversal.".to_string(),
        })
    }
}

fn lord_display(lord: &str) -> String {
    match lord {
        "sun" => "Sun",
        "moon" => "Moon",
        "mars" => "Mars",
        "mercury" => "Mercury",
        "jupiter" => "Jupiter",
        "venus" => "Venus",
        "saturn" => "Saturn",
        _ => lord,
    }
    .to_string()
}
