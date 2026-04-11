use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use crate::{
    backend::{BackendError, EphemerisBackend},
    contracts::{
        AyanamsaModel, BodyComputationMeta, BodyPosition, CelestialBody, CoordinateFrame,
        EngineConfig, HouseSet, HouseSystem, Observer, PositionResult,
    },
    math::normalize_degrees,
};

const DAF_RECORD_BYTES: usize = 1024;
const DAF_RECORD_BYTES_U64: u64 = 1024;
const J2000_JULIAN_DAY: f64 = 2_451_545.0;
const JULIAN_DAYS_PER_CENTURY: f64 = 36_525.0;
const SECONDS_PER_DAY: f64 = 86_400.0;
const LIGHT_SPEED_KM_S: f64 = 299_792.458;
const AU_IN_KM: f64 = 149_597_870.7;
const EXPECTED_LOCFMT: &str = "LTL-IEEE";
const ASTRO_EPHE_PATH: &str = "ASTRO_EPHE_PATH";
const MERCURY_BARYCENTER_TARGET: i32 = 1;
const VENUS_BARYCENTER_TARGET: i32 = 2;
const EARTH_MOON_BARYCENTER_TARGET: i32 = 3;
const MARS_BARYCENTER_TARGET: i32 = 4;
const JUPITER_BARYCENTER_TARGET: i32 = 5;
const SATURN_BARYCENTER_TARGET: i32 = 6;
const SUN_TARGET: i32 = 10;
const MOON_TARGET: i32 = 301;
const EARTH_TARGET: i32 = 399;
const SOLAR_SYSTEM_BARYCENTER: i32 = 0;
const EARTH_MOON_BARYCENTER_CENTER: i32 = 3;
const J2000_FRAME: i32 = 1;
const TYPE2_CHEBYSHEV: i32 = 2;
const SUN_GM_KM3_S2: f64 = 1.327_124_400_18e11;

