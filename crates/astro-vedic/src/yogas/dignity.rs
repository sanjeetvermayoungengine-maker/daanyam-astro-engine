//! Sign dignity helpers shared by multiple yoga detectors.

use crate::Rashi;

use super::{house_distance, YogaChartFacts};

pub(crate) fn graha_rashi(facts: &YogaChartFacts, body: &str) -> Option<Rashi> {
    let lon = match body {
        "sun" => facts.planet_longitudes.sun?,
        "moon" => facts.planet_longitudes.moon?,
        "mars" => facts.planet_longitudes.mars?,
        "mercury" => facts.planet_longitudes.mercury?,
        "jupiter" => facts.planet_longitudes.jupiter?,
        "venus" => facts.planet_longitudes.venus?,
        "saturn" => facts.planet_longitudes.saturn?,
        _ => return None,
    };
    Some(super::super::sidereal::sidereal_division(lon).rashi)
}

pub(crate) fn is_own_sign(body: &str, rashi: Rashi) -> bool {
    matches!(
        (body, rashi),
        ("sun", Rashi::Simha)
            | ("moon", Rashi::Karka)
            | ("mars", Rashi::Mesha)
            | ("mars", Rashi::Vrischika)
            | ("mercury", Rashi::Mithuna)
            | ("mercury", Rashi::Kanya)
            | ("jupiter", Rashi::Dhanu)
            | ("jupiter", Rashi::Meena)
            | ("venus", Rashi::Vrishabha)
            | ("venus", Rashi::Tula)
            | ("saturn", Rashi::Makara)
            | ("saturn", Rashi::Kumbha)
    )
}

pub(crate) fn is_exalted(body: &str, rashi: Rashi) -> bool {
    matches!(
        (body, rashi),
        ("sun", Rashi::Mesha)
            | ("moon", Rashi::Vrishabha)
            | ("mars", Rashi::Makara)
            | ("mercury", Rashi::Kanya)
            | ("jupiter", Rashi::Karka)
            | ("venus", Rashi::Meena)
            | ("saturn", Rashi::Tula)
    )
}

pub(crate) fn is_debilitated(body: &str, rashi: Rashi) -> bool {
    matches!(
        (body, rashi),
        ("sun", Rashi::Tula)
            | ("moon", Rashi::Vrischika)
            | ("mars", Rashi::Karka)
            | ("mercury", Rashi::Meena)
            | ("jupiter", Rashi::Makara)
            | ("venus", Rashi::Kanya)
            | ("saturn", Rashi::Mesha)
    )
}

pub(crate) fn is_kendra_house(house: u8) -> bool {
    matches!(house, 1 | 4 | 7 | 10)
}

pub(crate) fn is_trikona_house(house: u8) -> bool {
    matches!(house, 1 | 5 | 9)
}

pub(crate) fn is_dusthana_house(house: u8) -> bool {
    matches!(house, 6 | 8 | 12)
}

fn rashi_for_house(house: u8, lagna: Rashi) -> Rashi {
    let lagna_idx = super::rashi_index(lagna) as usize;
    let sign_idx = (lagna_idx + (house as usize) - 1) % 12;
    match sign_idx {
        0 => Rashi::Mesha,
        1 => Rashi::Vrishabha,
        2 => Rashi::Mithuna,
        3 => Rashi::Karka,
        4 => Rashi::Simha,
        5 => Rashi::Kanya,
        6 => Rashi::Tula,
        7 => Rashi::Vrischika,
        8 => Rashi::Dhanu,
        9 => Rashi::Makara,
        10 => Rashi::Kumbha,
        _ => Rashi::Meena,
    }
}

pub(crate) fn house_lord_for(house: u8, lagna: Rashi) -> &'static str {
    let rashi = rashi_for_house(house, lagna);
    match rashi {
        Rashi::Mesha | Rashi::Vrischika => "mars",
        Rashi::Vrishabha | Rashi::Tula => "venus",
        Rashi::Mithuna | Rashi::Kanya => "mercury",
        Rashi::Karka => "moon",
        Rashi::Simha => "sun",
        Rashi::Dhanu | Rashi::Meena => "jupiter",
        Rashi::Makara | Rashi::Kumbha => "saturn",
    }
}

pub(crate) fn planet_house(facts: &YogaChartFacts, body: &str) -> Option<u8> {
    match body {
        "sun" => facts.planet_houses.sun,
        "moon" => facts.planet_houses.moon,
        "mars" => facts.planet_houses.mars,
        "mercury" => facts.planet_houses.mercury,
        "jupiter" => facts.planet_houses.jupiter,
        "venus" => facts.planet_houses.venus,
        "saturn" => facts.planet_houses.saturn,
        _ => None,
    }
}

pub(crate) fn adjacent_house(from: u8, offset: i16) -> u8 {
    ((from as i16 - 1 + offset).rem_euclid(12) + 1) as u8
}

pub(crate) fn has_planet_in_house(facts: &YogaChartFacts, house: u8, exclude: &[&str]) -> bool {
    let bodies = [
        ("sun", facts.planet_houses.sun),
        ("moon", facts.planet_houses.moon),
        ("mars", facts.planet_houses.mars),
        ("mercury", facts.planet_houses.mercury),
        ("jupiter", facts.planet_houses.jupiter),
        ("venus", facts.planet_houses.venus),
        ("saturn", facts.planet_houses.saturn),
    ];
    bodies.iter().any(|(name, h)| {
        if exclude.contains(name) {
            return false;
        }
        *h == Some(house)
    })
}

pub(crate) fn same_sign(a_body: &str, b_body: &str, facts: &YogaChartFacts) -> bool {
    graha_rashi(facts, a_body) == graha_rashi(facts, b_body)
        && graha_rashi(facts, a_body).is_some()
}

pub(crate) fn mutual_kendra_houses(a: u8, b: u8) -> bool {
    let d = house_distance(a, b);
    matches!(d, 1 | 4 | 7 | 10)
}
