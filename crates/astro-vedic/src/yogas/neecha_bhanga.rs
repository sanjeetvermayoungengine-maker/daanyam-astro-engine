//! Neecha Bhanga Raja Yoga — debilitation cancelled when the debilitated planet's
//! dispositor sits in a kendra from lagna.

use super::dignity::{
    graha_rashi, house_lord_for, is_debilitated, is_kendra_house, planet_house,
};
use super::{DetectedYoga, Yoga, YogaChartFacts};

const GRAHAS: [&str; 7] =
    ["sun", "moon", "mars", "mercury", "jupiter", "venus", "saturn"];

pub struct NeechaBhanga;

impl Yoga for NeechaBhanga {
    fn key(&self) -> &'static str {
        "neecha_bhanga"
    }

    fn detect(&self, facts: &YogaChartFacts) -> Option<DetectedYoga> {
        for body in GRAHAS {
            let rashi = graha_rashi(facts, body)?;
            if !is_debilitated(body, rashi) {
                continue;
            }
            let deb_house = planet_house(facts, body)?;
            let lord = house_lord_for(deb_house, facts.lagna_rashi);
            let lord_house = planet_house(facts, lord)?;
            if is_kendra_house(lord_house) {
                return Some(DetectedYoga {
                    key: "neecha_bhanga".to_string(),
                    name: "Neecha Bhanga Raja Yoga".to_string(),
                    planets_involved: vec![format_name(body), format_name(lord)],
                    houses_involved: vec![deb_house, lord_house],
                    strength: 0.85,
                    voice_line: "A debilitated graha finds its lord in kendra — weakness turns into earned resilience.".to_string(),
                });
            }
        }
        None
    }
}

fn format_name(body: &str) -> String {
    match body {
        "sun" => "Sun",
        "moon" => "Moon",
        "mars" => "Mars",
        "mercury" => "Mercury",
        "jupiter" => "Jupiter",
        "venus" => "Venus",
        "saturn" => "Saturn",
        _ => body,
    }
    .to_string()
}
