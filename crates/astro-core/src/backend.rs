use std::collections::HashMap;

use thiserror::Error;

use crate::contracts::{
    AyanamsaModel, BodyComputationMeta, BodyPosition, CelestialBody, CoordinateFrame, EngineConfig,
    HouseSet, HouseSystem, Observer, PositionResult,
};

#[derive(Debug, Error)]
pub enum BackendError {
    #[error("unsupported body: {0:?}")]
    UnsupportedBody(CelestialBody),
    #[error("unsupported ayanamsa: {0:?}")]
    UnsupportedAyanamsa(AyanamsaModel),
    #[error("unsupported operation: {0}")]
    UnsupportedOperation(&'static str),
    #[error("environment variable ASTRO_EPHE_PATH is not set")]
    MissingEphemerisPath,
    #[error("ephemeris I/O error while {context}: {message}")]
    Io { context: &'static str, message: String },
    #[error("invalid ephemeris: {0}")]
    InvalidEphemeris(String),
    #[error("julian day {jd} is out of range for DE440 backend")]
    DateOutOfRange { jd: f64 },
}

pub trait EphemerisBackend: Send + Sync {
    fn position(
        &self,
        body: CelestialBody,
        jd: f64,
        frame: CoordinateFrame,
        observer: Option<&Observer>,
        config: &EngineConfig,
    ) -> Result<PositionResult, BackendError>;

    fn ayanamsa(&self, jd: f64, model: AyanamsaModel) -> Result<f64, BackendError>;

    fn houses(
        &self,
        jd: f64,
        lat_deg: f64,
        lon_deg: f64,
        system: HouseSystem,
    ) -> Result<HouseSet, BackendError>;
}

#[derive(Debug, Default)]
pub struct InMemoryBackend {
    positions: HashMap<CelestialBody, BodyPosition>,
    ayanamsa_deg: f64,
}

impl InMemoryBackend {
    pub fn new(positions: HashMap<CelestialBody, BodyPosition>, ayanamsa_deg: f64) -> Self {
        Self { positions, ayanamsa_deg }
    }
}

impl EphemerisBackend for InMemoryBackend {
    fn position(
        &self,
        body: CelestialBody,
        _jd: f64,
        frame: CoordinateFrame,
        _observer: Option<&Observer>,
        config: &EngineConfig,
    ) -> Result<PositionResult, BackendError> {
        let position =
            self.positions.get(&body).cloned().ok_or(BackendError::UnsupportedBody(body))?;

        if position.frame != frame {
            return Err(BackendError::UnsupportedOperation(
                "frame conversion is not available in in-memory backend",
            ));
        }

        Ok(PositionResult {
            position,
            computation_meta: BodyComputationMeta {
                frame: "seeded_ecliptic_geocentric".to_owned(),
                observer: "geocenter".to_owned(),
                topocentric_applied: false,
                kernel: "in_memory".to_owned(),
                kernel_notes: Some("seeded deterministic test backend".to_owned()),
                crate_version: env!("CARGO_PKG_VERSION").to_owned(),
                light_time: false,
                stellar_aberration: false,
                gravitational_deflection: config.gravitational_deflection,
                motion_model: None,
                node_policy: Some(config.node_mode),
                ayanamsa_algorithm: None,
            },
        })
    }

    fn ayanamsa(&self, _jd: f64, _model: AyanamsaModel) -> Result<f64, BackendError> {
        Ok(self.ayanamsa_deg)
    }

    fn houses(
        &self,
        _jd: f64,
        _lat_deg: f64,
        _lon_deg: f64,
        system: HouseSystem,
    ) -> Result<HouseSet, BackendError> {
        Ok(HouseSet {
            system,
            cusps_deg: (0..12).map(|idx| f64::from(idx) * 30.0).collect(),
            ascendant_deg: 0.0,
            midheaven_deg: 90.0,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn in_memory_backend_returns_seeded_position() {
        let mut positions = HashMap::new();
        positions.insert(
            CelestialBody::Moon,
            BodyPosition {
                body: CelestialBody::Moon,
                longitude_deg: 123.4,
                latitude_deg: 5.6,
                distance_au: None,
                frame: CoordinateFrame::EclipticGeocentric,
            },
        );

        let backend = InMemoryBackend::new(positions, 24.0);
        let position = backend
            .position(
                CelestialBody::Moon,
                2_451_545.0,
                CoordinateFrame::EclipticGeocentric,
                None,
                &EngineConfig::default(),
            )
            .expect("seeded position must exist");

        assert_eq!(position.position.longitude_deg, 123.4);
    }

    #[test]
    fn in_memory_backend_rejects_unknown_body() {
        let backend = InMemoryBackend::default();
        let err = backend
            .position(
                CelestialBody::Sun,
                2_451_545.0,
                CoordinateFrame::EclipticGeocentric,
                None,
                &EngineConfig::default(),
            )
            .expect_err("unknown body must fail");

        assert!(matches!(err, BackendError::UnsupportedBody(CelestialBody::Sun)));
    }
}
