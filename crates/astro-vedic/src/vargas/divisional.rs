use astro_core::math::normalize_degrees;

use crate::{sidereal_division, Rashi};

use super::{navamsa_sign, rashi_from_index, rashi_index};

fn sign_index(longitude_deg: f64) -> usize {
    rashi_index(sidereal_division(normalize_degrees(longitude_deg)).rashi)
}

fn deg_in_sign(longitude_deg: f64) -> f64 {
    normalize_degrees(longitude_deg).rem_euclid(30.0)
}

/// D10 Dashamsha — 3° parts. Odd signs from same sign; even from 9th sign.
pub fn dashamsha_sign(sidereal_longitude_deg: f64) -> Rashi {
    let sign = sign_index(sidereal_longitude_deg);
    let division = (deg_in_sign(sidereal_longitude_deg) / 3.0).floor() as usize;
    let is_odd = sign % 2 == 0;
    let start = if is_odd { sign } else { (sign + 8) % 12 };
    rashi_from_index((start + division) % 12)
}

/// D12 Dwadashamsha — 2°30′ parts, counting from the rashi itself.
pub fn dwadashamsha_sign(sidereal_longitude_deg: f64) -> Rashi {
    let sign = sign_index(sidereal_longitude_deg);
    let division = (deg_in_sign(sidereal_longitude_deg) / 2.5).floor() as usize;
    rashi_from_index((sign + division) % 12)
}

/// Varga lagna from D1 ascendant longitude (same rules as graha vargas).
pub fn varga_lagna_rashi(ascendant_sidereal_deg: f64, varga: &str) -> Option<Rashi> {
    match varga {
        "D9" => Some(navamsa_sign(ascendant_sidereal_deg)),
        "D10" => Some(dashamsha_sign(ascendant_sidereal_deg)),
        "D12" => Some(dwadashamsha_sign(ascendant_sidereal_deg)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patna_fixture_varga_lagnas() {
        // Patna 1999-02-13 22:10 IST — lagna ~181.656° sidereal → Tula D9/D10/D12 lagna
        let lagna = 181.656371;
        assert_eq!(navamsa_sign(lagna), Rashi::Tula);
        assert_eq!(dashamsha_sign(lagna), Rashi::Tula);
        assert_eq!(dwadashamsha_sign(lagna), Rashi::Tula);
    }
}
