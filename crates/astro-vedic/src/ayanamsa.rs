use astro_core::math::normalize_degrees;

use crate::SiderealDivision;

const J2000_JULIAN_DAY: f64 = 2_451_545.0;
const JULIAN_DAYS_PER_CENTURY: f64 = 36_525.0;
const LAHIRI_ZERO_EPOCH_JD_TT: f64 = 1_825_238.245_856_5;

pub const LAHIRI_ALGO_ID: &str = "lahiri_swe_zero_epoch_iau1976_v1";

pub fn lahiri_ayanamsa_deg(jd_tdb: f64) -> f64 {
    normalize_degrees(precession_in_longitude_arcsec(LAHIRI_ZERO_EPOCH_JD_TT, jd_tdb) / 3600.0)
}

pub fn sidereal_longitude_deg(tropical_longitude_deg: f64, jd_tdb: f64) -> f64 {
    normalize_degrees(tropical_longitude_deg - lahiri_ayanamsa_deg(jd_tdb))
}

pub fn moon_sidereal_division_from_tropical(
    tropical_longitude_deg: f64,
    jd_tdb: f64,
) -> SiderealDivision {
    crate::sidereal_division(sidereal_longitude_deg(tropical_longitude_deg, jd_tdb))
}

fn precession_in_longitude_arcsec(start_jd_tdb: f64, end_jd_tdb: f64) -> f64 {
    let t0 = (start_jd_tdb - J2000_JULIAN_DAY) / JULIAN_DAYS_PER_CENTURY;
    let t = (end_jd_tdb - start_jd_tdb) / JULIAN_DAYS_PER_CENTURY;

    t * (5029.0966 + t0 * (2.22226 - 0.000042 * t0) + t * (1.11113 - 0.000042 * t0 - 0.000006 * t))
}

#[cfg(test)]
mod tests {
    use super::*;

    const REFERENCE_POINTS: &[(f64, f64)] = &[
        (2_415_020.5, 22.466_941_870_607_517),
        (2_415_385.5, 22.480_895_901_091_554),
        (2_419_980.5, 22.656_569_039_237_62),
        (2_433_282.5, 23.165_177_801_921_462),
        (2_441_179.5, 23.467_162_375_289_42),
        (2_451_545.0, 23.863_587_230_322_29),
        (2_453_736.5, 23.947_406_736_142_7),
        (2_460_404.5, 24.202_455_071_994_372),
        (2_469_806.5, 24.562_112_783_996_08),
        (2_488_069.5, 25.260_850_096_206_017),
    ];

    #[test]
    fn lahiri_ayanamsa_is_positive_at_j2000() {
        let ayanamsa = lahiri_ayanamsa_deg(J2000_JULIAN_DAY);
        assert!(ayanamsa > 20.0);
        assert!(ayanamsa < 30.0);
    }

    #[test]
    fn lahiri_algo_id_is_stable() {
        assert_eq!(LAHIRI_ALGO_ID, "lahiri_swe_zero_epoch_iau1976_v1");
    }

    #[test]
    fn reference_points_span_centuries() {
        for &(jd_tdb, expected_deg) in REFERENCE_POINTS {
            let actual = lahiri_ayanamsa_deg(jd_tdb);
            assert!(
                (actual - expected_deg).abs() < 1.0e-12,
                "jd_tdb={jd_tdb} expected={expected_deg} actual={actual}"
            );
        }
    }
}
