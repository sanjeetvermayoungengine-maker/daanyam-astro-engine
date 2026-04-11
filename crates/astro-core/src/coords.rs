use thiserror::Error;

use crate::math::normalize_degrees;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EclipticCoordinate {
    pub longitude_deg: f64,
    pub latitude_deg: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EquatorialCoordinate {
    pub right_ascension_deg: f64,
    pub declination_deg: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrecessionModel {
    Iau2006,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NutationModel {
    Iau2000B,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TransformError {
    #[error("precession model not yet implemented: {0:?}")]
    UnimplementedPrecession(PrecessionModel),
    #[error("nutation model not yet implemented: {0:?}")]
    UnimplementedNutation(NutationModel),
}

pub trait PrecessionProvider {
    fn precess_ecliptic(
        &self,
        coord: EclipticCoordinate,
        from_jd: f64,
        to_jd: f64,
        model: PrecessionModel,
    ) -> Result<EclipticCoordinate, TransformError>;
}

pub trait NutationProvider {
    fn nutate_ecliptic(
        &self,
        coord: EclipticCoordinate,
        jd: f64,
        model: NutationModel,
    ) -> Result<EclipticCoordinate, TransformError>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct StubTransformProvider;

impl PrecessionProvider for StubTransformProvider {
    fn precess_ecliptic(
        &self,
        _coord: EclipticCoordinate,
        _from_jd: f64,
        _to_jd: f64,
        model: PrecessionModel,
    ) -> Result<EclipticCoordinate, TransformError> {
        Err(TransformError::UnimplementedPrecession(model))
    }
}

impl NutationProvider for StubTransformProvider {
    fn nutate_ecliptic(
        &self,
        _coord: EclipticCoordinate,
        _jd: f64,
        model: NutationModel,
    ) -> Result<EclipticCoordinate, TransformError> {
        Err(TransformError::UnimplementedNutation(model))
    }
}

pub fn normalize_ecliptic(coord: EclipticCoordinate) -> EclipticCoordinate {
    EclipticCoordinate {
        longitude_deg: normalize_degrees(coord.longitude_deg),
        latitude_deg: coord.latitude_deg,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_ecliptic_longitude() {
        let coord =
            normalize_ecliptic(EclipticCoordinate { longitude_deg: -1.0, latitude_deg: 2.5 });
        assert_eq!(coord.longitude_deg, 359.0);
        assert_eq!(coord.latitude_deg, 2.5);
    }

    #[test]
    fn precession_stub_fails_explicitly() {
        let provider = StubTransformProvider;
        let err = provider
            .precess_ecliptic(
                EclipticCoordinate { longitude_deg: 10.0, latitude_deg: 0.0 },
                2_451_545.0,
                2_460_000.0,
                PrecessionModel::Iau2006,
            )
            .expect_err("stub must fail explicitly");
        assert_eq!(err, TransformError::UnimplementedPrecession(PrecessionModel::Iau2006));
    }

    #[test]
    fn nutation_stub_fails_explicitly() {
        let provider = StubTransformProvider;
        let err = provider
            .nutate_ecliptic(
                EclipticCoordinate { longitude_deg: 10.0, latitude_deg: 0.0 },
                2_451_545.0,
                NutationModel::Iau2000B,
            )
            .expect_err("stub must fail explicitly");
        assert_eq!(err, TransformError::UnimplementedNutation(NutationModel::Iau2000B));
    }
}
