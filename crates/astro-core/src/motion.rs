//! Apparent geocentric ecliptic longitude motion (speed and retrograde flag).

use crate::{
    backend::{BackendError, EphemerisBackend},
    contracts::{CelestialBody, CoordinateFrame, EngineConfig},
};

/// Half-width of the symmetric Julian-day window used for central-difference speed.
pub const LONGITUDE_SPEED_DELTA_DAYS: f64 = 0.5;

/// Speed magnitudes below this threshold are treated as prograde (not retrograde).
pub const RETROGRADE_SPEED_EPSILON: f64 = 1e-12;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LongitudeMotion {
    pub longitude_speed_deg_per_day: f64,
    pub retrograde: bool,
}

pub fn signed_longitude_delta_deg(start_deg: f64, end_deg: f64) -> f64 {
    (end_deg - start_deg + 540.0).rem_euclid(360.0) - 180.0
}

pub fn retrograde_from_speed(speed_deg_per_day: f64) -> bool {
    speed_deg_per_day < -RETROGRADE_SPEED_EPSILON
}

pub fn longitude_motion(
    backend: &dyn EphemerisBackend,
    body: CelestialBody,
    jd_utc: f64,
    config: &EngineConfig,
) -> Result<LongitudeMotion, BackendError> {
    let previous = backend.position(
        body,
        jd_utc - LONGITUDE_SPEED_DELTA_DAYS,
        CoordinateFrame::EclipticGeocentric,
        None,
        config,
    )?;
    let next = backend.position(
        body,
        jd_utc + LONGITUDE_SPEED_DELTA_DAYS,
        CoordinateFrame::EclipticGeocentric,
        None,
        config,
    )?;
    let delta =
        signed_longitude_delta_deg(previous.position.longitude_deg, next.position.longitude_deg);
    let longitude_speed_deg_per_day = delta / (LONGITUDE_SPEED_DELTA_DAYS * 2.0);
    Ok(LongitudeMotion {
        longitude_speed_deg_per_day,
        retrograde: retrograde_from_speed(longitude_speed_deg_per_day),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_longitude_delta_wraps_correctly() {
        assert!((signed_longitude_delta_deg(359.0, 1.0) - 2.0).abs() < 1e-9);
        assert!((signed_longitude_delta_deg(1.0, 359.0) + 2.0).abs() < 1e-9);
    }

    #[test]
    fn station_speed_is_not_retrograde() {
        assert!(!retrograde_from_speed(0.0));
        assert!(!retrograde_from_speed(1e-13));
        assert!(retrograde_from_speed(-1e-11));
    }
}
