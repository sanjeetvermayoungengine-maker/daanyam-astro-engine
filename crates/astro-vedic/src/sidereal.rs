use serde::{Deserialize, Serialize};

use astro_core::math::normalize_degrees;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Rashi {
    Mesha,
    Vrishabha,
    Mithuna,
    Karka,
    Simha,
    Kanya,
    Tula,
    Vrischika,
    Dhanu,
    Makara,
    Kumbha,
    Meena,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Nakshatra {
    Ashwini,
    Bharani,
    Krittika,
    Rohini,
    Mrigashira,
    Ardra,
    Punarvasu,
    Pushya,
    Ashlesha,
    Magha,
    PurvaPhalguni,
    UttaraPhalguni,
    Hasta,
    Chitra,
    Swati,
    Vishakha,
    Anuradha,
    Jyeshtha,
    Mula,
    PurvaAshadha,
    UttaraAshadha,
    Shravana,
    Dhanishta,
    Shatabhisha,
    PurvaBhadrapada,
    UttaraBhadrapada,
    Revati,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Pada(pub u8);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Lagna {
    pub rashi: Rashi,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct LagnaPosition {
    pub rashi: Rashi,
    pub sidereal_longitude_deg: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct SiderealDivision {
    pub rashi: Rashi,
    pub nakshatra: Nakshatra,
    pub pada: Pada,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct WholeSignHouse {
    pub house: u8,
    pub rashi: Rashi,
    pub cusp_sidereal_longitude_deg: f64,
}

const RASHIS: [Rashi; 12] = [
    Rashi::Mesha,
    Rashi::Vrishabha,
    Rashi::Mithuna,
    Rashi::Karka,
    Rashi::Simha,
    Rashi::Kanya,
    Rashi::Tula,
    Rashi::Vrischika,
    Rashi::Dhanu,
    Rashi::Makara,
    Rashi::Kumbha,
    Rashi::Meena,
];

const NAKSHATRAS: [Nakshatra; 27] = [
    Nakshatra::Ashwini,
    Nakshatra::Bharani,
    Nakshatra::Krittika,
    Nakshatra::Rohini,
    Nakshatra::Mrigashira,
    Nakshatra::Ardra,
    Nakshatra::Punarvasu,
    Nakshatra::Pushya,
    Nakshatra::Ashlesha,
    Nakshatra::Magha,
    Nakshatra::PurvaPhalguni,
    Nakshatra::UttaraPhalguni,
    Nakshatra::Hasta,
    Nakshatra::Chitra,
    Nakshatra::Swati,
    Nakshatra::Vishakha,
    Nakshatra::Anuradha,
    Nakshatra::Jyeshtha,
    Nakshatra::Mula,
    Nakshatra::PurvaAshadha,
    Nakshatra::UttaraAshadha,
    Nakshatra::Shravana,
    Nakshatra::Dhanishta,
    Nakshatra::Shatabhisha,
    Nakshatra::PurvaBhadrapada,
    Nakshatra::UttaraBhadrapada,
    Nakshatra::Revati,
];

pub fn sidereal_division(longitude_deg: f64) -> SiderealDivision {
    let lon = normalize_degrees(longitude_deg);
    let rashi_index = (lon / 30.0).floor() as usize;
    let nakshatra_width = 360.0 / 27.0;
    let nakshatra_index = (lon / nakshatra_width).floor() as usize;
    let pada_width = nakshatra_width / 4.0;
    let pada = ((lon.rem_euclid(nakshatra_width)) / pada_width).floor() as u8 + 1;

    SiderealDivision {
        rashi: RASHIS[rashi_index],
        nakshatra: NAKSHATRAS[nakshatra_index],
        pada: Pada(pada),
    }
}

pub fn lagna_from_sidereal_longitude(longitude_deg: f64) -> Lagna {
    Lagna { rashi: sidereal_division(longitude_deg).rashi }
}

pub fn lagna_position_from_sidereal_longitude(longitude_deg: f64) -> LagnaPosition {
    let longitude_deg = normalize_degrees(longitude_deg);
    LagnaPosition {
        rashi: sidereal_division(longitude_deg).rashi,
        sidereal_longitude_deg: longitude_deg,
    }
}

pub fn whole_sign_houses_from_sidereal_ascendant(longitude_deg: f64) -> [WholeSignHouse; 12] {
    let lagna = lagna_position_from_sidereal_longitude(longitude_deg);
    let start_index = rashi_index(lagna.rashi);

    std::array::from_fn(|offset| {
        let rashi_index = (start_index + offset) % RASHIS.len();
        WholeSignHouse {
            house: u8::try_from(offset + 1).expect("whole sign house index fits in u8"),
            rashi: RASHIS[rashi_index],
            cusp_sidereal_longitude_deg: (rashi_index as f64) * 30.0,
        }
    })
}

fn rashi_index(rashi: Rashi) -> usize {
    RASHIS
        .iter()
        .position(|candidate| *candidate == rashi)
        .expect("rashi table must contain every enum variant")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_rashi_nakshatra_and_pada() {
        let division = sidereal_division(15.0);
        assert_eq!(division.rashi, Rashi::Mesha);
        assert_eq!(division.nakshatra, Nakshatra::Bharani);
        assert_eq!(division.pada, Pada(1));
    }

    #[test]
    fn lagna_is_modeled_separately_from_graha_identity() {
        let lagna = lagna_from_sidereal_longitude(210.0);
        assert_eq!(lagna.rashi, Rashi::Vrischika);
    }

    #[test]
    fn whole_sign_houses_follow_lagna_rashi_order() {
        let lagna = lagna_position_from_sidereal_longitude(47.5);
        let houses = whole_sign_houses_from_sidereal_ascendant(lagna.sidereal_longitude_deg);

        assert_eq!(lagna.rashi, Rashi::Vrishabha);
        assert_eq!(houses[0].house, 1);
        assert_eq!(houses[0].rashi, Rashi::Vrishabha);
        assert_eq!(houses[0].cusp_sidereal_longitude_deg, 30.0);
        assert_eq!(houses[1].rashi, Rashi::Mithuna);
        assert_eq!(houses[11].rashi, Rashi::Mesha);
        assert_eq!(houses[11].cusp_sidereal_longitude_deg, 0.0);
    }
}
