use astro_core::math::normalize_degrees;

use crate::{sidereal_division, Rashi};

use super::{rashi_from_index, rashi_index};

const DREKKANA_SPAN_DEGREES: f64 = 10.0;

pub fn drekkana_sign(sidereal_longitude_deg: f64) -> Rashi {
    let longitude_deg = normalize_degrees(sidereal_longitude_deg);
    let rashi = sidereal_division(longitude_deg).rashi;
    let degrees_within_rashi = longitude_deg.rem_euclid(30.0);
    let drekkana_index = (degrees_within_rashi / DREKKANA_SPAN_DEGREES).floor() as usize;
    let sign_offset = match drekkana_index {
        0 => 0,
        1 => 4,
        2 => 8,
        _ => unreachable!("drekkana index must be within a rashi"),
    };

    rashi_from_index(rashi_index(rashi) + sign_offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drekkana_starts_from_the_rashi_itself() {
        let cases = [
            (0.0, Rashi::Mesha),
            (30.0, Rashi::Vrishabha),
            (60.0, Rashi::Mithuna),
            (90.0, Rashi::Karka),
            (120.0, Rashi::Simha),
            (150.0, Rashi::Kanya),
            (180.0, Rashi::Tula),
            (210.0, Rashi::Vrischika),
            (240.0, Rashi::Dhanu),
            (270.0, Rashi::Makara),
            (300.0, Rashi::Kumbha),
            (330.0, Rashi::Meena),
        ];

        for (longitude_deg, expected) in cases {
            assert_eq!(drekkana_sign(longitude_deg), expected, "longitude {longitude_deg}");
        }
    }

    #[test]
    fn drekkana_uses_first_fifth_and_ninth_signs() {
        let cases = [
            (1.0, Rashi::Mesha),
            (11.0, Rashi::Simha),
            (21.0, Rashi::Dhanu),
            (31.0, Rashi::Vrishabha),
            (41.0, Rashi::Kanya),
            (51.0, Rashi::Makara),
            (91.0, Rashi::Karka),
            (101.0, Rashi::Vrischika),
            (111.0, Rashi::Meena),
            (181.0, Rashi::Tula),
            (191.0, Rashi::Kumbha),
            (201.0, Rashi::Mithuna),
            (331.0, Rashi::Meena),
            (341.0, Rashi::Karka),
            (351.0, Rashi::Vrischika),
        ];

        for (longitude_deg, expected) in cases {
            assert_eq!(drekkana_sign(longitude_deg), expected, "longitude {longitude_deg}");
        }
    }

    #[test]
    fn drekkana_normalizes_wrapped_longitudes() {
        assert_eq!(drekkana_sign(-1.0), drekkana_sign(359.0));
        assert_eq!(drekkana_sign(361.0), drekkana_sign(1.0));
    }
}
