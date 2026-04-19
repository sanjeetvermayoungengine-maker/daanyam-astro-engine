use astro_core::math::normalize_degrees;

use crate::{sidereal_division, Rashi};

use super::{rashi_from_index, rashi_index};

const NAVAMSA_SPAN_DEGREES: f64 = 30.0 / 9.0;

pub fn navamsa_sign(sidereal_longitude_deg: f64) -> Rashi {
    let longitude_deg = normalize_degrees(sidereal_longitude_deg);
    let rashi = sidereal_division(longitude_deg).rashi;
    let degrees_within_rashi = longitude_deg.rem_euclid(30.0);
    let navamsa_index = (degrees_within_rashi / NAVAMSA_SPAN_DEGREES).floor() as usize;
    let starting_rashi = navamsa_starting_rashi(rashi);

    rashi_from_index(rashi_index(starting_rashi) + navamsa_index)
}

fn navamsa_starting_rashi(rashi: Rashi) -> Rashi {
    match rashi {
        Rashi::Mesha | Rashi::Simha | Rashi::Dhanu => Rashi::Mesha,
        Rashi::Vrishabha | Rashi::Kanya | Rashi::Makara => Rashi::Makara,
        Rashi::Mithuna | Rashi::Tula | Rashi::Kumbha => Rashi::Tula,
        Rashi::Karka | Rashi::Vrischika | Rashi::Meena => Rashi::Karka,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navamsa_starts_follow_classical_elemental_sequences() {
        let cases = [
            (0.0, Rashi::Mesha),
            (30.0, Rashi::Makara),
            (60.0, Rashi::Tula),
            (90.0, Rashi::Karka),
            (120.0, Rashi::Mesha),
            (150.0, Rashi::Makara),
            (180.0, Rashi::Tula),
            (210.0, Rashi::Karka),
            (240.0, Rashi::Mesha),
            (270.0, Rashi::Makara),
            (300.0, Rashi::Tula),
            (330.0, Rashi::Karka),
        ];

        for (longitude_deg, expected) in cases {
            assert_eq!(navamsa_sign(longitude_deg), expected, "longitude {longitude_deg}");
        }
    }

    #[test]
    fn navamsa_advances_one_sign_per_pada_within_each_rashi() {
        let cases = [
            (1.0, Rashi::Mesha),
            (4.0, Rashi::Vrishabha),
            (7.0, Rashi::Mithuna),
            (11.0, Rashi::Karka),
            (17.0, Rashi::Kanya),
            (21.0, Rashi::Tula),
            (24.0, Rashi::Vrischika),
            (27.0, Rashi::Dhanu),
            (29.9, Rashi::Dhanu),
            (31.0, Rashi::Makara),
            (44.0, Rashi::Vrishabha),
            (62.0, Rashi::Tula),
            (94.0, Rashi::Simha),
            (214.0, Rashi::Simha),
            (351.0, Rashi::Makara),
        ];

        for (longitude_deg, expected) in cases {
            assert_eq!(navamsa_sign(longitude_deg), expected, "longitude {longitude_deg}");
        }
    }

    #[test]
    fn navamsa_normalizes_wrapped_longitudes() {
        assert_eq!(navamsa_sign(-1.0), navamsa_sign(359.0));
        assert_eq!(navamsa_sign(361.0), navamsa_sign(1.0));
    }
}
