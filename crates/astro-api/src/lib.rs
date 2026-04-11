#![recursion_limit = "256"]

use std::{collections::HashMap, sync::Arc};

use astro_core::{
    time::{julian_day, resolve_datetime_input, utc_julian_day_to_tdb_julian_day},
    AyanamsaModel, BodyComputationMeta, BodyPosition, CelestialBody, ComputationResult,
    CoordinateFrame, DateTimeInput, De440Backend, EngineConfig, EngineMode, EphemerisBackend,
    GeolocationInput, HouseSystem, InMemoryBackend, PositionResult, ResultMetadata,
};
use astro_vedic::{
    lagna_position_from_sidereal_longitude, moon_sidereal_division_from_tropical,
    sidereal_division, sidereal_longitude_deg, vimshottari_dasha, vimshottari_dasha_at,
    whole_sign_houses_from_sidereal_ascendant, LagnaPosition, Nakshatra, Rashi, SiderealDivision,
    VimshottariDasha, WholeSignHouse, LAHIRI_ALGO_ID,
};
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

pub const ENGINE_SEMANTIC_VERSION: &str = "0.17.0";
pub const NODE_POLICY_ID: &str = "true_node_mean_ecliptic_of_date";
pub const CHART_SIDEREAL_SCHEMA_VERSION: &str = "chart_sidereal_v1";
const RETROGRADE_DELTA_DAYS: f64 = 0.5;

#[derive(Clone)]
pub struct ApiState {
    backend: Arc<dyn EphemerisBackend>,
    config: EngineConfig,
    version: String,
}

