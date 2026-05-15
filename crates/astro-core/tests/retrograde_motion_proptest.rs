//! Property: retrograde flag matches longitude speed sign (DE440, 2020–2030).

use astro_core::{
    motion::{longitude_motion, retrograde_from_speed, RETROGRADE_SPEED_EPSILON},
    time::julian_day,
    CelestialBody, De440Backend, EngineConfig,
};
use chrono::{TimeZone, Utc};
use proptest::prelude::*;

#[path = "../../../tests/support/de440_kernel.rs"]
mod de440_kernel;

const JD_2020: f64 = 2_458_866.5;
const JD_2030_END: f64 = 2_493_019.5;

fn body_strategy() -> impl Strategy<Value = CelestialBody> {
    prop_oneof![
        Just(CelestialBody::Mercury),
        Just(CelestialBody::Venus),
        Just(CelestialBody::Mars),
        Just(CelestialBody::Jupiter),
        Just(CelestialBody::Saturn),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 64, ..ProptestConfig::default() })]

    #[test]
    fn retrograde_iff_speed_negative_within_epsilon(
        body in body_strategy(),
        jd_offset in 0.0f64..(JD_2030_END - JD_2020),
    ) {
        let Some(path) = de440_kernel::de440_kernel_path() else {
            return Ok(());
        };
        let backend = De440Backend::from_path(&path).expect("DE440 backend must load");
        let config = EngineConfig::default();
        let jd = JD_2020 + jd_offset;

        let motion = longitude_motion(&backend, body, jd, &config)?;
        let speed = motion.longitude_speed_deg_per_day;

        if speed.abs() > RETROGRADE_SPEED_EPSILON {
            prop_assert_eq!(motion.retrograde, retrograde_from_speed(speed));
            prop_assert_eq!(motion.retrograde, speed < 0.0);
        } else {
            prop_assert!(!motion.retrograde);
        }
    }
}

#[test]
fn proptest_sample_includes_known_retrograde_window() {
    let Some(path) = de440_kernel::require_de440_kernel() else {
        return;
    };
    let backend = De440Backend::from_path(&path).expect("DE440 backend must load");
    let config = EngineConfig::default();
    let jd = julian_day(Utc.with_ymd_and_hms(2024, 4, 10, 12, 0, 0).unwrap());
    let motion = longitude_motion(&backend, CelestialBody::Mercury, jd, &config).expect("motion");
    assert!(motion.retrograde);
    assert!(motion.longitude_speed_deg_per_day < 0.0);
}
