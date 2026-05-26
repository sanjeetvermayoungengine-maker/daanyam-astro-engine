//! Raja Yoga — a kendra lord and a trikona lord in mutual kendra.

use super::dignity::{
    house_lord_for, is_kendra_house, is_trikona_house, mutual_kendra_houses, planet_house,
};
use super::{DetectedYoga, Yoga, YogaChartFacts};

pub struct Raja;

impl Yoga for Raja {
    fn key(&self) -> &'static str {
        "raja"
    }

    fn detect(&self, facts: &YogaChartFacts) -> Option<DetectedYoga> {
        for kendra in [1u8, 4, 7, 10] {
            let k_lord = house_lord_for(kendra, facts.lagna_rashi);
            let k_house = planet_house(facts, k_lord)?;
            if !is_kendra_house(k_house) {
                continue;
            }
            for trikona in [1u8, 5, 9] {
                let t_lord = house_lord_for(trikona, facts.lagna_rashi);
                if k_lord == t_lord {
                    continue;
                }
                let t_house = planet_house(facts, t_lord)?;
                if !is_trikona_house(t_house) {
                    continue;
                }
                if mutual_kendra_houses(k_house, t_house) {
                    return Some(DetectedYoga {
                        key: "raja".to_string(),
                        name: "Raja Yoga".to_string(),
                        planets_involved: vec![display(k_lord), display(t_lord)],
                        houses_involved: vec![k_house, t_house],
                        strength: 1.0,
                        voice_line: "Kendra and trikona lords unite — authority and dharma reinforce each other in this chart.".to_string(),
                    });
                }
            }
        }
        None
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