#[derive(Debug, Clone)]
pub struct De440Backend {
    path: PathBuf,
    header: DafHeader,
    mercury_barycenter_segment: Type2Segment,
    venus_barycenter_segment: Type2Segment,
    earth_moon_barycenter_segment: Type2Segment,
    mars_barycenter_segment: Type2Segment,
    jupiter_barycenter_segment: Type2Segment,
    saturn_barycenter_segment: Type2Segment,
    sun_segment: Type2Segment,
    moon_segment: Type2Segment,
    earth_segment: Type2Segment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DafHeader {
    pub nd: i32,
    pub ni: i32,
    pub forward_record: i32,
    pub backward_record: i32,
    pub free_record: i32,
    pub little_endian: bool,
    pub record_length_bytes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SummaryRecordHeader {
    next_record: i32,
    summary_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SegmentSummary {
    initial_epoch_seconds: f64,
    final_epoch_seconds: f64,
    target: i32,
    center: i32,
    frame: i32,
    data_type: i32,
    start_address: i32,
    end_address: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Type2Segment {
    start_address: i32,
    initial_epoch_seconds: f64,
    final_epoch_seconds: f64,
    init_seconds: f64,
    interval_length_seconds: f64,
    record_size_words: usize,
    record_count: usize,
    coefficients_per_component: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct KernelState {
    position_km: [f64; 3],
    velocity_km_s: [f64; 3],
}

#[derive(Debug, Clone, Copy)]
struct LeapSecond {
    effective_jd_utc: f64,
    tai_minus_utc_seconds: f64,
}

const LEAP_SECONDS: [LeapSecond; 28] = [
    LeapSecond { effective_jd_utc: 2_441_317.5, tai_minus_utc_seconds: 10.0 },
    LeapSecond { effective_jd_utc: 2_441_499.5, tai_minus_utc_seconds: 11.0 },
    LeapSecond { effective_jd_utc: 2_441_683.5, tai_minus_utc_seconds: 12.0 },
    LeapSecond { effective_jd_utc: 2_442_048.5, tai_minus_utc_seconds: 13.0 },
    LeapSecond { effective_jd_utc: 2_442_413.5, tai_minus_utc_seconds: 14.0 },
    LeapSecond { effective_jd_utc: 2_442_778.5, tai_minus_utc_seconds: 15.0 },
    LeapSecond { effective_jd_utc: 2_443_144.5, tai_minus_utc_seconds: 16.0 },
    LeapSecond { effective_jd_utc: 2_443_509.5, tai_minus_utc_seconds: 17.0 },
    LeapSecond { effective_jd_utc: 2_443_874.5, tai_minus_utc_seconds: 18.0 },
    LeapSecond { effective_jd_utc: 2_444_239.5, tai_minus_utc_seconds: 19.0 },
    LeapSecond { effective_jd_utc: 2_444_786.5, tai_minus_utc_seconds: 20.0 },
    LeapSecond { effective_jd_utc: 2_445_151.5, tai_minus_utc_seconds: 21.0 },
    LeapSecond { effective_jd_utc: 2_445_516.5, tai_minus_utc_seconds: 22.0 },
    LeapSecond { effective_jd_utc: 2_446_247.5, tai_minus_utc_seconds: 23.0 },
    LeapSecond { effective_jd_utc: 2_447_161.5, tai_minus_utc_seconds: 24.0 },
    LeapSecond { effective_jd_utc: 2_447_892.5, tai_minus_utc_seconds: 25.0 },
    LeapSecond { effective_jd_utc: 2_448_257.5, tai_minus_utc_seconds: 26.0 },
    LeapSecond { effective_jd_utc: 2_448_804.5, tai_minus_utc_seconds: 27.0 },
    LeapSecond { effective_jd_utc: 2_449_169.5, tai_minus_utc_seconds: 28.0 },
    LeapSecond { effective_jd_utc: 2_449_534.5, tai_minus_utc_seconds: 29.0 },
    LeapSecond { effective_jd_utc: 2_450_083.5, tai_minus_utc_seconds: 30.0 },
    LeapSecond { effective_jd_utc: 2_450_630.5, tai_minus_utc_seconds: 31.0 },
    LeapSecond { effective_jd_utc: 2_451_179.5, tai_minus_utc_seconds: 32.0 },
    LeapSecond { effective_jd_utc: 2_453_736.5, tai_minus_utc_seconds: 33.0 },
    LeapSecond { effective_jd_utc: 2_454_832.5, tai_minus_utc_seconds: 34.0 },
    LeapSecond { effective_jd_utc: 2_456_109.5, tai_minus_utc_seconds: 35.0 },
    LeapSecond { effective_jd_utc: 2_457_204.5, tai_minus_utc_seconds: 36.0 },
    LeapSecond { effective_jd_utc: 2_457_754.5, tai_minus_utc_seconds: 37.0 },
];

impl De440Backend {
    pub fn from_env() -> Result<Self, BackendError> {
        let path =
            std::env::var(ASTRO_EPHE_PATH).map_err(|_| BackendError::MissingEphemerisPath)?;
        Self::from_path(path)
    }

    pub fn from_path(path: impl Into<PathBuf>) -> Result<Self, BackendError> {
        let path = path.into();
        let header = parse_header(&path)?;
        let summaries = parse_segment_summaries(&path, &header)?;

        let mercury_barycenter_segment = parse_type2_segment(
            &path,
            find_segment_summary(&summaries, MERCURY_BARYCENTER_TARGET, SOLAR_SYSTEM_BARYCENTER)?,
        )?;
        let venus_barycenter_segment = parse_type2_segment(
            &path,
            find_segment_summary(&summaries, VENUS_BARYCENTER_TARGET, SOLAR_SYSTEM_BARYCENTER)?,
        )?;
        let earth_moon_barycenter_segment = parse_type2_segment(
            &path,
            find_segment_summary(
                &summaries,
                EARTH_MOON_BARYCENTER_TARGET,
                SOLAR_SYSTEM_BARYCENTER,
            )?,
        )?;
        let mars_barycenter_segment = parse_type2_segment(
            &path,
            find_segment_summary(&summaries, MARS_BARYCENTER_TARGET, SOLAR_SYSTEM_BARYCENTER)?,
        )?;
        let jupiter_barycenter_segment = parse_type2_segment(
            &path,
            find_segment_summary(&summaries, JUPITER_BARYCENTER_TARGET, SOLAR_SYSTEM_BARYCENTER)?,
        )?;
        let saturn_barycenter_segment = parse_type2_segment(
            &path,
            find_segment_summary(&summaries, SATURN_BARYCENTER_TARGET, SOLAR_SYSTEM_BARYCENTER)?,
        )?;
        let sun_segment = parse_type2_segment(
            &path,
            find_segment_summary(&summaries, SUN_TARGET, SOLAR_SYSTEM_BARYCENTER)?,
        )?;
        let moon_segment = parse_type2_segment(
            &path,
            find_segment_summary(&summaries, MOON_TARGET, EARTH_MOON_BARYCENTER_CENTER)?,
        )?;
        let earth_segment = parse_type2_segment(
            &path,
            find_segment_summary(&summaries, EARTH_TARGET, EARTH_MOON_BARYCENTER_CENTER)?,
        )?;

        Ok(Self {
            path,
            header,
            mercury_barycenter_segment,
            venus_barycenter_segment,
            earth_moon_barycenter_segment,
            mars_barycenter_segment,
            jupiter_barycenter_segment,
            saturn_barycenter_segment,
            sun_segment,
            moon_segment,
            earth_segment,
        })
    }

    pub fn header(&self) -> &DafHeader {
        &self.header
    }

    fn open_file(&self) -> Result<File, BackendError> {
        File::open(&self.path).map_err(|err| BackendError::Io {
            context: "opening ephemeris file",
            message: err.to_string(),
        })
    }

    fn barycentric_state(
        &self,
        file: &mut File,
        target: KernelTarget,
        jd_tdb: f64,
    ) -> Result<KernelState, BackendError> {
        match target {
            KernelTarget::MercuryBarycenter => {
                interpolate_type2_segment(file, &self.mercury_barycenter_segment, jd_tdb)
            }
            KernelTarget::VenusBarycenter => {
                interpolate_type2_segment(file, &self.venus_barycenter_segment, jd_tdb)
            }
            KernelTarget::EarthMoonBarycenter => {
                interpolate_type2_segment(file, &self.earth_moon_barycenter_segment, jd_tdb)
            }
            KernelTarget::MarsBarycenter => {
                interpolate_type2_segment(file, &self.mars_barycenter_segment, jd_tdb)
            }
            KernelTarget::JupiterBarycenter => {
                interpolate_type2_segment(file, &self.jupiter_barycenter_segment, jd_tdb)
            }
            KernelTarget::SaturnBarycenter => {
                interpolate_type2_segment(file, &self.saturn_barycenter_segment, jd_tdb)
            }
            KernelTarget::Sun => interpolate_type2_segment(file, &self.sun_segment, jd_tdb),
            KernelTarget::Moon => {
                let emb =
                    self.barycentric_state(file, KernelTarget::EarthMoonBarycenter, jd_tdb)?;
                let moon_relative = interpolate_type2_segment(file, &self.moon_segment, jd_tdb)?;
                Ok(emb + moon_relative)
            }
            KernelTarget::Earth => {
                let emb =
                    self.barycentric_state(file, KernelTarget::EarthMoonBarycenter, jd_tdb)?;
                let earth_relative = interpolate_type2_segment(file, &self.earth_segment, jd_tdb)?;
                Ok(emb + earth_relative)
            }
        }
    }

    fn apparent_pipeline(
        &self,
        file: &mut File,
        target: KernelTarget,
        jd_utc: f64,
        config: &EngineConfig,
    ) -> Result<ApparentPosition, BackendError> {
        let jd_tdb = utc_to_tdb_julian_day(jd_utc);
        let observer = self.barycentric_state(file, KernelTarget::Earth, jd_tdb)?;
        let sun = self.barycentric_state(file, KernelTarget::Sun, jd_tdb)?;
        let apparent_vector_j2000 =
            self.apparent_vector_j2000(file, target, jd_tdb, observer, sun, config)?;
        let mean_equator_of_date = precess_j2000_to_mean_of_date(apparent_vector_j2000, jd_tdb);
        let ecliptic_of_date = mean_equator_to_ecliptic_of_date(mean_equator_of_date, jd_tdb);

        let longitude_deg =
            normalize_degrees(ecliptic_of_date[1].atan2(ecliptic_of_date[0]).to_degrees());
        let latitude_deg =
            ecliptic_of_date[2].atan2(ecliptic_of_date[0].hypot(ecliptic_of_date[1])).to_degrees();
        let distance_au = Some(
            apparent_vector_j2000[0]
                .hypot(apparent_vector_j2000[1])
                .hypot(apparent_vector_j2000[2])
                / AU_IN_KM,
        );

        Ok(ApparentPosition { longitude_deg, latitude_deg, distance_au })
    }

    fn true_node_position(
        &self,
        file: &mut File,
        body: CelestialBody,
        jd: f64,
        config: &EngineConfig,
    ) -> Result<PositionResult, BackendError> {
        if config.node_mode != crate::contracts::NodeMode::True {
            return Err(BackendError::UnsupportedOperation(
                "DE440 backend currently supports nodes only in true-node mode",
            ));
        }

        let jd_tdb = utc_to_tdb_julian_day(jd);
        let observer = self.barycentric_state(file, KernelTarget::Earth, jd_tdb)?;
        let moon = self.barycentric_state(file, KernelTarget::Moon, jd_tdb)?;
        let geocentric_moon = subtract_vectors(moon.position_km, observer.position_km);
        let geocentric_moon_velocity = subtract_vectors(moon.velocity_km_s, observer.velocity_km_s);
        let ecliptic_position = mean_equator_to_ecliptic_of_date(
            precess_j2000_to_mean_of_date(geocentric_moon, jd_tdb),
            jd_tdb,
        );
        let ecliptic_velocity = mean_equator_to_ecliptic_of_date(
            precess_j2000_to_mean_of_date(geocentric_moon_velocity, jd_tdb),
            jd_tdb,
        );
        let orbital_angular_momentum = cross(ecliptic_position, ecliptic_velocity);
        let ascending_node =
            normalize_vector([-orbital_angular_momentum[1], orbital_angular_momentum[0], 0.0]);

        let mut longitude_deg =
            normalize_degrees(ascending_node[1].atan2(ascending_node[0]).to_degrees());
        if body == CelestialBody::Ketu {
            longitude_deg = normalize_degrees(longitude_deg + 180.0);
        }

        Ok(PositionResult {
            position: BodyPosition {
                body,
                longitude_deg,
                latitude_deg: 0.0,
                distance_au: None,
                frame: CoordinateFrame::EclipticGeocentric,
            },
            computation_meta: BodyComputationMeta {
                frame: "mean_ecliptic_of_date".to_owned(),
                observer: "geocenter".to_owned(),
                topocentric_applied: false,
                kernel: "derived_true_node_from_de440_moon".to_owned(),
                kernel_notes: Some(
                    "derived from the Moon's instantaneous orbital plane against the mean ecliptic of date"
                        .to_owned(),
                ),
                crate_version: env!("CARGO_PKG_VERSION").to_owned(),
                light_time: false,
                stellar_aberration: false,
                gravitational_deflection: false,
                motion_model: None,
                node_policy: Some(config.node_mode),
                ayanamsa_algorithm: None,
            },
        })
    }

    fn apparent_vector_j2000(
        &self,
        file: &mut File,
        target: KernelTarget,
        jd_tdb: f64,
        observer: KernelState,
        sun: KernelState,
        config: &EngineConfig,
    ) -> Result<[f64; 3], BackendError> {
        // Stage 1: iterate down-leg light-time using the target barycentric state
        // evaluated at the retarded TDB epoch.
        let mut target_state = self.barycentric_state(file, target, jd_tdb)?;
        for _ in 0..5 {
            let line_of_sight = subtract_vectors(target_state.position_km, observer.position_km);
            let light_time_seconds = vector_norm(line_of_sight) / LIGHT_SPEED_KM_S;
            target_state = self.barycentric_state(
                file,
                target,
                jd_tdb - light_time_seconds / SECONDS_PER_DAY,
            )?;
        }

        // Stage 2: derive the astrometric line-of-sight vector from observer to
        // retarded target position.
        let line_of_sight = subtract_vectors(target_state.position_km, observer.position_km);
        let unit_astrometric = normalize_vector(line_of_sight);

        // Stage 3: apply observer-velocity stellar aberration in the J2000 frame.
        let aberrated = apply_stellar_aberration(
            unit_astrometric,
            observer.velocity_km_s,
            vector_norm(line_of_sight),
        );

        // Stage 4: optionally apply solar gravitational deflection before the
        // frame-of-date transforms.
        Ok(if config.gravitational_deflection {
            apply_solar_gravitational_deflection(aberrated, observer.position_km, sun.position_km)
        } else {
            aberrated
        })
    }
}

impl EphemerisBackend for De440Backend {
    fn position(
        &self,
        body: CelestialBody,
        jd: f64,
        frame: CoordinateFrame,
        observer: Option<&Observer>,
        config: &EngineConfig,
    ) -> Result<PositionResult, BackendError> {
        let kernel_target = match body {
            CelestialBody::Moon => KernelTarget::Moon,
            CelestialBody::Sun => KernelTarget::Sun,
            CelestialBody::Mercury => KernelTarget::MercuryBarycenter,
            CelestialBody::Venus => KernelTarget::VenusBarycenter,
            CelestialBody::Mars => KernelTarget::MarsBarycenter,
            CelestialBody::Jupiter => KernelTarget::JupiterBarycenter,
            CelestialBody::Saturn => KernelTarget::SaturnBarycenter,
            CelestialBody::Rahu | CelestialBody::Ketu => {
                let mut file = self.open_file()?;
                return self.true_node_position(&mut file, body, jd, config);
            }
        };

        if frame != CoordinateFrame::EclipticGeocentric {
            return Err(BackendError::UnsupportedOperation(
                "DE440 apparent pipeline currently supports only geocentric ecliptic positions",
            ));
        }

        if observer.is_some() {
            return Err(BackendError::UnsupportedOperation(
                "DE440 apparent pipeline currently supports only the geocenter observer",
            ));
        }

        let jd_tdb = utc_to_tdb_julian_day(jd);
        if !self.supports_jd(jd_tdb) {
            return Err(BackendError::DateOutOfRange { jd });
        }

        let mut file = self.open_file()?;
        let apparent = self.apparent_pipeline(&mut file, kernel_target, jd, config)?;

        Ok(PositionResult {
            position: BodyPosition {
                body,
                longitude_deg: apparent.longitude_deg,
                latitude_deg: apparent.latitude_deg,
                distance_au: apparent.distance_au,
                frame: CoordinateFrame::EclipticGeocentric,
            },
            computation_meta: BodyComputationMeta {
                frame: "apparent_ecliptic_of_date".to_owned(),
                observer: "geocenter".to_owned(),
                topocentric_applied: false,
                kernel: kernel_name(kernel_target).to_owned(),
                kernel_notes: Some(kernel_notes(kernel_target).to_owned()),
                crate_version: env!("CARGO_PKG_VERSION").to_owned(),
                light_time: true,
                stellar_aberration: true,
                gravitational_deflection: config.gravitational_deflection,
                motion_model: None,
                node_policy: Some(config.node_mode),
                ayanamsa_algorithm: None,
            },
        })
    }

    fn ayanamsa(&self, _jd: f64, _model: AyanamsaModel) -> Result<f64, BackendError> {
        Err(BackendError::UnsupportedOperation(
            "DE440 backend does not provide ayanamsa in Phase 1",
        ))
    }

    fn houses(
        &self,
        jd: f64,
        lat_deg: f64,
        lon_deg: f64,
        system: HouseSystem,
    ) -> Result<HouseSet, BackendError> {
        if system != HouseSystem::WholeSign {
            return Err(BackendError::UnsupportedOperation(
                "DE440 backend currently supports only whole sign houses",
            ));
        }

        let jd_tdb = utc_to_tdb_julian_day(jd);
        if !self.supports_jd(jd_tdb) {
            return Err(BackendError::DateOutOfRange { jd });
        }

        let obliquity_deg = mean_obliquity_of_date_deg(jd_tdb);
        let local_sidereal_time_deg = local_mean_sidereal_time_deg(jd, lon_deg);
        let ascendant_deg =
            ascendant_longitude_deg(local_sidereal_time_deg, lat_deg, obliquity_deg);
        let first_house_cusp_deg = (ascendant_deg / 30.0).floor() * 30.0;

        Ok(HouseSet {
            system,
            cusps_deg: (0..12)
                .map(|offset| normalize_degrees(first_house_cusp_deg + f64::from(offset) * 30.0))
                .collect(),
            ascendant_deg,
            midheaven_deg: midheaven_longitude_deg(local_sidereal_time_deg, obliquity_deg),
        })
    }
}

impl De440Backend {
    fn supports_jd(&self, jd_tdb: f64) -> bool {
        let et_seconds = (jd_tdb - J2000_JULIAN_DAY) * SECONDS_PER_DAY;
        et_seconds >= self.sun_segment.initial_epoch_seconds
            && et_seconds <= self.sun_segment.final_epoch_seconds
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KernelTarget {
    MercuryBarycenter,
    VenusBarycenter,
    EarthMoonBarycenter,
    MarsBarycenter,
    JupiterBarycenter,
    SaturnBarycenter,
    Sun,
    Moon,
    Earth,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ApparentPosition {
    longitude_deg: f64,
    latitude_deg: f64,
    distance_au: Option<f64>,
}

impl std::ops::Add for KernelState {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            position_km: [
                self.position_km[0] + rhs.position_km[0],
                self.position_km[1] + rhs.position_km[1],
                self.position_km[2] + rhs.position_km[2],
            ],
            velocity_km_s: [
                self.velocity_km_s[0] + rhs.velocity_km_s[0],
                self.velocity_km_s[1] + rhs.velocity_km_s[1],
                self.velocity_km_s[2] + rhs.velocity_km_s[2],
            ],
        }
    }
}

fn parse_header(path: &Path) -> Result<DafHeader, BackendError> {
    let mut file = File::open(path).map_err(|err| BackendError::Io {
        context: "opening ephemeris header",
        message: err.to_string(),
    })?;
    let mut header = [0_u8; DAF_RECORD_BYTES];
    file.read_exact(&mut header).map_err(|err| BackendError::Io {
        context: "reading ephemeris header",
        message: err.to_string(),
    })?;

    if &header[0..8] != b"DAF/SPK " {
        return Err(BackendError::InvalidEphemeris("file is not a DAF/SPK kernel".to_owned()));
    }

    let locfmt = std::str::from_utf8(&header[88..96])
        .map_err(|_| BackendError::InvalidEphemeris("invalid LOCFMT field".to_owned()))?
        .trim();
    if locfmt != EXPECTED_LOCFMT {
        return Err(BackendError::InvalidEphemeris(format!(
            "unsupported LOCFMT `{locfmt}`; expected `{EXPECTED_LOCFMT}`"
        )));
    }

    Ok(DafHeader {
        nd: i32::from_le_bytes(header[8..12].try_into().expect("fixed-width slice")),
        ni: i32::from_le_bytes(header[12..16].try_into().expect("fixed-width slice")),
        forward_record: i32::from_le_bytes(header[76..80].try_into().expect("fixed-width slice")),
        backward_record: i32::from_le_bytes(header[80..84].try_into().expect("fixed-width slice")),
        free_record: i32::from_le_bytes(header[84..88].try_into().expect("fixed-width slice")),
        little_endian: true,
        record_length_bytes: DAF_RECORD_BYTES,
    })
}

fn parse_segment_summaries(
    path: &Path,
    header: &DafHeader,
) -> Result<Vec<SegmentSummary>, BackendError> {
    let mut file = File::open(path).map_err(|err| BackendError::Io {
        context: "opening ephemeris summary records",
        message: err.to_string(),
    })?;
    let mut summaries = Vec::new();
    let summary_words = usize::try_from(header.nd)
        .map_err(|_| BackendError::InvalidEphemeris("negative ND in header".to_owned()))?
        + usize::try_from((header.ni + 1) / 2)
            .map_err(|_| BackendError::InvalidEphemeris("negative NI in header".to_owned()))?;

    let mut record_number = header.forward_record;
    while record_number != 0 {
        let mut record = [0_u8; DAF_RECORD_BYTES];
        file.seek(SeekFrom::Start(
            u64::try_from(record_number - 1).expect("record number must be positive")
                * DAF_RECORD_BYTES_U64,
        ))
        .map_err(|err| BackendError::Io {
            context: "seeking summary record",
            message: err.to_string(),
        })?;
        file.read_exact(&mut record).map_err(|err| BackendError::Io {
            context: "reading summary record",
            message: err.to_string(),
        })?;

        let summary_header = SummaryRecordHeader {
            next_record: f64::from_le_bytes(record[0..8].try_into().expect("fixed-width slice"))
                as i32,
            summary_count: f64::from_le_bytes(record[16..24].try_into().expect("fixed-width slice"))
                as usize,
        };

        for summary_index in 0..summary_header.summary_count {
            let offset = 24 + summary_index * summary_words * 8;
            summaries.push(SegmentSummary {
                initial_epoch_seconds: f64::from_le_bytes(
                    record[offset..offset + 8].try_into().expect("fixed-width slice"),
                ),
                final_epoch_seconds: f64::from_le_bytes(
                    record[offset + 8..offset + 16].try_into().expect("fixed-width slice"),
                ),
                target: i32::from_le_bytes(
                    record[offset + 16..offset + 20].try_into().expect("fixed-width slice"),
                ),
                center: i32::from_le_bytes(
                    record[offset + 20..offset + 24].try_into().expect("fixed-width slice"),
                ),
                frame: i32::from_le_bytes(
                    record[offset + 24..offset + 28].try_into().expect("fixed-width slice"),
                ),
                data_type: i32::from_le_bytes(
                    record[offset + 28..offset + 32].try_into().expect("fixed-width slice"),
                ),
                start_address: i32::from_le_bytes(
                    record[offset + 32..offset + 36].try_into().expect("fixed-width slice"),
                ),
                end_address: i32::from_le_bytes(
                    record[offset + 36..offset + 40].try_into().expect("fixed-width slice"),
                ),
            });
        }

        record_number = summary_header.next_record;
    }

    Ok(summaries)
}

fn find_segment_summary(
    summaries: &[SegmentSummary],
    target: i32,
    center: i32,
) -> Result<SegmentSummary, BackendError> {
    summaries
        .iter()
        .copied()
        .find(|summary| {
            summary.target == target
                && summary.center == center
                && summary.frame == J2000_FRAME
                && summary.data_type == TYPE2_CHEBYSHEV
        })
        .ok_or_else(|| {
            BackendError::InvalidEphemeris(format!(
                "missing type-2 J2000 segment for target {target} center {center}"
            ))
        })
}

fn local_mean_sidereal_time_deg(jd_utc: f64, longitude_deg: f64) -> f64 {
    let centuries = (jd_utc - J2000_JULIAN_DAY) / JULIAN_DAYS_PER_CENTURY;
    normalize_degrees(
        280.460_618_37
            + 360.985_647_366_29 * (jd_utc - J2000_JULIAN_DAY)
            + 0.000_387_933 * centuries * centuries
            - centuries * centuries * centuries / 38_710_000.0
            + longitude_deg,
    )
}

fn mean_obliquity_of_date_deg(jd_tdb: f64) -> f64 {
    let centuries = (jd_tdb - J2000_JULIAN_DAY) / JULIAN_DAYS_PER_CENTURY;
    let arcseconds = 84_381.448 - 46.8150 * centuries - 0.00059 * centuries * centuries
        + 0.001_813 * centuries * centuries * centuries;
    arcseconds / 3600.0
}

fn ascendant_longitude_deg(
    local_sidereal_time_deg: f64,
    latitude_deg: f64,
    obliquity_deg: f64,
) -> f64 {
    let local_sidereal_time_rad = local_sidereal_time_deg.to_radians();
    let latitude_rad = latitude_deg.to_radians();
    let obliquity_rad = obliquity_deg.to_radians();

    normalize_degrees(
        (-local_sidereal_time_rad.cos())
            .atan2(
                obliquity_rad.sin() * latitude_rad.tan()
                    + obliquity_rad.cos() * local_sidereal_time_rad.sin(),
            )
            .to_degrees(),
    )
}

fn midheaven_longitude_deg(local_sidereal_time_deg: f64, obliquity_deg: f64) -> f64 {
    let local_sidereal_time_rad = local_sidereal_time_deg.to_radians();
    let obliquity_rad = obliquity_deg.to_radians();

    normalize_degrees(
        local_sidereal_time_rad
            .sin()
            .atan2(local_sidereal_time_rad.cos() * obliquity_rad.cos())
            .to_degrees(),
    )
}

fn parse_type2_segment(path: &Path, summary: SegmentSummary) -> Result<Type2Segment, BackendError> {
    let mut file = File::open(path).map_err(|err| BackendError::Io {
        context: "opening type-2 segment",
        message: err.to_string(),
    })?;
    let trailer_offset_words = i64::from(summary.end_address) - 4;
    file.seek(SeekFrom::Start(
        u64::try_from(trailer_offset_words).expect("segment trailer offset must be positive") * 8,
    ))
    .map_err(|err| BackendError::Io {
        context: "seeking type-2 segment trailer",
        message: err.to_string(),
    })?;

    let mut trailer = [0_u8; 32];
    file.read_exact(&mut trailer).map_err(|err| BackendError::Io {
        context: "reading type-2 segment trailer",
        message: err.to_string(),
    })?;

    let init_seconds = f64::from_le_bytes(trailer[0..8].try_into().expect("fixed-width slice"));
    let interval_length_seconds =
        f64::from_le_bytes(trailer[8..16].try_into().expect("fixed-width slice"));
    let record_size_words =
        f64::from_le_bytes(trailer[16..24].try_into().expect("fixed-width slice")) as usize;
    let record_count =
        f64::from_le_bytes(trailer[24..32].try_into().expect("fixed-width slice")) as usize;

    if record_size_words < 5 || (record_size_words - 2) % 3 != 0 {
        return Err(BackendError::InvalidEphemeris(format!(
            "invalid type-2 record size {record_size_words}"
        )));
    }

    Ok(Type2Segment {
        start_address: summary.start_address,
        initial_epoch_seconds: summary.initial_epoch_seconds,
        final_epoch_seconds: summary.final_epoch_seconds,
        init_seconds,
        interval_length_seconds,
        record_size_words,
        record_count,
        coefficients_per_component: (record_size_words - 2) / 3,
    })
}

fn interpolate_type2_segment(
    file: &mut File,
    segment: &Type2Segment,
    jd_tdb: f64,
) -> Result<KernelState, BackendError> {
    let et_seconds = (jd_tdb - J2000_JULIAN_DAY) * SECONDS_PER_DAY;
    let record_index =
        ((et_seconds - segment.init_seconds) / segment.interval_length_seconds).floor() as isize;
    let clamped_record_index = record_index
        .clamp(0, isize::try_from(segment.record_count).expect("record count fits in isize") - 1)
        as usize;
    let record_address = usize::try_from(segment.start_address)
        .expect("positive segment start address")
        + clamped_record_index * segment.record_size_words;

    file.seek(SeekFrom::Start(
        u64::try_from(record_address - 1).expect("record address must be positive") * 8,
    ))
    .map_err(|err| BackendError::Io {
        context: "seeking type-2 record",
        message: err.to_string(),
    })?;

    let mut raw_record = vec![0_u8; segment.record_size_words * 8];
    file.read_exact(&mut raw_record).map_err(|err| BackendError::Io {
        context: "reading type-2 record",
        message: err.to_string(),
    })?;

    let values = raw_record
        .chunks_exact(8)
        .map(|chunk| f64::from_le_bytes(chunk.try_into().expect("fixed-width slice")))
        .collect::<Vec<_>>();
    let midpoint_seconds = values[0];
    let radius_seconds = values[1];
    let x = (et_seconds - midpoint_seconds) / radius_seconds;
    let chebyshev = chebyshev_terms(x, segment.coefficients_per_component);
    let chebyshev_second_kind = chebyshev_second_kind_terms(x, segment.coefficients_per_component);
    let coefficients = &values[2..];

    let mut position_km = [0.0; 3];
    let mut velocity_km_s = [0.0; 3];
    for (axis, component) in position_km.iter_mut().enumerate() {
        let start = axis * segment.coefficients_per_component;
        let end = start + segment.coefficients_per_component;
        let axis_coefficients = &coefficients[start..end];
        *component = axis_coefficients
            .iter()
            .zip(&chebyshev)
            .map(|(coefficient, term)| coefficient * term)
            .sum();
        velocity_km_s[axis] = axis_coefficients
            .iter()
            .enumerate()
            .skip(1)
            .map(|(index, coefficient)| {
                f64::from(index as u32) * coefficient * chebyshev_second_kind[index - 1]
            })
            .sum::<f64>()
            / radius_seconds;
    }

    Ok(KernelState { position_km, velocity_km_s })
}

fn chebyshev_terms(x: f64, count: usize) -> Vec<f64> {
    let mut terms = Vec::with_capacity(count);
    if count == 0 {
        return terms;
    }

    terms.push(1.0);
    if count == 1 {
        return terms;
    }

    terms.push(x);
    while terms.len() < count {
        let next = 2.0 * x * terms[terms.len() - 1] - terms[terms.len() - 2];
        terms.push(next);
    }
    terms
}

fn chebyshev_second_kind_terms(x: f64, count: usize) -> Vec<f64> {
    let mut terms = Vec::with_capacity(count.saturating_sub(1));
    if count <= 1 {
        return terms;
    }

    terms.push(1.0);
    if count == 2 {
        return terms;
    }

    terms.push(2.0 * x);
    while terms.len() < count - 1 {
        let next = 2.0 * x * terms[terms.len() - 1] - terms[terms.len() - 2];
        terms.push(next);
    }
    terms
}

fn utc_to_tdb_julian_day(jd_utc: f64) -> f64 {
    let jd_tt = jd_utc + (tai_minus_utc_seconds(jd_utc) + 32.184) / SECONDS_PER_DAY;
    let mean_anomaly = (357.53 + 0.985_600_3 * (jd_tt - J2000_JULIAN_DAY)).to_radians();
    let tdb_offset_days =
        (0.001_658 * mean_anomaly.sin() + 0.000_014 * (2.0 * mean_anomaly).sin()) / SECONDS_PER_DAY;
    jd_tt + tdb_offset_days
}

fn tai_minus_utc_seconds(jd_utc: f64) -> f64 {
    let mut offset = 0.0;
    for leap_second in LEAP_SECONDS {
        if jd_utc >= leap_second.effective_jd_utc {
            offset = leap_second.tai_minus_utc_seconds;
        } else {
            break;
        }
    }
    offset
}

fn apply_stellar_aberration(
    unit_astrometric: [f64; 3],
    observer_velocity_km_s: [f64; 3],
    geometric_distance_km: f64,
) -> [f64; 3] {
    let _ = geometric_distance_km;
    let beta = [
        observer_velocity_km_s[0] / LIGHT_SPEED_KM_S,
        observer_velocity_km_s[1] / LIGHT_SPEED_KM_S,
        observer_velocity_km_s[2] / LIGHT_SPEED_KM_S,
    ];
    let beta_squared = dot(beta, beta);
    let gamma = 1.0 / (1.0 - beta_squared).sqrt();
    let projection = dot(unit_astrometric, beta);
    let scale = 1.0 / (1.0 + projection);
    let aberrated = [
        scale / gamma * unit_astrometric[0]
            + scale * (1.0 + gamma / (1.0 + gamma) * projection) * beta[0],
        scale / gamma * unit_astrometric[1]
            + scale * (1.0 + gamma / (1.0 + gamma) * projection) * beta[1],
        scale / gamma * unit_astrometric[2]
            + scale * (1.0 + gamma / (1.0 + gamma) * projection) * beta[2],
    ];
    normalize_vector(aberrated)
}

fn apply_solar_gravitational_deflection(
    unit_direction: [f64; 3],
    observer_barycentric_position_km: [f64; 3],
    sun_barycentric_position_km: [f64; 3],
) -> [f64; 3] {
    let observer_from_sun =
        subtract_vectors(observer_barycentric_position_km, sun_barycentric_position_km);
    let observer_distance = vector_norm(observer_from_sun);
    let sun_to_observer = normalize_vector(observer_from_sun);
    let denominator = 1.0 + dot(unit_direction, sun_to_observer);
    if denominator.abs() < 1.0e-15 {
        return unit_direction;
    }

    let scale = (2.0 * SUN_GM_KM3_S2) / (LIGHT_SPEED_KM_S * LIGHT_SPEED_KM_S * observer_distance);
    let correction = scale / denominator;
    normalize_vector([
        unit_direction[0]
            + correction
                * (sun_to_observer[0] - dot(unit_direction, sun_to_observer) * unit_direction[0]),
        unit_direction[1]
            + correction
                * (sun_to_observer[1] - dot(unit_direction, sun_to_observer) * unit_direction[1]),
        unit_direction[2]
            + correction
                * (sun_to_observer[2] - dot(unit_direction, sun_to_observer) * unit_direction[2]),
    ])
}

fn precess_j2000_to_mean_of_date(vector: [f64; 3], jd_tdb: f64) -> [f64; 3] {
    let t = (jd_tdb - J2000_JULIAN_DAY) / 36_525.0;
    let zeta_arcsec = 2306.2181 * t + 0.30188 * t * t + 0.017998 * t * t * t;
    let z_arcsec = 2306.2181 * t + 1.09468 * t * t + 0.018203 * t * t * t;
    let theta_arcsec = 2004.3109 * t - 0.42665 * t * t - 0.041833 * t * t * t;

    let zeta = (zeta_arcsec / 3600.0).to_radians();
    let z = (z_arcsec / 3600.0).to_radians();
    let theta = (theta_arcsec / 3600.0).to_radians();

    let rotation = [
        [
            z.cos() * theta.cos() * zeta.cos() - z.sin() * zeta.sin(),
            -z.cos() * theta.cos() * zeta.sin() - z.sin() * zeta.cos(),
            -z.cos() * theta.sin(),
        ],
        [
            z.sin() * theta.cos() * zeta.cos() + z.cos() * zeta.sin(),
            -z.sin() * theta.cos() * zeta.sin() + z.cos() * zeta.cos(),
            -z.sin() * theta.sin(),
        ],
        [theta.sin() * zeta.cos(), -theta.sin() * zeta.sin(), theta.cos()],
    ];

    mat_vec(rotation, vector)
}

fn mean_equator_to_ecliptic_of_date(vector: [f64; 3], jd_tdb: f64) -> [f64; 3] {
    let t = (jd_tdb - J2000_JULIAN_DAY) / 36_525.0;
    let mean_obliquity_arcsec = 84_381.448 - 46.8150 * t - 0.00059 * t * t + 0.001813 * t * t * t;
    let mean_obliquity = (mean_obliquity_arcsec / 3600.0).to_radians();

    [
        vector[0],
        mean_obliquity.cos() * vector[1] + mean_obliquity.sin() * vector[2],
        -mean_obliquity.sin() * vector[1] + mean_obliquity.cos() * vector[2],
    ]
}

fn mat_vec(matrix: [[f64; 3]; 3], vector: [f64; 3]) -> [f64; 3] {
    [
        matrix[0][0] * vector[0] + matrix[0][1] * vector[1] + matrix[0][2] * vector[2],
        matrix[1][0] * vector[0] + matrix[1][1] * vector[1] + matrix[1][2] * vector[2],
        matrix[2][0] * vector[0] + matrix[2][1] * vector[1] + matrix[2][2] * vector[2],
    ]
}

fn normalize_vector(vector: [f64; 3]) -> [f64; 3] {
    let norm = vector_norm(vector);
    [vector[0] / norm, vector[1] / norm, vector[2] / norm]
}

fn vector_norm(vector: [f64; 3]) -> f64 {
    dot(vector, vector).sqrt()
}

fn dot(lhs: [f64; 3], rhs: [f64; 3]) -> f64 {
    lhs[0] * rhs[0] + lhs[1] * rhs[1] + lhs[2] * rhs[2]
}

fn subtract_vectors(lhs: [f64; 3], rhs: [f64; 3]) -> [f64; 3] {
    [lhs[0] - rhs[0], lhs[1] - rhs[1], lhs[2] - rhs[2]]
}

fn cross(lhs: [f64; 3], rhs: [f64; 3]) -> [f64; 3] {
    [
        lhs[1] * rhs[2] - lhs[2] * rhs[1],
        lhs[2] * rhs[0] - lhs[0] * rhs[2],
        lhs[0] * rhs[1] - lhs[1] * rhs[0],
    ]
}

fn kernel_name(target: KernelTarget) -> &'static str {
    match target {
        KernelTarget::MercuryBarycenter => "de440_mercury_barycenter",
        KernelTarget::VenusBarycenter => "de440_venus_barycenter",
        KernelTarget::EarthMoonBarycenter => "de440_earth_moon_barycenter",
        KernelTarget::MarsBarycenter => "de440_mars_barycenter",
        KernelTarget::JupiterBarycenter => "de440_jupiter_barycenter",
        KernelTarget::SaturnBarycenter => "de440_saturn_barycenter",
        KernelTarget::Sun => "de440_sun",
        KernelTarget::Moon => "de440_moon",
        KernelTarget::Earth => "de440_earth",
    }
}

fn kernel_notes(target: KernelTarget) -> &'static str {
    match target {
        KernelTarget::MercuryBarycenter => "planetary barycenter segment from DE440",
        KernelTarget::VenusBarycenter => "planetary barycenter segment from DE440",
        KernelTarget::EarthMoonBarycenter => "earth-moon barycenter segment from DE440",
        KernelTarget::MarsBarycenter => "planetary barycenter segment from DE440",
        KernelTarget::JupiterBarycenter => "planetary barycenter segment from DE440",
        KernelTarget::SaturnBarycenter => "planetary barycenter segment from DE440",
        KernelTarget::Sun => "solar system barycentric Sun segment from DE440",
        KernelTarget::Moon => "Moon segment relative to the earth-moon barycenter plus EMB state",
        KernelTarget::Earth => "Earth segment relative to the earth-moon barycenter plus EMB state",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de440_kernel {
        include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/support/de440_kernel.rs"));
    }

    #[test]
    fn parses_de440_header() {
        let Some(backend) = test_backend() else {
            return;
        };
        assert_eq!(backend.header().nd, 2);
        assert_eq!(backend.header().ni, 6);
        assert_eq!(backend.header().record_length_bytes, 1024);
        assert!(backend.header().little_endian);
    }

    #[test]
    fn supports_parashari_grahas_and_true_nodes() {
        let Some(backend) = test_backend() else {
            return;
        };
        let supported_bodies = [
            CelestialBody::Moon,
            CelestialBody::Sun,
            CelestialBody::Mercury,
            CelestialBody::Venus,
            CelestialBody::Mars,
            CelestialBody::Jupiter,
            CelestialBody::Saturn,
            CelestialBody::Rahu,
            CelestialBody::Ketu,
        ];

        for body in supported_bodies {
            let result = backend.position(
                body,
                J2000_JULIAN_DAY,
                CoordinateFrame::EclipticGeocentric,
                None,
                &EngineConfig::default(),
            );
            assert!(result.is_ok(), "{body:?} must be supported by the DE440 backend");
        }
    }

    #[test]
    fn rejects_mean_node_policy_for_rahu_and_ketu() {
        let Some(backend) = test_backend() else {
            return;
        };
        let config =
            EngineConfig { node_mode: crate::contracts::NodeMode::Mean, ..EngineConfig::default() };

        let rahu_error = backend
            .position(
                CelestialBody::Rahu,
                J2000_JULIAN_DAY,
                CoordinateFrame::EclipticGeocentric,
                None,
                &config,
            )
            .expect_err("mean-node Rahu must remain unsupported");
        assert!(matches!(rahu_error, BackendError::UnsupportedOperation(_)));

        let ketu_error = backend
            .position(
                CelestialBody::Ketu,
                J2000_JULIAN_DAY,
                CoordinateFrame::EclipticGeocentric,
                None,
                &config,
            )
            .expect_err("mean-node Ketu must remain unsupported");
        assert!(matches!(ketu_error, BackendError::UnsupportedOperation(_)));
    }

    #[test]
    fn requires_geocentric_ecliptic_frame() {
        let Some(backend) = test_backend() else {
            return;
        };

        let error = backend
            .position(
                CelestialBody::Moon,
                J2000_JULIAN_DAY,
                CoordinateFrame::EquatorialGeocentric,
                None,
                &EngineConfig::default(),
            )
            .expect_err("equatorial frame must be unsupported");
        assert!(matches!(error, BackendError::UnsupportedOperation(_)));
    }

    #[test]
    fn requires_geocenter_observer() {
        let Some(backend) = test_backend() else {
            return;
        };
        let observer = Observer {
            geo: crate::contracts::GeolocationInput {
                latitude_deg: 0.0,
                longitude_deg: 0.0,
                elevation_m: Some(0.0),
            },
        };

        let error = backend
            .position(
                CelestialBody::Moon,
                J2000_JULIAN_DAY,
                CoordinateFrame::EclipticGeocentric,
                Some(&observer),
                &EngineConfig::default(),
            )
            .expect_err("topocentric observer must be unsupported");
        assert!(matches!(error, BackendError::UnsupportedOperation(_)));
    }

    #[test]
    fn deflection_toggle_changes_metadata_and_pipeline_option() {
        let Some(backend) = test_backend() else {
            return;
        };
        let jd = J2000_JULIAN_DAY;
        let with_deflection = backend
            .position(
                CelestialBody::Moon,
                jd,
                CoordinateFrame::EclipticGeocentric,
                None,
                &EngineConfig::default(),
            )
            .expect("moon position with deflection must compute");
        let without_deflection = backend
            .position(
                CelestialBody::Moon,
                jd,
                CoordinateFrame::EclipticGeocentric,
                None,
                &EngineConfig { gravitational_deflection: false, ..EngineConfig::default() },
            )
            .expect("moon position without deflection must compute");

        assert!(with_deflection.computation_meta.gravitational_deflection);
        assert!(!without_deflection.computation_meta.gravitational_deflection);
        assert!(
            (with_deflection.position.longitude_deg - without_deflection.position.longitude_deg)
                .abs()
                > 1.0e-9
        );
    }

    fn test_backend() -> Option<De440Backend> {
        let path = de440_kernel::require_de440_kernel()?;

        Some(De440Backend::from_path(path).expect("DE440 backend must load for tests"))
    }
}