impl ApiState {
    pub fn new(
        backend: Arc<dyn EphemerisBackend>,
        config: EngineConfig,
        version: impl Into<String>,
    ) -> Self {
        Self { backend, config, version: version.into() }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: String,
}

#[derive(Debug, Deserialize)]
pub struct PositionsRequest {
    pub julian_day: f64,
    pub bodies: Vec<CelestialBody>,
    pub compact: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct PositionsPayload {
    pub positions: Vec<ApiPositionResult>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ApiPositionResult {
    pub position: BodyPosition,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub computation_meta: Option<BodyComputationMeta>,
}

#[derive(Debug, Deserialize)]
pub struct SiderealPositionsRequest {
    pub datetime: DateTimeInput,
    pub geo: GeolocationInput,
    pub ayanamsa: AyanamsaModel,
    pub bodies: Vec<CelestialBody>,
    pub gravitational_deflection: Option<bool>,
    pub compact: Option<bool>,
    pub projection: Option<ChartProjection>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct SiderealPositionsPayload {
    pub positions: Vec<SiderealPositionResult>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct SiderealPositionResult {
    pub body: CelestialBody,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tropical_longitude_deg: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tropical_latitude_deg: Option<f64>,
    pub sidereal_longitude_deg: f64,
    pub longitude_speed_deg_per_day: f64,
    pub retrograde: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance_au: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moon_division: Option<SiderealDivision>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub computation_meta: Option<BodyComputationMeta>,
}

#[derive(Debug, Deserialize)]
pub struct SiderealChartRequest {
    pub datetime: DateTimeInput,
    pub geo: GeolocationInput,
    pub ayanamsa: AyanamsaModel,
    pub gravitational_deflection: Option<bool>,
    pub as_of: Option<DateTimeInput>,
    pub compact: Option<bool>,
    pub projection: Option<ChartProjection>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct SiderealChartPayload {
    pub schema_version: String,
    pub extensions: Map<String, Value>,
    pub summary: ChartSummary,
    pub grahas: Vec<ChartGrahaPositionResult>,
    pub lagna: LagnaPosition,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub houses: Option<Vec<WholeSignHouse>>,
    pub house_system: HouseSystem,
    pub moon_sidereal_longitude_deg: f64,
    pub moon_nakshatra: Nakshatra,
    pub moon_pada: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dasha: Option<ChartDashaSummary>,
    pub node_policy: String,
    pub lahiri_algorithm: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ChartGrahaPositionResult {
    pub body: CelestialBody,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tropical_longitude_deg: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tropical_latitude_deg: Option<f64>,
    pub sidereal_longitude_deg: f64,
    pub longitude_speed_deg_per_day: f64,
    pub sidereal_rashi: Rashi,
    pub whole_sign_house: u8,
    pub house_context: GrahaHouseContext,
    pub retrograde: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance_au: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moon_division: Option<SiderealDivision>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub computation_meta: Option<BodyComputationMeta>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChartDashaSummary {
    pub as_of_utc: chrono::DateTime<Utc>,
    pub birth_nakshatra: Nakshatra,
    pub birth_pada: u8,
    pub current: VimshottariDasha,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ChartSummary {
    pub moon_rashi: Rashi,
    pub lagna_rashi: Rashi,
    pub lagna_lord: CelestialBody,
    pub house_lords: Vec<CelestialBody>,
    pub houses: Vec<HouseOccupancySummary>,
    pub grahas_by_rashi: std::collections::BTreeMap<String, Vec<CelestialBody>>,
    pub dispositors: Vec<GrahaDispositorSummary>,
    pub placement_table: Vec<GrahaPlacementSummary>,
    pub motion: ChartMotionSummary,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ChartProjection {
    Full,
    SiderealOnly,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct HouseOccupancySummary {
    pub house: u8,
    pub occupants: Vec<CelestialBody>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct GrahaDispositorSummary {
    pub body: CelestialBody,
    pub occupied_rashi: Rashi,
    pub dispositor: CelestialBody,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct GrahaHouseContext {
    pub whole_sign_house: u8,
    pub house_lord: CelestialBody,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct GrahaPlacementSummary {
    pub body: CelestialBody,
    pub sidereal_rashi: Rashi,
    pub whole_sign_house: u8,
    pub sign_lord: CelestialBody,
    pub house_context: GrahaHouseContext,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ChartMotionSummary {
    pub retrograde_bodies: Vec<CelestialBody>,
    pub fastest: FastestBodySummary,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct FastestBodySummary {
    pub body: CelestialBody,
    pub longitude_speed_deg_per_day: f64,
}

#[derive(Debug, Deserialize)]
pub struct DashaRequest {
    pub moon_sidereal_longitude_deg: f64,
    pub birth_time_utc_rfc3339: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DashaPayload {
    pub dasha: VimshottariDasha,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorPayload {
    pub error: String,
}

pub fn app_router(state: ApiState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/positions", post(positions))
        .route("/positions/sidereal", post(sidereal_positions))
        .route("/chart/sidereal", post(sidereal_chart))
        .route("/openapi.json", get(openapi_json))
        .route("/dasha", post(dasha))
        .with_state(state)
}

async fn health(State(state): State<ApiState>) -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok", version: state.version })
}

async fn openapi_json() -> Json<Value> {
    Json(openapi_spec())
}

async fn positions(
    State(state): State<ApiState>,
    Json(request): Json<PositionsRequest>,
) -> Result<Json<ComputationResult<PositionsPayload>>, (StatusCode, Json<ErrorPayload>)> {
    let compact = request.compact.unwrap_or(false);
    let mut positions = Vec::with_capacity(request.bodies.len());
    for body in request.bodies {
        let position = state
            .backend
            .position(
                body,
                request.julian_day,
                CoordinateFrame::EclipticGeocentric,
                None,
                &state.config,
            )
            .map_err(error_response)?;
        positions.push(compact_position_result(position, compact));
    }

    Ok(Json(ComputationResult { data: PositionsPayload { positions }, metadata: metadata(&state) }))
}

async fn dasha(
    State(state): State<ApiState>,
    Json(request): Json<DashaRequest>,
) -> Result<Json<ComputationResult<DashaPayload>>, (StatusCode, Json<ErrorPayload>)> {
    let birth_time = chrono::DateTime::parse_from_rfc3339(&request.birth_time_utc_rfc3339)
        .map_err(|err| bad_request(err.to_string()))?
        .with_timezone(&Utc);

    Ok(Json(ComputationResult {
        data: DashaPayload {
            dasha: vimshottari_dasha(request.moon_sidereal_longitude_deg, birth_time),
        },
        metadata: metadata(&state),
    }))
}

async fn sidereal_positions(
    State(state): State<ApiState>,
    Json(request): Json<SiderealPositionsRequest>,
) -> Result<Json<ComputationResult<SiderealPositionsPayload>>, (StatusCode, Json<ErrorPayload>)> {
    let compact = request.compact.unwrap_or(false);
    let projection = request.projection.unwrap_or(ChartProjection::Full);
    let (_, config, positions) = compute_sidereal_positions(
        &state,
        request.datetime,
        request.geo,
        request.ayanamsa,
        request.bodies,
        request.gravitational_deflection,
        projection,
    )
    .map_err(map_sidereal_error)?;

    Ok(Json(ComputationResult {
        data: SiderealPositionsPayload {
            positions: positions
                .into_iter()
                .map(|position| compact_sidereal_position(position, compact))
                .collect(),
        },
        metadata: ResultMetadata {
            gravitational_deflection: config.gravitational_deflection,
            ..metadata(&state)
        },
    }))
}

async fn sidereal_chart(
    State(state): State<ApiState>,
    Json(request): Json<SiderealChartRequest>,
) -> Result<Json<ComputationResult<SiderealChartPayload>>, (StatusCode, Json<ErrorPayload>)> {
    let compact = request.compact.unwrap_or(false);
    let projection = request.projection.unwrap_or(ChartProjection::Full);
    let (jd_tdb, config, grahas) = compute_sidereal_positions(
        &state,
        request.datetime.clone(),
        request.geo.clone(),
        request.ayanamsa,
        parashari_bodies().to_vec(),
        request.gravitational_deflection,
        projection,
    )
    .map_err(map_sidereal_error)?;

    let utc = resolve_datetime_input(&request.datetime)
        .map_err(|error| map_sidereal_error(SiderealRequestError::Time(error)))?;
    let as_of_utc = request
        .as_of
        .as_ref()
        .map(resolve_datetime_input)
        .transpose()
        .map_err(|error| map_sidereal_error(SiderealRequestError::Time(error)))?
        .unwrap_or(utc);
    let jd_utc = julian_day(utc);
    let house_set = state
        .backend
        .houses(jd_utc, request.geo.latitude_deg, request.geo.longitude_deg, config.house_system)
        .map_err(error_response)?;
    let lagna_sidereal_longitude_deg = sidereal_longitude_deg(house_set.ascendant_deg, jd_tdb);
    let lagna = lagna_position_from_sidereal_longitude(lagna_sidereal_longitude_deg);
    let houses = whole_sign_houses_from_sidereal_ascendant(lagna.sidereal_longitude_deg).to_vec();
    let moon = grahas
        .iter()
        .find(|position| position.body == CelestialBody::Moon)
        .expect("moon must exist in chart payload");
    let moon_division = moon.moon_division.expect("moon division must exist in chart payload");
    let moon_sidereal_longitude_deg = moon.sidereal_longitude_deg;
    let moon_nakshatra = moon_division.nakshatra;
    let moon_pada = moon_division.pada.0;
    let dasha = ChartDashaSummary {
        as_of_utc,
        birth_nakshatra: moon_nakshatra,
        birth_pada: moon_pada,
        current: vimshottari_dasha_at(moon_sidereal_longitude_deg, utc, as_of_utc),
    };
    let grahas = grahas
        .into_iter()
        .map(|graha| build_chart_graha_position(graha, lagna.rashi, compact, projection))
        .collect::<Vec<_>>();
    let summary = ChartSummary {
        moon_rashi: moon_division.rashi,
        lagna_rashi: lagna.rashi,
        lagna_lord: rashi_lord(lagna.rashi),
        house_lords: houses.iter().map(|house| rashi_lord(house.rashi)).collect(),
        houses: chart_house_occupancy_summary(&grahas),
        grahas_by_rashi: chart_grahas_by_rashi(&grahas),
        dispositors: chart_dispositors(&grahas),
        placement_table: chart_placement_table(&grahas),
        motion: chart_motion_summary(&grahas),
    };

    Ok(Json(ComputationResult {
        data: SiderealChartPayload {
            schema_version: CHART_SIDEREAL_SCHEMA_VERSION.to_owned(),
            extensions: Map::new(),
            summary,
            grahas,
            lagna,
            houses: (!compact).then_some(houses),
            house_system: config.house_system,
            moon_sidereal_longitude_deg,
            moon_nakshatra,
            moon_pada,
            dasha: (!compact).then_some(dasha),
            node_policy: NODE_POLICY_ID.to_owned(),
            lahiri_algorithm: LAHIRI_ALGO_ID.to_owned(),
        },
        metadata: ResultMetadata {
            gravitational_deflection: config.gravitational_deflection,
            ..metadata(&state)
        },
    }))
}

fn metadata(state: &ApiState) -> ResultMetadata {
    ResultMetadata {
        engine_mode: state.config.mode,
        ayanamsa_used: state.config.ayanamsa,
        house_system: state.config.house_system,
        gravitational_deflection: state.config.gravitational_deflection,
        engine_semantic_version: ENGINE_SEMANTIC_VERSION.to_owned(),
        version: state.version.clone(),
    }
}

fn compute_sidereal_positions(
    state: &ApiState,
    datetime: DateTimeInput,
    geo: GeolocationInput,
    ayanamsa: AyanamsaModel,
    bodies: Vec<CelestialBody>,
    gravitational_deflection: Option<bool>,
    projection: ChartProjection,
) -> Result<(f64, EngineConfig, Vec<SiderealPositionResult>), SiderealRequestError> {
    if ayanamsa != AyanamsaModel::Lahiri {
        return Err(SiderealRequestError::Backend(astro_core::BackendError::UnsupportedAyanamsa(
            ayanamsa,
        )));
    }

    let utc = resolve_datetime_input(&datetime).map_err(SiderealRequestError::Time)?;
    let jd_utc = julian_day(utc);
    let jd_tdb = utc_julian_day_to_tdb_julian_day(jd_utc);
    let config = EngineConfig {
        gravitational_deflection: gravitational_deflection
            .unwrap_or(state.config.gravitational_deflection),
        ..state.config.clone()
    };

    let _geo = geo;

    let mut positions = Vec::with_capacity(bodies.len());
    for body in bodies {
        let tropical = state
            .backend
            .position(body, jd_utc, CoordinateFrame::EclipticGeocentric, None, &config)
            .map_err(SiderealRequestError::Backend)?;
        let motion = body_longitude_motion(state, body, jd_utc, &config)?;
        positions.push(build_sidereal_position(tropical, jd_tdb, motion, projection));
    }

    Ok((jd_tdb, config, positions))
}

fn build_sidereal_position(
    tropical: PositionResult,
    jd_tdb: f64,
    motion: LongitudeMotion,
    projection: ChartProjection,
) -> SiderealPositionResult {
    let sidereal_longitude = sidereal_longitude_deg(tropical.position.longitude_deg, jd_tdb);
    let moon_division = if tropical.position.body == CelestialBody::Moon {
        Some(moon_sidereal_division_from_tropical(tropical.position.longitude_deg, jd_tdb))
    } else {
        None
    };
    let mut computation_meta = tropical.computation_meta;
    computation_meta.ayanamsa_algorithm = Some(LAHIRI_ALGO_ID.to_owned());
    let include_tropical = projection == ChartProjection::Full;

    SiderealPositionResult {
        body: tropical.position.body,
        tropical_longitude_deg: include_tropical.then_some(tropical.position.longitude_deg),
        tropical_latitude_deg: include_tropical.then_some(tropical.position.latitude_deg),
        sidereal_longitude_deg: sidereal_longitude,
        longitude_speed_deg_per_day: motion.speed_deg_per_day,
        retrograde: motion.retrograde,
        distance_au: tropical.position.distance_au,
        moon_division,
        computation_meta: Some(computation_meta),
    }
}

fn compact_sidereal_position(
    mut position: SiderealPositionResult,
    compact: bool,
) -> SiderealPositionResult {
    if compact {
        position.distance_au = None;
        position.moon_division = None;
        position.computation_meta = None;
    }

    position
}

fn compact_position_result(mut position: PositionResult, compact: bool) -> ApiPositionResult {
    if compact {
        position.position.distance_au = None;
        return ApiPositionResult { position: position.position, computation_meta: None };
    }

    ApiPositionResult {
        position: position.position,
        computation_meta: Some(position.computation_meta),
    }
}

fn build_chart_graha_position(
    graha: SiderealPositionResult,
    lagna_rashi: Rashi,
    compact: bool,
    projection: ChartProjection,
) -> ChartGrahaPositionResult {
    let sidereal_rashi = sidereal_division(graha.sidereal_longitude_deg).rashi;
    let whole_sign_house = whole_sign_house_number(lagna_rashi, sidereal_rashi);

    ChartGrahaPositionResult {
        body: graha.body,
        tropical_longitude_deg: if projection == ChartProjection::Full {
            graha.tropical_longitude_deg
        } else {
            None
        },
        tropical_latitude_deg: if projection == ChartProjection::Full {
            graha.tropical_latitude_deg
        } else {
            None
        },
        sidereal_longitude_deg: graha.sidereal_longitude_deg,
        longitude_speed_deg_per_day: graha.longitude_speed_deg_per_day,
        sidereal_rashi,
        whole_sign_house,
        house_context: GrahaHouseContext {
            whole_sign_house,
            house_lord: rashi_lord(sidereal_rashi),
        },
        retrograde: graha.retrograde,
        distance_au: (!compact).then_some(graha.distance_au).flatten(),
        moon_division: (!compact).then_some(graha.moon_division).flatten(),
        computation_meta: if compact { None } else { graha.computation_meta },
    }
}

fn chart_house_occupancy_summary(
    grahas: &[ChartGrahaPositionResult],
) -> Vec<HouseOccupancySummary> {
    (1..=12)
        .map(|house| HouseOccupancySummary {
            house,
            occupants: grahas
                .iter()
                .filter(|graha| graha.whole_sign_house == house)
                .map(|graha| graha.body)
                .collect(),
        })
        .collect()
}

fn chart_grahas_by_rashi(
    grahas: &[ChartGrahaPositionResult],
) -> std::collections::BTreeMap<String, Vec<CelestialBody>> {
    let mut grouped = std::collections::BTreeMap::new();
    for rashi in [
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
    ] {
        let key = serde_json::to_string(&rashi)
            .expect("rashi key must serialize")
            .trim_matches('"')
            .to_owned();
        let bodies = grahas
            .iter()
            .filter(|graha| graha.sidereal_rashi == rashi)
            .map(|graha| graha.body)
            .collect::<Vec<_>>();
        grouped.insert(key, bodies);
    }
    grouped
}

fn chart_dispositors(grahas: &[ChartGrahaPositionResult]) -> Vec<GrahaDispositorSummary> {
    grahas
        .iter()
        .map(|graha| GrahaDispositorSummary {
            body: graha.body,
            occupied_rashi: graha.sidereal_rashi,
            dispositor: rashi_lord(graha.sidereal_rashi),
        })
        .collect()
}

fn chart_placement_table(grahas: &[ChartGrahaPositionResult]) -> Vec<GrahaPlacementSummary> {
    grahas
        .iter()
        .map(|graha| GrahaPlacementSummary {
            body: graha.body,
            sidereal_rashi: graha.sidereal_rashi,
            whole_sign_house: graha.whole_sign_house,
            sign_lord: rashi_lord(graha.sidereal_rashi),
            house_context: GrahaHouseContext {
                whole_sign_house: graha.house_context.whole_sign_house,
                house_lord: graha.house_context.house_lord,
            },
        })
        .collect()
}

fn chart_motion_summary(grahas: &[ChartGrahaPositionResult]) -> ChartMotionSummary {
    let retrograde_bodies =
        grahas.iter().filter(|graha| graha.retrograde).map(|graha| graha.body).collect::<Vec<_>>();
    let fastest_graha = grahas.iter().skip(1).fold(&grahas[0], |fastest, candidate| {
        if candidate.longitude_speed_deg_per_day.abs() > fastest.longitude_speed_deg_per_day.abs() {
            candidate
        } else {
            fastest
        }
    });
    let fastest = FastestBodySummary {
        body: fastest_graha.body,
        longitude_speed_deg_per_day: fastest_graha.longitude_speed_deg_per_day,
    };

    ChartMotionSummary { retrograde_bodies, fastest }
}

fn whole_sign_house_number(lagna_rashi: Rashi, graha_rashi: Rashi) -> u8 {
    let lagna_sign_index = rashi_index(lagna_rashi);
    let graha_sign_index = rashi_index(graha_rashi);

    ((graha_sign_index + 12 - lagna_sign_index) % 12 + 1) as u8
}

fn rashi_index(rashi: Rashi) -> usize {
    match rashi {
        Rashi::Mesha => 0,
        Rashi::Vrishabha => 1,
        Rashi::Mithuna => 2,
        Rashi::Karka => 3,
        Rashi::Simha => 4,
        Rashi::Kanya => 5,
        Rashi::Tula => 6,
        Rashi::Vrischika => 7,
        Rashi::Dhanu => 8,
        Rashi::Makara => 9,
        Rashi::Kumbha => 10,
        Rashi::Meena => 11,
    }
}

fn body_longitude_motion(
    state: &ApiState,
    body: CelestialBody,
    jd_utc: f64,
    config: &EngineConfig,
) -> Result<LongitudeMotion, SiderealRequestError> {
    let previous = state
        .backend
        .position(
            body,
            jd_utc - RETROGRADE_DELTA_DAYS,
            CoordinateFrame::EclipticGeocentric,
            None,
            config,
        )
        .map_err(SiderealRequestError::Backend)?;
    let next = state
        .backend
        .position(
            body,
            jd_utc + RETROGRADE_DELTA_DAYS,
            CoordinateFrame::EclipticGeocentric,
            None,
            config,
        )
        .map_err(SiderealRequestError::Backend)?;
    let delta =
        signed_longitude_delta_deg(previous.position.longitude_deg, next.position.longitude_deg);
    let speed_deg_per_day = delta / (RETROGRADE_DELTA_DAYS * 2.0);
    Ok(LongitudeMotion { speed_deg_per_day, retrograde: speed_deg_per_day < 0.0 })
}

fn signed_longitude_delta_deg(start_deg: f64, end_deg: f64) -> f64 {
    (end_deg - start_deg + 540.0).rem_euclid(360.0) - 180.0
}

#[derive(Debug, Clone, Copy)]
struct LongitudeMotion {
    speed_deg_per_day: f64,
    retrograde: bool,
}

fn rashi_lord(rashi: Rashi) -> CelestialBody {
    match rashi {
        Rashi::Mesha | Rashi::Vrischika => CelestialBody::Mars,
        Rashi::Vrishabha | Rashi::Tula => CelestialBody::Venus,
        Rashi::Mithuna | Rashi::Kanya => CelestialBody::Mercury,
        Rashi::Karka => CelestialBody::Moon,
        Rashi::Simha => CelestialBody::Sun,
        Rashi::Dhanu | Rashi::Meena => CelestialBody::Jupiter,
        Rashi::Makara | Rashi::Kumbha => CelestialBody::Saturn,
    }
}

pub fn openapi_spec() -> Value {
    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Daanyam Astro Engine API",
            "version": ENGINE_SEMANTIC_VERSION
        },
        "paths": {
            "/positions": {
                "post": {
                    "summary": "Compute tropical positions",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/PositionsRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Tropical positions",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/PositionsResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/positions/sidereal": {
                "post": {
                    "summary": "Compute Lahiri sidereal positions",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/SiderealPositionsRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Sidereal positions",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/SiderealPositionsResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/chart/sidereal": {
                "post": {
                    "summary": "Compute a sidereal chart payload",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/SiderealChartRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Sidereal chart",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/SiderealChartResponse" }
                                }
                            }
                        }
                    }
                }
            }
        },
        "components": {
            "schemas": {
                "PositionsRequest": {
                    "type": "object",
                    "required": ["julian_day", "bodies"],
                    "properties": {
                        "julian_day": { "type": "number" },
                        "bodies": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/CelestialBody" }
                        },
                        "compact": {
                            "type": "boolean",
                            "description": "When true, omits position distance and computation metadata for lightweight clients."
                        }
                    }
                },
                "SiderealPositionsRequest": {
                    "type": "object",
                    "required": ["datetime", "geo", "ayanamsa", "bodies"],
                    "properties": {
                        "datetime": { "$ref": "#/components/schemas/DateTimeInput" },
                        "geo": { "$ref": "#/components/schemas/GeolocationInput" },
                        "ayanamsa": { "type": "string", "enum": ["lahiri", "raman", "krishnamurti", "custom"] },
                        "bodies": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/CelestialBody" }
                        },
                        "gravitational_deflection": { "type": "boolean" },
                        "compact": {
                            "type": "boolean",
                            "description": "When true, omits distance_au, moon_division, and computation_meta in each position."
                        },
                        "projection": {
                            "type": "string",
                            "enum": ["full", "sidereal_only"],
                            "description": "When sidereal_only, omits tropical-only body coordinate fields while keeping sidereal positions and motion fields."
                        }
                    }
                },
                "SiderealChartRequest": {
                    "type": "object",
                    "required": ["datetime", "geo", "ayanamsa"],
                    "properties": {
                        "datetime": { "$ref": "#/components/schemas/DateTimeInput" },
                        "geo": { "$ref": "#/components/schemas/GeolocationInput" },
                        "ayanamsa": { "type": "string", "enum": ["lahiri", "raman", "krishnamurti", "custom"] },
                        "gravitational_deflection": { "type": "boolean" },
                        "as_of": { "$ref": "#/components/schemas/DateTimeInput" },
                        "compact": {
                            "type": "boolean",
                            "description": "When true, omits houses, dasha, and heavy per-graha metadata fields."
                        },
                        "projection": {
                            "type": "string",
                            "enum": ["full", "sidereal_only"],
                            "description": "When sidereal_only, omits tropical-only graha coordinate fields while keeping sidereal chart integrity."
                        }
                    }
                },
                "DateTimeInput": {
                    "oneOf": [
                        {
                            "type": "object",
                            "required": ["kind", "utc"],
                            "properties": {
                                "kind": { "const": "utc" },
                                "utc": { "type": "string", "format": "date-time" }
                            }
                        },
                        {
                            "type": "object",
                            "required": ["kind", "local", "timezone"],
                            "properties": {
                                "kind": { "const": "local" },
                                "local": { "type": "string", "format": "date-time" },
                                "timezone": { "type": "string" }
                            }
                        },
                        {
                            "type": "object",
                            "required": ["kind", "datetime"],
                            "properties": {
                                "kind": { "const": "offset" },
                                "datetime": { "type": "string", "format": "date-time" }
                            }
                        }
                    ]
                },
                "GeolocationInput": {
                    "type": "object",
                    "required": ["latitude_deg", "longitude_deg"],
                    "properties": {
                        "latitude_deg": { "type": "number" },
                        "longitude_deg": { "type": "number" },
                        "elevation_m": { "type": ["number", "null"] }
                    }
                },
                "CelestialBody": {
                    "type": "string",
                    "enum": ["sun", "moon", "mercury", "venus", "mars", "jupiter", "saturn", "rahu", "ketu"]
                },
                "PositionsResponse": {
                    "type": "object",
                    "properties": {
                        "data": {
                            "type": "object",
                            "properties": {
                                "positions": {
                                    "type": "array",
                                    "items": { "$ref": "#/components/schemas/ApiPositionResult" }
                                }
                            }
                        },
                        "metadata": { "$ref": "#/components/schemas/ResultMetadata" }
                    }
                },
                "SiderealPositionsResponse": {
                    "type": "object",
                    "properties": {
                        "data": {
                            "type": "object",
                            "properties": {
                                "positions": {
                                    "type": "array",
                                    "items": { "$ref": "#/components/schemas/SiderealPositionResult" }
                                }
                            }
                        },
                        "metadata": { "$ref": "#/components/schemas/ResultMetadata" }
                    }
                },
                "SiderealChartResponse": {
                    "type": "object",
                    "properties": {
                        "data": { "$ref": "#/components/schemas/SiderealChartPayload" },
                        "metadata": { "$ref": "#/components/schemas/ResultMetadata" }
                    }
                },
                "PositionResult": {
                    "type": "object",
                    "properties": {
                        "position": { "$ref": "#/components/schemas/BodyPosition" },
                        "computation_meta": { "$ref": "#/components/schemas/BodyComputationMeta" }
                    }
                },
                "ApiPositionResult": {
                    "type": "object",
                    "properties": {
                        "position": { "$ref": "#/components/schemas/BodyPosition" },
                        "computation_meta": {
                            "oneOf": [
                                { "$ref": "#/components/schemas/BodyComputationMeta" },
                                { "type": "null" }
                            ],
                            "description": "Null when compact=true."
                        }
                    }
                },
                "SiderealPositionResult": {
                    "type": "object",
                    "properties": {
                        "body": { "$ref": "#/components/schemas/CelestialBody" },
                        "tropical_longitude_deg": {
                            "type": ["number", "null"],
                            "description": "Omitted in JSON when projection=sidereal_only."
                        },
                        "tropical_latitude_deg": {
                            "type": ["number", "null"],
                            "description": "Omitted in JSON when projection=sidereal_only."
                        },
                        "sidereal_longitude_deg": { "type": "number" },
                        "longitude_speed_deg_per_day": { "type": "number" },
                        "retrograde": { "type": "boolean" },
                        "distance_au": {
                            "type": ["number", "null"],
                            "description": "Null when compact=true."
                        },
                        "moon_division": {
                            "oneOf": [
                                { "$ref": "#/components/schemas/SiderealDivision" },
                                { "type": "null" }
                            ],
                            "description": "Null when compact=true or for non-Moon bodies."
                        },
                        "computation_meta": {
                            "oneOf": [
                                { "$ref": "#/components/schemas/BodyComputationMeta" },
                                { "type": "null" }
                            ],
                            "description": "Null when compact=true."
                        }
                    }
                },
                "SiderealChartPayload": {
                    "type": "object",
                    "properties": {
                        "schema_version": { "type": "string" },
                        "extensions": { "type": "object" },
                        "summary": { "$ref": "#/components/schemas/ChartSummary" },
                        "grahas": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/ChartGrahaPositionResult" }
                        },
                        "lagna": { "$ref": "#/components/schemas/LagnaPosition" },
                        "houses": {
                            "oneOf": [
                                {
                                    "type": "array",
                                    "items": { "$ref": "#/components/schemas/WholeSignHouse" }
                                },
                                { "type": "null" }
                            ],
                            "description": "Null when compact=true."
                        },
                        "house_system": { "type": "string" },
                        "moon_sidereal_longitude_deg": { "type": "number" },
                        "moon_nakshatra": { "type": "string" },
                        "moon_pada": { "type": "integer" },
                        "dasha": {
                            "oneOf": [
                                { "$ref": "#/components/schemas/ChartDashaSummary" },
                                { "type": "null" }
                            ],
                            "description": "Null when compact=true."
                        },
                        "node_policy": { "type": "string" },
                        "lahiri_algorithm": { "type": "string" }
                    }
                },
                "ChartGrahaPositionResult": {
                    "type": "object",
                    "properties": {
                        "body": { "$ref": "#/components/schemas/CelestialBody" },
                        "tropical_longitude_deg": {
                            "type": ["number", "null"],
                            "description": "Omitted in JSON when projection=sidereal_only."
                        },
                        "tropical_latitude_deg": {
                            "type": ["number", "null"],
                            "description": "Omitted in JSON when projection=sidereal_only."
                        },
                        "sidereal_longitude_deg": { "type": "number" },
                        "longitude_speed_deg_per_day": { "type": "number" },
                        "sidereal_rashi": { "type": "string" },
                        "whole_sign_house": { "type": "integer" },
                        "house_context": { "$ref": "#/components/schemas/GrahaHouseContext" },
                        "retrograde": { "type": "boolean" },
                        "distance_au": {
                            "type": ["number", "null"],
                            "description": "Null when compact=true."
                        },
                        "moon_division": {
                            "oneOf": [
                                { "$ref": "#/components/schemas/SiderealDivision" },
                                { "type": "null" }
                            ],
                            "description": "Null when compact=true or for non-Moon bodies."
                        },
                        "computation_meta": {
                            "oneOf": [
                                { "$ref": "#/components/schemas/BodyComputationMeta" },
                                { "type": "null" }
                            ],
                            "description": "Null when compact=true."
                        }
                    }
                },
                "ChartSummary": {
                    "type": "object",
                    "properties": {
                        "moon_rashi": { "type": "string" },
                        "lagna_rashi": { "type": "string" },
                        "lagna_lord": { "$ref": "#/components/schemas/CelestialBody" },
                        "house_lords": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/CelestialBody" }
                        },
                        "houses": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/HouseOccupancySummary" }
                        },
                        "grahas_by_rashi": {
                            "type": "object",
                            "additionalProperties": {
                                "type": "array",
                                "items": { "$ref": "#/components/schemas/CelestialBody" }
                            }
                        },
                        "dispositors": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/GrahaDispositorSummary" }
                        },
                        "placement_table": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/GrahaPlacementSummary" }
                        },
                        "motion": { "$ref": "#/components/schemas/ChartMotionSummary" }
                    }
                },
                "GrahaDispositorSummary": {
                    "type": "object",
                    "properties": {
                        "body": { "$ref": "#/components/schemas/CelestialBody" },
                        "occupied_rashi": { "type": "string" },
                        "dispositor": { "$ref": "#/components/schemas/CelestialBody" }
                    }
                },
                "GrahaHouseContext": {
                    "type": "object",
                    "properties": {
                        "whole_sign_house": { "type": "integer" },
                        "house_lord": { "$ref": "#/components/schemas/CelestialBody" }
                    }
                },
                "GrahaPlacementSummary": {
                    "type": "object",
                    "properties": {
                        "body": { "$ref": "#/components/schemas/CelestialBody" },
                        "sidereal_rashi": { "type": "string" },
                        "whole_sign_house": { "type": "integer" },
                        "sign_lord": { "$ref": "#/components/schemas/CelestialBody" },
                        "house_context": { "$ref": "#/components/schemas/GrahaHouseContext" }
                    }
                },
                "HouseOccupancySummary": {
                    "type": "object",
                    "properties": {
                        "house": { "type": "integer" },
                        "occupants": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/CelestialBody" }
                        }
                    }
                },
                "ChartMotionSummary": {
                    "type": "object",
                    "properties": {
                        "retrograde_bodies": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/CelestialBody" }
                        },
                        "fastest": { "$ref": "#/components/schemas/FastestBodySummary" }
                    }
                },
                "FastestBodySummary": {
                    "type": "object",
                    "properties": {
                        "body": { "$ref": "#/components/schemas/CelestialBody" },
                        "longitude_speed_deg_per_day": { "type": "number" }
                    }
                },
                "SiderealDivision": {
                    "type": "object",
                    "properties": {
                        "rashi": { "type": "string" },
                        "nakshatra": { "type": "string" },
                        "pada": { "type": "integer" }
                    }
                },
                "LagnaPosition": {
                    "type": "object",
                    "properties": {
                        "rashi": { "type": "string" },
                        "sidereal_longitude_deg": { "type": "number" }
                    }
                },
                "WholeSignHouse": {
                    "type": "object",
                    "properties": {
                        "house": { "type": "integer" },
                        "rashi": { "type": "string" },
                        "cusp_sidereal_longitude_deg": { "type": "number" }
                    }
                },
                "ChartDashaSummary": {
                    "type": "object",
                    "properties": {
                        "as_of_utc": { "type": "string", "format": "date-time" },
                        "birth_nakshatra": { "type": "string" },
                        "birth_pada": { "type": "integer" },
                        "current": { "$ref": "#/components/schemas/VimshottariDasha" }
                    }
                },
                "VimshottariDasha": {
                    "type": "object",
                    "properties": {
                        "maha": { "$ref": "#/components/schemas/DashaPeriod" },
                        "antar": { "$ref": "#/components/schemas/DashaPeriod" },
                        "pratyantar": { "$ref": "#/components/schemas/DashaPeriod" }
                    }
                },
                "DashaPeriod": {
                    "type": "object",
                    "properties": {
                        "lord": { "type": "string" },
                        "start": { "type": "string", "format": "date-time" },
                        "end": { "type": "string", "format": "date-time" }
                    }
                },
                "BodyPosition": {
                    "type": "object",
                    "properties": {
                        "body": { "$ref": "#/components/schemas/CelestialBody" },
                        "longitude_deg": { "type": "number" },
                        "latitude_deg": { "type": "number" },
                        "distance_au": {
                            "type": ["number", "null"],
                            "description": "Null when compact=true in /positions."
                        },
                        "frame": { "type": "string" }
                    }
                },
                "BodyComputationMeta": {
                    "type": "object",
                    "properties": {
                        "frame": { "type": "string" },
                        "observer": { "type": "string" },
                        "topocentric_applied": { "type": "boolean" },
                        "kernel": { "type": "string" },
                        "kernel_notes": { "type": ["string", "null"] },
                        "crate_version": { "type": "string" },
                        "light_time": { "type": "boolean" },
                        "stellar_aberration": { "type": "boolean" },
                        "gravitational_deflection": { "type": "boolean" },
                        "motion_model": { "type": ["string", "null"] },
                        "node_policy": { "type": ["string", "null"] },
                        "ayanamsa_algorithm": { "type": ["string", "null"] }
                    }
                },
                "ResultMetadata": {
                    "type": "object",
                    "properties": {
                        "engine_mode": { "type": "string" },
                        "ayanamsa_used": { "type": "string" },
                        "house_system": { "type": "string" },
                        "gravitational_deflection": { "type": "boolean" },
                        "engine_semantic_version": { "type": "string" },
                        "version": { "type": "string" }
                    }
                }
            }
        }
    })
}

fn parashari_bodies() -> [CelestialBody; 9] {
    [
        CelestialBody::Sun,
        CelestialBody::Moon,
        CelestialBody::Mars,
        CelestialBody::Mercury,
        CelestialBody::Jupiter,
        CelestialBody::Venus,
        CelestialBody::Saturn,
        CelestialBody::Rahu,
        CelestialBody::Ketu,
    ]
}

#[derive(Debug)]
enum SiderealRequestError {
    Backend(astro_core::BackendError),
    Time(astro_core::time::TimeError),
}

fn map_sidereal_error(error: SiderealRequestError) -> (StatusCode, Json<ErrorPayload>) {
    match error {
        SiderealRequestError::Backend(error) => error_response(error),
        SiderealRequestError::Time(error) => bad_request(error.to_string()),
    }
}

fn error_response(error: astro_core::BackendError) -> (StatusCode, Json<ErrorPayload>) {
    bad_request(error.to_string())
}

fn bad_request(error: String) -> (StatusCode, Json<ErrorPayload>) {
    (StatusCode::BAD_REQUEST, Json(ErrorPayload { error }))
}

pub fn demo_state() -> ApiState {
    let mut positions = HashMap::new();
    for (body, longitude_deg, latitude_deg) in [
        (CelestialBody::Sun, 280.0, 0.0),
        (CelestialBody::Moon, 123.45, 4.56),
        (CelestialBody::Mars, 210.0, -1.2),
        (CelestialBody::Mercury, 145.0, 1.1),
        (CelestialBody::Jupiter, 25.0, -0.8),
        (CelestialBody::Venus, 241.0, 2.0),
        (CelestialBody::Saturn, 344.0, -1.6),
        (CelestialBody::Rahu, 15.0, 0.0),
        (CelestialBody::Ketu, 195.0, 0.0),
    ] {
        positions.insert(
            body,
            BodyPosition {
                body,
                longitude_deg,
                latitude_deg,
                distance_au: None,
                frame: CoordinateFrame::EclipticGeocentric,
            },
        );
    }

    let backend = InMemoryBackend::new(positions, 24.0);
    ApiState::new(Arc::new(backend), EngineConfig::default(), env!("CARGO_PKG_VERSION"))
}

pub fn de440_state_from_env() -> Result<ApiState, astro_core::BackendError> {
    let backend = De440Backend::from_env()?;
    Ok(ApiState::new(Arc::new(backend), EngineConfig::default(), env!("CARGO_PKG_VERSION")))
}

pub fn example_sidereal_division(longitude_deg: f64) -> SiderealDivision {
    sidereal_division(longitude_deg)
}

pub fn example_engine_mode() -> EngineMode {
    EngineMode::Vedic
}

pub fn example_ayanamsa() -> AyanamsaModel {
    AyanamsaModel::Lahiri
}

pub fn example_house_system() -> HouseSystem {
    HouseSystem::WholeSign
}

#[cfg(test)]
mod tests {
    use axum::{
        body::to_bytes,
        body::Body,
        http::{Method, Request, StatusCode},
    };
    use serde_json::Value;
    use tower::util::ServiceExt;

    use super::*;

    #[tokio::test]
    async fn health_endpoint_reports_version() {
        let app = app_router(demo_state());
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/health")
                    .body(Body::empty())
                    .expect("request must build"),
            )
            .await
            .expect("response must succeed");

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn positions_endpoint_returns_metadata() {
        let app = app_router(demo_state());
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/positions")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"julian_day":2451545.0,"bodies":["moon"]}"#))
                    .expect("request must build"),
            )
            .await
            .expect("response must succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.expect("body must be readable");
        let json: Value = serde_json::from_slice(&body).expect("body must be valid json");
        assert_eq!(json["metadata"]["engine_mode"], "vedic");
        assert_eq!(json["metadata"]["ayanamsa_used"], "lahiri");
        assert_eq!(json["metadata"]["house_system"], "whole_sign");
        assert_eq!(json["metadata"]["gravitational_deflection"], true);
        assert_eq!(json["metadata"]["engine_semantic_version"], ENGINE_SEMANTIC_VERSION);
        assert_eq!(
            json["data"]["positions"][0]["computation_meta"]["gravitational_deflection"],
            true
        );
    }

    #[tokio::test]
    async fn sidereal_positions_endpoint_returns_sidereal_payload() {
        let app = app_router(demo_state());
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/positions/sidereal")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"datetime":{"kind":"utc","utc":"2000-01-01T12:00:00Z"},"geo":{"latitude_deg":12.97,"longitude_deg":77.59,"elevation_m":920.0},"ayanamsa":"lahiri","bodies":["moon"],"gravitational_deflection":false}"#,
                    ))
                    .expect("request must build"),
            )
            .await
            .expect("response must succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.expect("body must be readable");
        let json: Value = serde_json::from_slice(&body).expect("body must be valid json");
        assert_eq!(json["metadata"]["gravitational_deflection"], false);
        assert_eq!(
            json["data"]["positions"][0]["computation_meta"]["ayanamsa_algorithm"],
            LAHIRI_ALGO_ID
        );
        assert!(json["data"]["positions"][0]["sidereal_longitude_deg"].is_number());
        assert!(json["data"]["positions"][0]["tropical_longitude_deg"].is_number());
        assert_eq!(json["data"]["positions"][0]["longitude_speed_deg_per_day"], 0.0);
        assert_eq!(json["data"]["positions"][0]["retrograde"], false);
        assert_eq!(json["data"]["positions"][0]["computation_meta"]["topocentric_applied"], false);
    }

    #[tokio::test]
    async fn sidereal_positions_rejects_unsupported_ayanamsa() {
        let app = app_router(demo_state());
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/positions/sidereal")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"datetime":{"kind":"utc","utc":"2000-01-01T12:00:00Z"},"geo":{"latitude_deg":12.97,"longitude_deg":77.59,"elevation_m":920.0},"ayanamsa":"raman","bodies":["moon"]}"#,
                    ))
                    .expect("request must build"),
            )
            .await
            .expect("response must succeed");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn sidereal_chart_endpoint_returns_parashari_payload() {
        let app = app_router(demo_state());
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/chart/sidereal")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"datetime":{"kind":"utc","utc":"2000-01-01T12:00:00Z"},"geo":{"latitude_deg":12.97,"longitude_deg":77.59,"elevation_m":920.0},"ayanamsa":"lahiri","gravitational_deflection":false}"#,
                    ))
                    .expect("request must build"),
            )
            .await
            .expect("response must succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.expect("body must be readable");
        let json: Value = serde_json::from_slice(&body).expect("body must be valid json");
        assert_eq!(json["data"]["grahas"].as_array().expect("grahas must exist").len(), 9);
        assert_eq!(json["data"]["schema_version"], CHART_SIDEREAL_SCHEMA_VERSION);
        assert_eq!(json["data"]["extensions"], serde_json::json!({}));
        assert!(json["data"]["lagna"]["sidereal_longitude_deg"].is_number());
        assert!(json["data"]["lagna"]["rashi"].is_string());
        assert_eq!(json["data"]["houses"].as_array().expect("houses must exist").len(), 12);
        assert_eq!(json["data"]["house_system"], "whole_sign");
        assert_eq!(json["data"]["summary"]["lagna_lord"], "jupiter");
        assert_eq!(json["data"]["summary"]["house_lords"][0], "jupiter");
        assert_eq!(json["data"]["summary"]["houses"][0]["house"], 1);
        assert_eq!(json["data"]["summary"]["houses"][0]["occupants"], serde_json::json!(["rahu"]));
        assert_eq!(json["data"]["summary"]["grahas_by_rashi"]["dhanu"], serde_json::json!(["sun"]));
        assert_eq!(
            json["data"]["summary"]["dispositors"][0]["occupied_rashi"],
            json["data"]["grahas"][0]["sidereal_rashi"]
        );
        assert_eq!(json["data"]["summary"]["placement_table"][0]["body"], "sun");
        assert_eq!(json["data"]["summary"]["motion"]["fastest"]["body"], "sun");
        assert_eq!(
            json["data"]["summary"]["motion"]["fastest"]["longitude_speed_deg_per_day"],
            0.0
        );
        assert_eq!(
            json["data"]["summary"]["motion"]["retrograde_bodies"]
                .as_array()
                .expect("retrograde_bodies must exist")
                .len(),
            0
        );
        assert!(json["data"]["grahas"][0]["sidereal_rashi"].is_string());
        assert!(json["data"]["grahas"][0]["whole_sign_house"].is_number());
        assert!(json["data"]["grahas"][0]["house_context"]["whole_sign_house"].is_number());
        assert_eq!(json["data"]["grahas"][0]["longitude_speed_deg_per_day"], 0.0);
        assert_eq!(json["data"]["grahas"][0]["retrograde"], false);
        assert_eq!(json["data"]["dasha"]["as_of_utc"], "2000-01-01T12:00:00Z");
        assert_eq!(json["data"]["dasha"]["birth_nakshatra"], "pushya");
        assert_eq!(json["data"]["dasha"]["birth_pada"], 2);
        assert_eq!(json["data"]["dasha"]["current"]["maha"]["lord"], "saturn");
        assert_eq!(json["data"]["dasha"]["current"]["antar"]["lord"], "saturn");
        assert_eq!(json["data"]["dasha"]["current"]["pratyantar"]["lord"], "saturn");
        assert_eq!(json["data"]["node_policy"], NODE_POLICY_ID);
        assert_eq!(json["data"]["lahiri_algorithm"], LAHIRI_ALGO_ID);
        assert_eq!(json["metadata"]["engine_semantic_version"], ENGINE_SEMANTIC_VERSION);
    }

    #[tokio::test]
    async fn sidereal_chart_compact_mode_omits_heavy_fields() {
        let app = app_router(demo_state());
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/chart/sidereal")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"datetime":{"kind":"utc","utc":"2000-01-01T12:00:00Z"},"geo":{"latitude_deg":12.97,"longitude_deg":77.59,"elevation_m":920.0},"ayanamsa":"lahiri","compact":true}"#,
                    ))
                    .expect("request must build"),
            )
            .await
            .expect("response must succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.expect("body must be readable");
        let json: Value = serde_json::from_slice(&body).expect("body must be valid json");

        assert!(json["data"]["summary"].is_object());
        assert!(json["data"]["summary"]["houses"].is_array());
        assert!(json["data"]["houses"].is_null());
        assert!(json["data"]["dasha"].is_null());
        assert!(json["data"]["grahas"][0]["computation_meta"].is_null());
        assert!(json["data"]["grahas"][0]["moon_division"].is_null());
    }

    #[tokio::test]
    async fn sidereal_chart_sidereal_only_projection_omits_tropical_fields() {
        let app = app_router(demo_state());
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/chart/sidereal")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"datetime":{"kind":"utc","utc":"2000-01-01T12:00:00Z"},"geo":{"latitude_deg":12.97,"longitude_deg":77.59,"elevation_m":920.0},"ayanamsa":"lahiri","compact":true,"projection":"sidereal_only"}"#,
                    ))
                    .expect("request must build"),
            )
            .await
            .expect("response must succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.expect("body must be readable");
        let json: Value = serde_json::from_slice(&body).expect("body must be valid json");

        assert!(json["data"]["grahas"][0]["sidereal_longitude_deg"].is_number());
        assert!(json["data"]["grahas"][0]["tropical_longitude_deg"].is_null());
        assert!(json["data"]["grahas"][0]["tropical_latitude_deg"].is_null());
        assert!(json["data"]["summary"]["houses"].is_array());
    }

    #[tokio::test]
    async fn sidereal_positions_compact_mode_omits_heavy_fields() {
        let app = app_router(demo_state());
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/positions/sidereal")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"datetime":{"kind":"utc","utc":"2000-01-01T12:00:00Z"},"geo":{"latitude_deg":12.97,"longitude_deg":77.59,"elevation_m":920.0},"ayanamsa":"lahiri","bodies":["moon"],"compact":true}"#,
                    ))
                    .expect("request must build"),
            )
            .await
            .expect("response must succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.expect("body must be readable");
        let json: Value = serde_json::from_slice(&body).expect("body must be valid json");

        assert_eq!(json["data"]["positions"][0]["longitude_speed_deg_per_day"], 0.0);
        assert!(json["data"]["positions"][0]["distance_au"].is_null());
        assert!(json["data"]["positions"][0]["moon_division"].is_null());
        assert!(json["data"]["positions"][0]["computation_meta"].is_null());
    }

    #[tokio::test]
    async fn sidereal_positions_sidereal_only_projection_omits_tropical_fields() {
        let app = app_router(demo_state());
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/positions/sidereal")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"datetime":{"kind":"utc","utc":"2000-01-01T12:00:00Z"},"geo":{"latitude_deg":12.97,"longitude_deg":77.59,"elevation_m":920.0},"ayanamsa":"lahiri","bodies":["moon"],"compact":true,"projection":"sidereal_only"}"#,
                    ))
                    .expect("request must build"),
            )
            .await
            .expect("response must succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.expect("body must be readable");
        let json: Value = serde_json::from_slice(&body).expect("body must be valid json");

        assert!(json["data"]["positions"][0]["sidereal_longitude_deg"].is_number());
        assert!(json["data"]["positions"][0]["tropical_longitude_deg"].is_null());
        assert!(json["data"]["positions"][0]["tropical_latitude_deg"].is_null());
    }

    #[tokio::test]
    async fn openapi_json_route_exposes_current_routes() {
        let app = app_router(demo_state());
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/openapi.json")
                    .body(Body::empty())
                    .expect("request must build"),
            )
            .await
            .expect("response must succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.expect("body must be readable");
        let json: Value = serde_json::from_slice(&body).expect("body must be valid json");

        assert_eq!(json["openapi"], "3.1.0");
        assert!(json["paths"]["/chart/sidereal"]["post"].is_object());
        assert!(json["paths"]["/positions"]["post"].is_object());
        assert!(json["paths"]["/positions/sidereal"]["post"].is_object());
        assert!(json["components"]["schemas"]["SiderealChartPayload"]["properties"]["dasha"]
            .is_object());
        assert!(json["components"]["schemas"]["SiderealChartPayload"]["properties"]["summary"]
            .is_object());
        assert!(json["components"]["schemas"]["ChartSummary"]["properties"]["motion"].is_object());
        assert!(json["components"]["schemas"]["ChartSummary"]["properties"]["houses"].is_object());
        assert!(json["components"]["schemas"]["ChartSummary"]["properties"]["grahas_by_rashi"]
            .is_object());
        assert!(
            json["components"]["schemas"]["ChartSummary"]["properties"]["dispositors"].is_object()
        );
        assert!(json["components"]["schemas"]["ChartSummary"]["properties"]["placement_table"]
            .is_object());
        assert!(json["components"]["schemas"]["GrahaDispositorSummary"]["properties"]
            ["dispositor"]
            .is_object());
        assert!(json["components"]["schemas"]["GrahaPlacementSummary"]["properties"]["sign_lord"]
            .is_object());
        assert!(json["components"]["schemas"]["GrahaHouseContext"]["properties"]["house_lord"]
            .is_object());
        assert!(json["components"]["schemas"]["HouseOccupancySummary"]["properties"]["occupants"]
            .is_object());
        assert!(json["components"]["schemas"]["ChartMotionSummary"]["properties"]["fastest"]
            .is_object());
        assert!(json["components"]["schemas"]["SiderealPositionsRequest"]["properties"]
            ["projection"]
            .is_object());
        assert!(json["components"]["schemas"]["ChartGrahaPositionResult"]["properties"]
            ["retrograde"]
            .is_object());
        assert!(json["components"]["schemas"]["ChartGrahaPositionResult"]["properties"]
            ["longitude_speed_deg_per_day"]
            .is_object());
        assert!(json["components"]["schemas"]["ChartGrahaPositionResult"]["properties"]
            ["tropical_longitude_deg"]
            .is_object());
        assert!(json["components"]["schemas"]["ChartGrahaPositionResult"]["properties"]
            ["house_context"]
            .is_object());
        assert!(json["components"]["schemas"]["SiderealPositionResult"]["properties"]
            ["tropical_longitude_deg"]
            .is_object());
    }

    #[tokio::test]
    async fn dasha_endpoint_returns_deterministic_payload() {
        let app = app_router(demo_state());
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/dasha")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"moon_sidereal_longitude_deg":15.0,"birth_time_utc_rfc3339":"2024-01-01T00:00:00Z"}"#,
                    ))
                    .expect("request must build"),
            )
            .await
            .expect("response must succeed");

        assert_eq!(response.status(), StatusCode::OK);
    }
}
