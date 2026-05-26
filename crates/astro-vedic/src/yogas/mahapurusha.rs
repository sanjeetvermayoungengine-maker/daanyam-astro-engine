//! Pancha Mahapurusha yogas — Mars, Mercury, Jupiter, Venus, Saturn in own or
//! exalted sign AND in a kendra from lagna.

use super::dignity::{graha_rashi, is_exalted, is_kendra_house, is_own_sign, planet_house};
use super::{DetectedYoga, Yoga, YogaChartFacts};

macro_rules! mahapurusha_yoga {
    ($struct_name:ident, $key:literal, $body:literal, $display:literal, $name:literal, $voice:literal) => {
        pub struct $struct_name;

        impl Yoga for $struct_name {
            fn key(&self) -> &'static str {
                $key
            }

            fn detect(&self, facts: &YogaChartFacts) -> Option<DetectedYoga> {
                let rashi = graha_rashi(facts, $body)?;
                if !(is_own_sign($body, rashi) || is_exalted($body, rashi)) {
                    return None;
                }
                let house = planet_house(facts, $body)?;
                if !is_kendra_house(house) {
                    return None;
                }
                Some(DetectedYoga {
                    key: $key.to_string(),
                    name: $name.to_string(),
                    planets_involved: vec![$display.to_string()],
                    houses_involved: vec![house],
                    strength: if is_exalted($body, rashi) { 1.0 } else { 0.9 },
                    voice_line: $voice.to_string(),
                })
            }
        }
    };
}

mahapurusha_yoga!(
    Ruchaka,
    "ruchaka",
    "mars",
    "Mars",
    "Ruchaka Yoga",
    "Mars in strength in kendra — courage, command, and physical vitality are signature themes."
);
mahapurusha_yoga!(
    Bhadra,
    "bhadra",
    "mercury",
    "Mercury",
    "Bhadra Yoga",
    "Mercury in strength in kendra — intellect, speech, and skilled work carry unusual weight."
);
mahapurusha_yoga!(
    Hamsa,
    "hamsa",
    "jupiter",
    "Jupiter",
    "Hamsa Yoga",
    "Jupiter in strength in kendra — wisdom, ethics, and generous leadership are highlighted."
);
mahapurusha_yoga!(
    Malavya,
    "malavya",
    "venus",
    "Venus",
    "Malavya Yoga",
    "Venus in strength in kendra — refinement, relationships, and aesthetic sense are pronounced."
);
mahapurusha_yoga!(
    Shasha,
    "shasha",
    "saturn",
    "Saturn",
    "Shasha Yoga",
    "Saturn in strength in kendra — discipline, endurance, and authority earned slowly are central."
);
