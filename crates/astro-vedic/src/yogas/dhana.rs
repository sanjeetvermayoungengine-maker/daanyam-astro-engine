//! Dhana Yoga — lords of the 2nd and 11th houses in mutual kendra.

use super::dignity::{house_lord_for, mutual_kendra_houses, planet_house};
use super::{DetectedYoga, Yoga, YogaChartFacts};

pub struct Dhana;

impl Yoga for Dhana {
    fn key(&self) -> &'static str {
        "dhana"
    }

    fn detect(&self, facts: &YogaChartFacts) -> Option<DetectedYoga> {
        let lord2 = house_lord_for(2, facts.lagna_rashi);
        let lord11 = house_lord_for(11, facts.lagna_rashi);
        let h2 = planet_house(facts, lord2)?;
        let h11 = planet_house(facts, lord11)?;
        if !mutual_kendra_houses(h2, h11) {
            return None;
        }
        Some(DetectedYoga {
            key: "dhana".to_string(),
            name: "Dhana Yoga".to_string(),
            planets_involved: vec![display(lord2), display(lord11)],
            houses_involved: vec![h2, h11],
            strength: 0.9,
            voice_line: "Lords of gain and income meet in kendra — the chart supports steady accumulation when effort is consistent.".to_string(),
        })
    }
}

fn display(lord: &str) -> String {
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
