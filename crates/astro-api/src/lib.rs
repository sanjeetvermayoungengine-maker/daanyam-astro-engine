#![recursion_limit = "256"]

mod metrics;

use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use astro_core::{
    time::{julian_day, resolve_datetime_input, utc_julian_day_to_tdb_julian_day},
    AyanamsaModel, BodyComputationMeta, BodyPosition, CelestialBody, ComputationResult,
    CoordinateFrame, DateTimeInput, De440Backend, EngineConfig, EngineMode, EphemerisBackend,
    GeolocationInput, HouseSystem, InMemoryBackend, PositionResult, ResultMetadata,
};
use astro_vedic::{
    compute_panchang_day, detect_yogas, drekkana_sign, lagna_position_from_sidereal_longitude,
    moon_sidereal_division_from_tropical, navamsa_sign, sidereal_division, sidereal_longitude_deg,
    vimshottari_dasha, vimshottari_dasha_at, vimshottari_timeline,
    whole_sign_houses_from_sidereal_ascendant, DetectedYoga, LagnaPosition, Nakshatra,
    PanchangDay, PlanetHouses, PlanetLongitudes, Rashi, SiderealDivision, VimshottariDasha,
    VimshottariTimeline, WholeSignHouse, YogaChartFacts, LAHIRI_ALGO_ID,
};
use axum::{
    body::{to_bytes, Body},
    extract::{Request, State},
    http::{header::CACHE_CONTROL, HeaderMap, HeaderValue, Method, StatusCode},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub const ENGINE_SEMANTIC_VERSION: &str = "0.19.0";
pub const PANCHANG_SCHEMA_VERSION: &str = "panchang_v1";
pub const NODE_POLICY_ID: &str = "true_node_mean_ecliptic_of_date";
pub const VALIDATION_BASELINE: &str = "JPL Horizons";
/// Station-suite longitude tolerance: 1e-7° expressed in arcseconds.
pub const PROVENANCE_TOLERANCE_ARCSEC: f64 = 0.00036;
pub const DEFAULT_CHANGELOG_URL: &str =
    "https://github.com/daanyam/astro-engine/blob/main/CHANGELOG.md";
const BUILD_GIT_COMMIT: &str = env!("GIT_COMMIT");
const BUILD_DATE_RFC3339: &str = env!("BUILD_DATE");
pub const CHART_SIDEREAL_SCHEMA_VERSION: &str = "chart_sidereal_v1";
pub const DASHA_SCHEMA_VERSION: &str = "dasha_v2";
/// Inbound `X-Request-Id` is accepted when present; the same value is echoed on the response.
const REQUEST_ID_HEADER: &str = "x-request-id";
const CORRELATION_ID_HEADER: &str = "x-correlation-id";
const TRACEPARENT_HEADER: &str = "traceparent";
const CLOUD_TRACE_CONTEXT_HEADER: &str = "x-cloud-trace-context";
const VALID_API_KEYS_ENV_VAR: &str = "VALID_API_KEYS";
const RATE_LIMIT_RPM_ENV_VAR: &str = "RATE_LIMIT_RPM";
const RETRY_AFTER_HEADER: &str = "retry-after";
const API_KEY_HEADER: &str = "x-api-key";
const AUTHORIZATION_HEADER: &str = "authorization";
const API_KEY_LOG_PREFIX_CHARS: usize = 8;
const METRICS_TOKEN_ENV_VAR: &str = "METRICS_TOKEN";

/// Redoc UI loads the spec from the same origin (`GET /openapi.json`). The bundle is pinned for stable builds.
const REDOC_STANDALONE_JS: &str =
    "https://cdn.jsdelivr.net/npm/redoc@2.5.1/bundles/redoc.standalone.js";

#[derive(Clone)]
pub struct ApiState {
    backend: Arc<dyn EphemerisBackend>,
    config: EngineConfig,
    version: String,
    /// Stable digest for the loaded ephemeris kernel when known (hex sha256 over source + path).
    kernel_hash: String,
    kernel_load_seconds: f64,
}

impl ApiState {
    pub fn new(
        backend: Arc<dyn EphemerisBackend>,
        config: EngineConfig,
        version: impl Into<String>,
    ) -> Self {
        Self {
            backend,
            config,
            version: version.into(),
            kernel_hash: String::new(),
            kernel_load_seconds: 0.0,
        }
    }

    pub fn with_kernel_provenance(
        mut self,
        kernel_hash: impl Into<String>,
        kernel_load_seconds: f64,
    ) -> Self {
        self.kernel_hash = kernel_hash.into();
        self.kernel_load_seconds = kernel_load_seconds;
        self
    }

    pub fn engine_version(&self) -> &str {
        &self.version
    }

    pub fn kernel_hash(&self) -> &str {
        &self.kernel_hash
    }

    pub fn kernel_load_seconds(&self) -> f64 {
        self.kernel_load_seconds
    }
}

/// Install the Prometheus recorder and set startup gauges from `state`.
pub fn init_observability(state: &ApiState) {
    metrics::init(state);
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ProvenanceResponse {
    pub engine_semantic_version: String,
    pub version: String,
    pub engine_mode: EngineMode,
    pub ayanamsa_used: AyanamsaModel,
    pub house_system: HouseSystem,
    pub gravitational_deflection: bool,
    pub kernel_hash: String,
    pub kernel_load_seconds: f64,
    pub kernel_id: String,
    pub kernel_source: String,
    pub ayanamsa_id: String,
    pub ayanamsa_algorithm: String,
    pub ayanamsa_version: String,
    pub git_commit: String,
    pub build_date: String,
    pub tolerance_arcsec: f64,
    pub validation_baseline: String,
    pub changelog_url: String,
    pub node_policy_id: String,
    pub supported_bodies: Vec<CelestialBody>,
}

pub fn supported_celestial_bodies() -> &'static [CelestialBody] {
    &[
        CelestialBody::Sun,
        CelestialBody::Moon,
        CelestialBody::Mercury,
        CelestialBody::Venus,
        CelestialBody::Mars,
        CelestialBody::Jupiter,
        CelestialBody::Saturn,
        CelestialBody::Rahu,
        CelestialBody::Ketu,
    ]
}

fn changelog_url_from_env() -> String {
    std::env::var("GITHUB_REPO_URL")
        .map(|base| format!("{}/blob/main/CHANGELOG.md", base.trim_end_matches('/')))
        .unwrap_or_else(|_| DEFAULT_CHANGELOG_URL.to_owned())
}

fn ayanamsa_id_from_model(model: AyanamsaModel) -> String {
    serde_json::to_value(model)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| "lahiri".to_owned())
}

fn kernel_catalog(kernel_hash: &str) -> (&'static str, &'static str) {
    if kernel_hash.is_empty() {
        ("demo", "In-memory analytic ephemeris")
    } else {
        ("de440", "JPL DE440")
    }
}

pub fn build_provenance_response(state: &ApiState) -> ProvenanceResponse {
    let (kernel_id, kernel_source) = kernel_catalog(state.kernel_hash());
    let ayanamsa_id = ayanamsa_id_from_model(state.config.ayanamsa);
    ProvenanceResponse {
        engine_semantic_version: ENGINE_SEMANTIC_VERSION.to_owned(),
        version: state.engine_version().to_owned(),
        engine_mode: state.config.mode,
        ayanamsa_used: state.config.ayanamsa,
        house_system: state.config.house_system,
        gravitational_deflection: state.config.gravitational_deflection,
        kernel_hash: state.kernel_hash().to_owned(),
        kernel_load_seconds: state.kernel_load_seconds(),
        kernel_id: kernel_id.to_owned(),
        kernel_source: kernel_source.to_owned(),
        ayanamsa_id: ayanamsa_id.clone(),
        ayanamsa_algorithm: LAHIRI_ALGO_ID.to_owned(),
        ayanamsa_version: LAHIRI_ALGO_ID.to_owned(),
        git_commit: BUILD_GIT_COMMIT.to_owned(),
        build_date: BUILD_DATE_RFC3339.to_owned(),
        tolerance_arcsec: PROVENANCE_TOLERANCE_ARCSEC,
        validation_baseline: VALIDATION_BASELINE.to_owned(),
        changelog_url: changelog_url_from_env(),
        node_policy_id: NODE_POLICY_ID.to_owned(),
        supported_bodies: supported_celestial_bodies().to_vec(),
    }
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
    /// When true, populate `extensions.yogas` with classical yoga detections.
    /// Defaults to true when `compact` is false.
    pub include_yogas: Option<bool>,
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
    pub d3_rashi: Rashi,
    pub d9_rashi: Rashi,
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
    pub d3_rashi: Rashi,
    pub d9_rashi: Rashi,
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
    /// Optional RFC-3339 instant for the "current period" snapshot.
    /// Defaults to UTC now when omitted (preserves dasha_v1 behaviour).
    #[serde(default)]
    pub as_of_utc_rfc3339: Option<String>,
    /// When true, include the full Maha + Antar timeline in the response.
    /// Defaults to false to preserve dasha_v1 payload shape.
    #[serde(default)]
    pub include_timeline: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DashaPayload {
    /// Original dasha_v1 field — current Maha+Antar+Pratyantar snapshot.
    /// Retained for backward compatibility; identical to `current`.
    pub dasha: VimshottariDasha,
    /// Schema marker — "dasha_v1" when timeline omitted, "dasha_v2" when present.
    #[serde(default)]
    pub schema_version: String,
    /// Current Maha+Antar+Pratyantar snapshot. Same content as `dasha`; named
    /// for clarity in dasha_v2 clients that prefer not to overload `dasha`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current: Option<VimshottariDasha>,
    /// Full Vimshottari timeline (9 Mahas, 81 Antars, 9 Pratyantars within the
    /// current Antar). Present only when `include_timeline: true` in the request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline: Option<VimshottariTimeline>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorPayload {
    pub error: String,
}

#[derive(Debug, Deserialize)]
pub struct PanchangRequest {
    pub datetime: DateTimeInput,
    pub geo: GeolocationInput,
    pub ayanamsa: AyanamsaModel,
    pub gravitational_deflection: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct PanchangPayload {
    pub schema_version: String,
    pub panchang: PanchangDay,
}

#[derive(Debug, Deserialize)]
pub struct PanchangBatchRequest {
    pub dates: Vec<DateTimeInput>,
    pub geo: GeolocationInput,
    pub ayanamsa: AyanamsaModel,
    pub gravitational_deflection: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct PanchangBatchPayload {
    pub schema_version: String,
    pub panchang: Vec<PanchangDay>,
}

#[derive(Debug, Deserialize)]
pub struct YogaGrahaInput {
    pub body: CelestialBody,
    pub sidereal_longitude_deg: f64,
    pub whole_sign_house: u8,
}

#[derive(Debug, Deserialize)]
pub struct YogasAnalysisRequest {
    pub lagna_rashi: Rashi,
    pub grahas: Vec<YogaGrahaInput>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct YogasAnalysisPayload {
    pub schema_version: String,
    pub yogas: Vec<DetectedYoga>,
}

pub fn app_router(state: ApiState) -> Router {
    let auth = ApiAuthConfig::from_env();
    build_app_router(state, auth)
}

pub fn app_router_with_api_keys<I, S>(state: ApiState, valid_api_keys: I) -> Router
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let auth = ApiAuthConfig::from_keys(valid_api_keys);
    build_app_router(state, auth)
}

fn build_app_router(state: ApiState, auth: ApiAuthConfig) -> Router {
    let rate_limiter = RateLimiter::from_env();
    let observability_state = state.clone();
    Router::new()
        .route("/health", get(health))
        .route("/provenance", get(provenance))
        .route("/metrics", get(metrics_endpoint))
        .route("/positions", post(positions))
        .route("/positions/sidereal", post(sidereal_positions))
        .route("/chart/sidereal", post(sidereal_chart))
        .route("/panchang/daily", post(panchang_daily))
        .route("/panchang/batch", post(panchang_batch))
        .route("/analysis/yogas", post(analysis_yogas))
        .route("/openapi.json", get(openapi_json))
        .route("/docs", get(redoc_docs))
        .route("/dasha", post(dasha))
        .with_state(state)
        .layer(middleware::from_fn({
            let rate_limiter = rate_limiter.clone();
            move |request, next| {
                let rate_limiter = rate_limiter.clone();
                async move { rate_limit_middleware(request, next, rate_limiter).await }
            }
        }))
        .layer(middleware::from_fn(move |request, next| {
            let auth = auth.clone();
            async move { auth_middleware(request, next, auth).await }
        }))
        .layer(middleware::from_fn_with_state(observability_state, observability_middleware))
}

#[derive(Clone, Debug)]
struct ApiAuthConfig {
    valid_keys: Arc<HashSet<String>>,
}

impl ApiAuthConfig {
    fn from_env() -> Self {
        Self::from_csv(std::env::var(VALID_API_KEYS_ENV_VAR).ok().as_deref())
    }

    fn from_keys<I, S>(keys: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let valid_keys = keys.into_iter().map(Into::into).collect::<HashSet<_>>();
        Self { valid_keys: Arc::new(valid_keys) }
    }

    fn from_csv(value: Option<&str>) -> Self {
        let valid_keys = value
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|key| !key.is_empty())
            .map(str::to_owned)
            .collect::<HashSet<_>>();
        Self { valid_keys: Arc::new(valid_keys) }
    }

    fn is_valid_key(&self, key: &str) -> bool {
        self.valid_keys.contains(key)
    }
}

#[derive(Clone)]
struct RateLimiter {
    rpm: u32,
    inner: Arc<Mutex<HashMap<String, VecDeque<Instant>>>>,
}

impl RateLimiter {
    fn from_env() -> Self {
        Self::new(rate_limit_rpm_from_env())
    }

    fn new(rpm: u32) -> Self {
        Self { rpm, inner: Arc::new(Mutex::new(HashMap::new())) }
    }

    fn is_enabled(&self) -> bool {
        self.rpm > 0
    }

    /// Returns `Ok(())` when the request is allowed, or `Err(retry_after_seconds)` when limited.
    fn try_acquire(&self, key: &str, now: Instant) -> Result<(), u64> {
        if !self.is_enabled() {
            return Ok(());
        }
        let mut guard = self.inner.lock().expect("rate limiter mutex poisoned");
        let timestamps = guard.entry(key.to_owned()).or_default();
        prune_rolling_window(timestamps, now);
        let limit = self.rpm as usize;
        if timestamps.len() < limit {
            timestamps.push_back(now);
            Ok(())
        } else {
            Err(retry_after_seconds_for_window(timestamps.front().copied(), now))
        }
    }
}

fn rate_limit_rpm_from_env() -> u32 {
    std::env::var(RATE_LIMIT_RPM_ENV_VAR)
        .ok()
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or(0)
}

fn prune_rolling_window(timestamps: &mut VecDeque<Instant>, now: Instant) {
    let window = Duration::from_secs(60);
    while let Some(&front) = timestamps.front() {
        if now.saturating_duration_since(front) >= window {
            timestamps.pop_front();
        } else {
            break;
        }
    }
}

fn retry_after_seconds_for_window(oldest: Option<Instant>, now: Instant) -> u64 {
    let window = Duration::from_secs(60);
    let oldest = match oldest {
        Some(inst) => inst,
        None => return 1,
    };
    let elapsed = now.saturating_duration_since(oldest);
    if elapsed >= window {
        return 1;
    }
    let remaining = window - elapsed;
    remaining.as_secs().max(1)
}

async fn rate_limit_middleware(request: Request, next: Next, limiter: RateLimiter) -> Response {
    if !limiter.is_enabled() {
        return next.run(request).await;
    }

    let method = request.method().clone();
    let path = request.uri().path();
    if is_public_route(&method, path) {
        return next.run(request).await;
    }

    let Some(api_key) = request_api_key(request.headers()) else {
        return next.run(request).await;
    };

    let now = Instant::now();
    match limiter.try_acquire(api_key, now) {
        Ok(()) => next.run(request).await,
        Err(retry_after_secs) => rate_limited_response(retry_after_secs),
    }
}

fn rate_limited_response(retry_after_secs: u64) -> Response {
    let mut response = (
        StatusCode::TOO_MANY_REQUESTS,
        Json(ErrorPayload { error: "rate_limit_exceeded".to_owned() }),
    )
        .into_response();
    if let Ok(value) = HeaderValue::from_str(&retry_after_secs.to_string()) {
        response.headers_mut().insert(RETRY_AFTER_HEADER, value);
    }
    response
}

/// Returns a log-safe prefix of the API key (never the full secret).
fn api_key_prefix_for_log(key: Option<&str>) -> String {
    let Some(key) = key.map(str::trim).filter(|value| !value.is_empty()) else {
        return "none".to_owned();
    };
    let mut chars = key.chars();
    let prefix: String = chars.by_ref().take(API_KEY_LOG_PREFIX_CHARS).collect();
    if chars.next().is_some() {
        format!("{prefix}…")
    } else {
        prefix
    }
}

async fn auth_middleware(request: Request, next: Next, auth: ApiAuthConfig) -> Response {
    if is_public_route(request.method(), request.uri().path()) {
        return next.run(request).await;
    }

    match request_api_key(request.headers()) {
        Some(key) if auth.is_valid_key(key) => next.run(request).await,
        Some(_) => unauthorized_response("invalid_api_key"),
        None => unauthorized_response("missing_api_key"),
    }
}

fn is_public_route(method: &Method, path: &str) -> bool {
    matches!(
        (method, path),
        (&Method::GET, "/health")
            | (&Method::GET, "/provenance")
            | (&Method::GET, "/metrics")
            | (&Method::GET, "/openapi.json")
            | (&Method::GET, "/docs")
    )
}

fn request_api_key(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(API_KEY_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| bearer_token(headers))
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(AUTHORIZATION_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn unauthorized_response(error: &'static str) -> Response {
    (StatusCode::UNAUTHORIZED, Json(ErrorPayload { error: error.to_owned() })).into_response()
}

async fn observability_middleware(
    State(state): State<ApiState>,
    mut request: Request,
    next: Next,
) -> Response {
    init_observability(&state);
    let started_at = Instant::now();
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let query = request.uri().query().map(str::to_owned);
    let headers = request.headers().clone();
    let request_id = request_id_from_headers(&headers).unwrap_or_else(generate_request_id);
    let api_key_prefix = api_key_prefix_for_log(request_api_key(&headers));
    let body_hash = capture_request_body_hash(&mut request).await;

    let mut response = next.run(request).await;
    if let Ok(header_value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert(REQUEST_ID_HEADER, header_value);
    }

    let status = response.status().as_u16();
    let latency_ms = started_at.elapsed().as_millis();
    metrics::record_request(&path, status, latency_ms);
    let log_entry = RequestLogEntry {
        method: &method,
        path: &path,
        query: query.as_deref(),
        status,
        latency_ms,
        request_id: &request_id,
        body_hash: body_hash.as_deref(),
        api_key_prefix: &api_key_prefix,
        headers: &headers,
    };
    let engine_version = state.engine_version();
    let kernel_hash = state.kernel_hash();
    emit_request_log(&log_entry, engine_version, kernel_hash);
    if should_emit_slo_breach(log_entry.path, status, latency_ms) {
        let target_ms = slo_target_ms(log_entry.path).expect("checked by should_emit_slo_breach");
        emit_slo_breach_log(&log_entry, target_ms, engine_version, kernel_hash);
    }

    response
}

struct RequestLogEntry<'a> {
    method: &'a Method,
    path: &'a str,
    query: Option<&'a str>,
    status: u16,
    latency_ms: u128,
    request_id: &'a str,
    body_hash: Option<&'a str>,
    api_key_prefix: &'a str,
    headers: &'a HeaderMap,
}

fn request_log_payload(
    entry: &RequestLogEntry<'_>,
    engine_version: &str,
    kernel_hash: &str,
) -> Value {
    let mut payload = json!({
        "severity": request_log_severity(entry.status),
        "message": "api_usage",
        "request_id": entry.request_id,
        "api_key_prefix": entry.api_key_prefix,
        "method": entry.method.as_str(),
        "path": entry.path,
        "status": entry.status,
        "latency_ms": entry.latency_ms,
        "engine_version": engine_version,
        "kernel_hash": kernel_hash,
    });

    if let Some(query) = entry.query {
        payload["query"] = json!(query);
    }

    if let Some(body_hash) = entry.body_hash {
        payload["request_body_hash"] = json!(body_hash);
    }

    if let Some(user_agent) = header_value(entry.headers, axum::http::header::USER_AGENT.as_str()) {
        payload["user_agent"] = json!(user_agent);
    }

    if let Some(remote_ip) = forwarded_for_ip(entry.headers) {
        payload["remote_ip"] = json!(remote_ip);
    }

    if let Some(traceparent) = header_value(entry.headers, TRACEPARENT_HEADER) {
        payload["traceparent"] = json!(traceparent);
    }

    if let Some((trace, span_id, sampled)) = cloud_logging_trace_fields(entry.headers) {
        payload["logging.googleapis.com/trace"] = json!(trace);
        if let Some(span_id) = span_id {
            payload["logging.googleapis.com/spanId"] = json!(span_id);
        }
        if let Some(sampled) = sampled {
            payload["logging.googleapis.com/trace_sampled"] = json!(sampled);
        }
    }

    payload
}

fn emit_request_log(entry: &RequestLogEntry<'_>, engine_version: &str, kernel_hash: &str) {
    let payload = request_log_payload(entry, engine_version, kernel_hash);
    eprintln!("{payload}");
}

const CHART_SIDEREAL_SLO_MS: u128 = 200;
const DASHA_SLO_MS: u128 = 300;

fn slo_target_ms(path: &str) -> Option<u128> {
    match path {
        "/chart/sidereal" => Some(CHART_SIDEREAL_SLO_MS),
        "/dasha" => Some(DASHA_SLO_MS),
        _ => None,
    }
}

fn should_emit_slo_breach(path: &str, status: u16, latency_ms: u128) -> bool {
    (200..300).contains(&status) && slo_target_ms(path).is_some_and(|target| latency_ms > target)
}

fn slo_breach_payload(
    entry: &RequestLogEntry<'_>,
    target_ms: u128,
    engine_version: &str,
    kernel_hash: &str,
) -> Value {
    json!({
        "severity": "WARNING",
        "message": "slo_breach",
        "slo_breach": true,
        "path": entry.path,
        "target_ms": target_ms,
        "actual_ms": entry.latency_ms,
        "request_id": entry.request_id,
        "engine_version": engine_version,
        "kernel_hash": kernel_hash,
    })
}

fn emit_slo_breach_log(
    entry: &RequestLogEntry<'_>,
    target_ms: u128,
    engine_version: &str,
    kernel_hash: &str,
) {
    let payload = slo_breach_payload(entry, target_ms, engine_version, kernel_hash);
    eprintln!("{payload}");
}

fn request_log_severity(status: u16) -> &'static str {
    match status {
        500..=599 => "ERROR",
        400..=499 => "WARNING",
        _ => "INFO",
    }
}

fn request_id_from_headers(headers: &HeaderMap) -> Option<String> {
    [REQUEST_ID_HEADER, CORRELATION_ID_HEADER]
        .into_iter()
        .find_map(|name| header_value(headers, name))
        .filter(|value| !value.is_empty())
}

fn generate_request_id() -> String {
    Uuid::new_v4().to_string()
}

async fn capture_request_body_hash(request: &mut Request) -> Option<String> {
    if !request_body_hashing_enabled(request.method(), request.headers()) {
        return None;
    }

    let request_to_buffer = std::mem::replace(request, Request::new(Body::empty()));
    let (parts, body) = request_to_buffer.into_parts();
    let bytes = to_bytes(body, usize::MAX).await.ok()?;
    let hash = if bytes.is_empty() { None } else { Some(body_hash(&bytes)) };
    *request = Request::from_parts(parts, Body::from(bytes));
    hash
}

fn request_body_hashing_enabled(method: &Method, headers: &HeaderMap) -> bool {
    matches!(*method, Method::POST | Method::PUT | Method::PATCH)
        && header_value(headers, axum::http::header::CONTENT_TYPE.as_str())
            .is_some_and(|content_type| content_type.starts_with("application/json"))
}

fn body_hash(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    format!("sha256:{}", format_sha256(&digest))
}

fn format_sha256(bytes: &[u8]) -> String {
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(&mut hex, "{byte:02x}");
    }
    hex
}

fn cloud_logging_trace_fields(
    headers: &HeaderMap,
) -> Option<(String, Option<String>, Option<bool>)> {
    let project_id = google_cloud_project_id()?;
    let cloud_trace_context = header_value(headers, CLOUD_TRACE_CONTEXT_HEADER)?;
    let trace_id = cloud_trace_context
        .split('/')
        .next()
        .map(str::trim)
        .filter(|trace_id| !trace_id.is_empty())?;
    let span_id = cloud_trace_context
        .split('/')
        .nth(1)
        .and_then(|tail| tail.split(';').next())
        .map(str::trim)
        .filter(|span_id| !span_id.is_empty())
        .map(str::to_owned);
    let sampled = cloud_trace_context.split(";o=").nth(1).and_then(|value| match value.trim() {
        "1" => Some(true),
        "0" => Some(false),
        _ => None,
    });
    Some((format!("projects/{project_id}/traces/{trace_id}"), span_id, sampled))
}

fn google_cloud_project_id() -> Option<String> {
    ["GOOGLE_CLOUD_PROJECT", "GCP_PROJECT", "GCLOUD_PROJECT"]
        .into_iter()
        .find_map(|name| std::env::var(name).ok())
        .filter(|value| !value.trim().is_empty())
}

fn header_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers.get(name).and_then(|value| value.to_str().ok()).map(str::to_owned)
}

fn forwarded_for_ip(headers: &HeaderMap) -> Option<String> {
    header_value(headers, "x-forwarded-for")
        .map(|value| value.split(',').next().map(str::trim).unwrap_or("").to_owned())
}

fn metrics_token_from_env() -> Option<String> {
    std::env::var(METRICS_TOKEN_ENV_VAR)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

async fn metrics_endpoint(State(state): State<ApiState>, headers: HeaderMap) -> Response {
    let Some(expected_token) = metrics_token_from_env() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorPayload { error: "metrics_disabled".to_owned() }),
        )
            .into_response();
    };

    init_observability(&state);

    let Some(token) = bearer_token(&headers) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ErrorPayload { error: "missing_metrics_token".to_owned() }),
        )
            .into_response();
    };

    if token != expected_token {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorPayload { error: "invalid_metrics_token".to_owned() }),
        )
            .into_response();
    }

    let Some(body) = metrics::render() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorPayload { error: "metrics_disabled".to_owned() }),
        )
            .into_response();
    };

    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")],
        body,
    )
        .into_response()
}

async fn health(State(state): State<ApiState>) -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok", version: state.version })
}

async fn provenance(State(state): State<ApiState>) -> Response {
    let cache_control = HeaderValue::from_static("public, max-age=3600");
    (StatusCode::OK, [(CACHE_CONTROL, cache_control)], Json(build_provenance_response(&state)))
        .into_response()
}

async fn openapi_json() -> Json<Value> {
    Json(openapi_spec())
}

async fn redoc_docs() -> Html<String> {
    Html(format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Daanyam Astro Engine API</title>
</head>
<body>
  <redoc spec-url="/openapi.json"></redoc>
  <script src="{REDOC_STANDALONE_JS}" crossorigin="anonymous"></script>
</body>
</html>"#
    ))
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

    // as_of defaults to "now" when the caller does not supply it, mirroring
    // the existing dasha_v1 contract (vimshottari_dasha == _at(birth, birth)).
    let as_of = match request.as_of_utc_rfc3339.as_deref() {
        Some(value) => chrono::DateTime::parse_from_rfc3339(value)
            .map_err(|err| bad_request(err.to_string()))?
            .with_timezone(&Utc),
        None => Utc::now(),
    };

    let include_timeline = request.include_timeline.unwrap_or(false);

    let payload = if include_timeline {
        let (current, timeline) =
            vimshottari_timeline(request.moon_sidereal_longitude_deg, birth_time, as_of);
        DashaPayload {
            dasha: current.clone(),
            schema_version: DASHA_SCHEMA_VERSION.to_string(),
            current: Some(current),
            timeline: Some(timeline),
        }
    } else {
        // dasha_v1 behaviour preserved: if no as_of was passed, evaluate at
        // birth (matches vimshottari_dasha); otherwise use the as_of snapshot.
        let snapshot = if request.as_of_utc_rfc3339.is_some() {
            vimshottari_dasha_at(request.moon_sidereal_longitude_deg, birth_time, as_of)
        } else {
            vimshottari_dasha(request.moon_sidereal_longitude_deg, birth_time)
        };
        DashaPayload {
            dasha: snapshot,
            schema_version: "dasha_v1".to_string(),
            current: None,
            timeline: None,
        }
    };

    Ok(Json(ComputationResult { data: payload, metadata: metadata(&state) }))
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
    let include_yogas = request.include_yogas.unwrap_or(!compact);
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
    let mut extensions = Map::new();
    if include_yogas {
        let yoga_facts = build_yoga_chart_facts(lagna.rashi, &grahas);
        let yogas = detect_yogas(&yoga_facts);
        extensions.insert(
            "yogas".to_owned(),
            serde_json::to_value(yogas).expect("yogas must serialize"),
        );
    }
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
            extensions,
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

async fn panchang_daily(
    State(state): State<ApiState>,
    Json(request): Json<PanchangRequest>,
) -> Result<Json<ComputationResult<PanchangPayload>>, (StatusCode, Json<ErrorPayload>)> {
    let panchang = compute_panchang_for_request(&state, &request.datetime, request.geo, request.ayanamsa, request.gravitational_deflection)
        .map_err(map_sidereal_error)?;
    Ok(Json(ComputationResult {
        data: PanchangPayload {
            schema_version: PANCHANG_SCHEMA_VERSION.to_owned(),
            panchang,
        },
        metadata: metadata(&state),
    }))
}

async fn panchang_batch(
    State(state): State<ApiState>,
    Json(request): Json<PanchangBatchRequest>,
) -> Result<Json<ComputationResult<PanchangBatchPayload>>, (StatusCode, Json<ErrorPayload>)> {
    if request.dates.is_empty() {
        return Err(bad_request("dates must not be empty".to_owned()));
    }
    if request.dates.len() > 366 {
        return Err(bad_request("dates must contain at most 366 entries".to_owned()));
    }

    let mut panchang = Vec::with_capacity(request.dates.len());
    for datetime in &request.dates {
        let day = compute_panchang_for_request(
            &state,
            datetime,
            request.geo.clone(),
            request.ayanamsa,
            request.gravitational_deflection,
        )
        .map_err(map_sidereal_error)?;
        panchang.push(day);
    }

    Ok(Json(ComputationResult {
        data: PanchangBatchPayload {
            schema_version: PANCHANG_SCHEMA_VERSION.to_owned(),
            panchang,
        },
        metadata: metadata(&state),
    }))
}

async fn analysis_yogas(
    State(state): State<ApiState>,
    Json(request): Json<YogasAnalysisRequest>,
) -> Json<ComputationResult<YogasAnalysisPayload>> {
    let facts = yoga_facts_from_inputs(request.lagna_rashi, &request.grahas);
    let yogas = detect_yogas(&facts);
    Json(ComputationResult {
        data: YogasAnalysisPayload {
            schema_version: "yogas_v1".to_owned(),
            yogas,
        },
        metadata: metadata(&state),
    })
}

fn compute_panchang_for_request(
    state: &ApiState,
    datetime: &DateTimeInput,
    geo: GeolocationInput,
    ayanamsa: AyanamsaModel,
    gravitational_deflection: Option<bool>,
) -> Result<PanchangDay, SiderealRequestError> {
    let utc = resolve_datetime_input(datetime).map_err(SiderealRequestError::Time)?;
    let (_, _, positions) = compute_sidereal_positions(
        state,
        datetime.clone(),
        geo.clone(),
        ayanamsa,
        vec![CelestialBody::Sun, CelestialBody::Moon],
        gravitational_deflection,
        ChartProjection::SiderealOnly,
    )?;
    let sun = positions
        .iter()
        .find(|p| p.body == CelestialBody::Sun)
        .expect("sun position");
    let moon = positions
        .iter()
        .find(|p| p.body == CelestialBody::Moon)
        .expect("moon position");
    Ok(compute_panchang_day(
        utc,
        &geo,
        moon.sidereal_longitude_deg,
        sun.sidereal_longitude_deg,
    ))
}

fn build_yoga_chart_facts(
    lagna_rashi: Rashi,
    positions: &[SiderealPositionResult],
) -> YogaChartFacts {
    let mut facts = YogaChartFacts {
        lagna_rashi,
        planet_longitudes: PlanetLongitudes::default(),
        planet_houses: PlanetHouses::default(),
    };

    for position in positions {
        let house = whole_sign_house_number(
            lagna_rashi,
            sidereal_division(position.sidereal_longitude_deg).rashi,
        );
        match position.body {
            CelestialBody::Sun => {
                facts.planet_longitudes.sun = Some(position.sidereal_longitude_deg);
                facts.planet_houses.sun = Some(house);
            }
            CelestialBody::Moon => {
                facts.planet_longitudes.moon = Some(position.sidereal_longitude_deg);
                facts.planet_houses.moon = Some(house);
            }
            CelestialBody::Mars => {
                facts.planet_longitudes.mars = Some(position.sidereal_longitude_deg);
                facts.planet_houses.mars = Some(house);
            }
            CelestialBody::Mercury => {
                facts.planet_longitudes.mercury = Some(position.sidereal_longitude_deg);
                facts.planet_houses.mercury = Some(house);
            }
            CelestialBody::Jupiter => {
                facts.planet_longitudes.jupiter = Some(position.sidereal_longitude_deg);
                facts.planet_houses.jupiter = Some(house);
            }
            CelestialBody::Venus => {
                facts.planet_longitudes.venus = Some(position.sidereal_longitude_deg);
                facts.planet_houses.venus = Some(house);
            }
            CelestialBody::Saturn => {
                facts.planet_longitudes.saturn = Some(position.sidereal_longitude_deg);
                facts.planet_houses.saturn = Some(house);
            }
            CelestialBody::Rahu => {
                facts.planet_longitudes.rahu = Some(position.sidereal_longitude_deg);
                facts.planet_houses.rahu = Some(house);
            }
            CelestialBody::Ketu => {
                facts.planet_longitudes.ketu = Some(position.sidereal_longitude_deg);
                facts.planet_houses.ketu = Some(house);
            }
            _ => {}
        }
    }

    facts
}

fn yoga_facts_from_inputs(lagna_rashi: Rashi, grahas: &[YogaGrahaInput]) -> YogaChartFacts {
    let mut facts = YogaChartFacts {
        lagna_rashi,
        planet_longitudes: PlanetLongitudes::default(),
        planet_houses: PlanetHouses::default(),
    };
    for graha in grahas {
        match graha.body {
            CelestialBody::Sun => {
                facts.planet_longitudes.sun = Some(graha.sidereal_longitude_deg);
                facts.planet_houses.sun = Some(graha.whole_sign_house);
            }
            CelestialBody::Moon => {
                facts.planet_longitudes.moon = Some(graha.sidereal_longitude_deg);
                facts.planet_houses.moon = Some(graha.whole_sign_house);
            }
            CelestialBody::Mars => {
                facts.planet_longitudes.mars = Some(graha.sidereal_longitude_deg);
                facts.planet_houses.mars = Some(graha.whole_sign_house);
            }
            CelestialBody::Mercury => {
                facts.planet_longitudes.mercury = Some(graha.sidereal_longitude_deg);
                facts.planet_houses.mercury = Some(graha.whole_sign_house);
            }
            CelestialBody::Jupiter => {
                facts.planet_longitudes.jupiter = Some(graha.sidereal_longitude_deg);
                facts.planet_houses.jupiter = Some(graha.whole_sign_house);
            }
            CelestialBody::Venus => {
                facts.planet_longitudes.venus = Some(graha.sidereal_longitude_deg);
                facts.planet_houses.venus = Some(graha.whole_sign_house);
            }
            CelestialBody::Saturn => {
                facts.planet_longitudes.saturn = Some(graha.sidereal_longitude_deg);
                facts.planet_houses.saturn = Some(graha.whole_sign_house);
            }
            CelestialBody::Rahu => {
                facts.planet_longitudes.rahu = Some(graha.sidereal_longitude_deg);
                facts.planet_houses.rahu = Some(graha.whole_sign_house);
            }
            CelestialBody::Ketu => {
                facts.planet_longitudes.ketu = Some(graha.sidereal_longitude_deg);
                facts.planet_houses.ketu = Some(graha.whole_sign_house);
            }
            _ => {}
        }
    }
    facts
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
    motion: astro_core::LongitudeMotion,
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
        longitude_speed_deg_per_day: motion.longitude_speed_deg_per_day,
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
        d3_rashi: drekkana_sign(graha.sidereal_longitude_deg),
        d9_rashi: navamsa_sign(graha.sidereal_longitude_deg),
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
            d3_rashi: graha.d3_rashi,
            d9_rashi: graha.d9_rashi,
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
) -> Result<astro_core::LongitudeMotion, SiderealRequestError> {
    astro_core::longitude_motion(state.backend.as_ref(), body, jd_utc, config)
        .map_err(SiderealRequestError::Backend)
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
            "/provenance": {
                "get": {
                    "summary": "Return engine runtime provenance metadata",
                    "responses": {
                        "200": {
                            "description": "Provenance payload",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ProvenanceResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/positions": {
                "post": {
                    "summary": "Compute tropical positions",
                    "security": [
                        { "ApiKeyAuth": [] },
                        { "BearerAuth": [] }
                    ],
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
                    "security": [
                        { "ApiKeyAuth": [] },
                        { "BearerAuth": [] }
                    ],
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
                    "security": [
                        { "ApiKeyAuth": [] },
                        { "BearerAuth": [] }
                    ],
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
            "securitySchemes": {
                "ApiKeyAuth": {
                    "type": "apiKey",
                    "in": "header",
                    "name": "x-api-key",
                    "description": "API key issued for this service. Send the same secret in either this header or as a Bearer token."
                },
                "BearerAuth": {
                    "type": "http",
                    "scheme": "bearer",
                    "bearerFormat": "API key",
                    "description": "Bearer token using the same secret as x-api-key (`Authorization: Bearer` …)."
                }
            },
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
                "ProvenanceResponse": {
                    "type": "object",
                    "required": [
                        "engine_semantic_version",
                        "version",
                        "engine_mode",
                        "ayanamsa_used",
                        "house_system",
                        "gravitational_deflection",
                        "kernel_hash",
                        "kernel_load_seconds",
                        "kernel_id",
                        "kernel_source",
                        "ayanamsa_id",
                        "ayanamsa_algorithm",
                        "ayanamsa_version",
                        "git_commit",
                        "build_date",
                        "tolerance_arcsec",
                        "validation_baseline",
                        "changelog_url",
                        "node_policy_id",
                        "supported_bodies"
                    ],
                    "properties": {
                        "engine_semantic_version": { "type": "string" },
                        "version": { "type": "string" },
                        "engine_mode": { "type": "string" },
                        "ayanamsa_used": { "type": "string" },
                        "house_system": { "type": "string" },
                        "gravitational_deflection": { "type": "boolean" },
                        "kernel_hash": { "type": "string" },
                        "kernel_load_seconds": { "type": "number" },
                        "kernel_id": { "type": "string", "example": "de440" },
                        "kernel_source": { "type": "string", "example": "JPL DE440" },
                        "ayanamsa_id": { "type": "string", "example": "lahiri" },
                        "ayanamsa_algorithm": { "type": "string", "example": "lahiri_swe_zero_epoch_iau1976_v1" },
                        "ayanamsa_version": { "type": "string", "example": "lahiri_swe_zero_epoch_iau1976_v1" },
                        "git_commit": { "type": "string" },
                        "build_date": { "type": "string", "format": "date-time" },
                        "tolerance_arcsec": { "type": "number", "example": 0.00036 },
                        "validation_baseline": { "type": "string", "example": "JPL Horizons" },
                        "changelog_url": { "type": "string", "format": "uri" },
                        "node_policy_id": { "type": "string" },
                        "supported_bodies": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/CelestialBody" }
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
                        "d3_rashi": {
                            "type": "string",
                            "description": "Drekkana (D3) rashi derived from the graha sidereal longitude."
                        },
                        "d9_rashi": {
                            "type": "string",
                            "description": "Navamsa (D9) rashi derived from the graha sidereal longitude."
                        },
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
                        "d3_rashi": {
                            "type": "string",
                            "description": "Drekkana (D3) rashi derived from the graha sidereal longitude."
                        },
                        "d9_rashi": {
                            "type": "string",
                            "description": "Navamsa (D9) rashi derived from the graha sidereal longitude."
                        },
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
                "VimshottariTimeline": {
                    "type": "object",
                    "description": "Full Vimshottari timeline (dasha_v2). Returned when include_timeline=true on POST /dasha.",
                    "properties": {
                        "mahadashas": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/DashaPeriod" },
                            "description": "9 Mahadashas spanning the 120-yr cycle from birth_time."
                        },
                        "antardashas": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/DashaPeriod" },
                            "description": "81 Antardashas (9 per Maha) in chronological order."
                        },
                        "current_antar_pratyantars": {
                            "type": "array",
                            "items": { "$ref": "#/components/schemas/DashaPeriod" },
                            "description": "9 Pratyantars within the currently-active Antar."
                        }
                    },
                    "required": ["mahadashas", "antardashas", "current_antar_pratyantars"]
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
        body::Body,
        http::{Method, Request, StatusCode},
    };
    use serde_json::Value;
    use std::sync::{Mutex, OnceLock};
    use std::time::{Duration, Instant};
    use tower::util::ServiceExt;

    use super::*;

    const TEST_API_KEY: &str = "test-api-key";

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().expect("env lock poisoned")
    }

    fn metrics_test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().expect("metrics test lock poisoned")
    }

    fn test_app() -> Router {
        app_router_with_api_keys(demo_state(), [TEST_API_KEY])
    }

    #[tokio::test]
    async fn health_endpoint_reports_version() {
        let app = test_app();
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
        assert!(response.headers().contains_key(REQUEST_ID_HEADER));
    }

    #[tokio::test]
    async fn request_id_header_is_preserved_when_provided() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/health")
                    .header(REQUEST_ID_HEADER, "webapp-request-123")
                    .body(Body::empty())
                    .expect("request must build"),
            )
            .await
            .expect("response must succeed");

        assert_eq!(
            response.headers().get(REQUEST_ID_HEADER).and_then(|value| value.to_str().ok()),
            Some("webapp-request-123")
        );
    }

    #[tokio::test]
    async fn correlation_id_header_promotes_to_request_id() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/health")
                    .header(CORRELATION_ID_HEADER, "correlation-456")
                    .body(Body::empty())
                    .expect("request must build"),
            )
            .await
            .expect("response must succeed");

        assert_eq!(
            response.headers().get(REQUEST_ID_HEADER).and_then(|value| value.to_str().ok()),
            Some("correlation-456")
        );
    }

    #[tokio::test]
    async fn request_id_header_canonical_casing_is_preserved() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/health")
                    .header("X-Request-Id", "canonical-case-id-789")
                    .body(Body::empty())
                    .expect("request must build"),
            )
            .await
            .expect("response must succeed");

        assert_eq!(
            response.headers().get(REQUEST_ID_HEADER).and_then(|value| value.to_str().ok()),
            Some("canonical-case-id-789")
        );
    }

    #[test]
    fn request_log_payload_includes_engine_version_and_kernel_hash() {
        let headers = HeaderMap::new();
        let entry = RequestLogEntry {
            method: &Method::POST,
            path: "/chart/sidereal",
            query: None,
            status: 200,
            latency_ms: 42,
            request_id: "req-1",
            body_hash: Some("sha256:abc"),
            api_key_prefix: "testkey…",
            headers: &headers,
        };
        let payload = request_log_payload(&entry, "0.1.0", "kernel-digest");
        assert_eq!(payload["message"].as_str(), Some("api_usage"));
        assert_eq!(payload["request_id"].as_str(), Some("req-1"));
        assert_eq!(payload["path"].as_str(), Some("/chart/sidereal"));
        assert_eq!(payload["status"].as_u64(), Some(200));
        assert_eq!(payload["latency_ms"].as_u64(), Some(42));
        assert_eq!(payload["engine_version"].as_str(), Some("0.1.0"));
        assert_eq!(payload["kernel_hash"].as_str(), Some("kernel-digest"));
        assert_eq!(payload["api_key_prefix"].as_str(), Some("testkey…"));
        assert_eq!(payload["request_body_hash"].as_str(), Some("sha256:abc"));
    }

    fn sample_log_entry<'a>(
        path: &'a str,
        status: u16,
        latency_ms: u128,
        headers: &'a HeaderMap,
    ) -> RequestLogEntry<'a> {
        RequestLogEntry {
            method: &Method::POST,
            path,
            query: None,
            status,
            latency_ms,
            request_id: "req-slo-test",
            body_hash: None,
            api_key_prefix: "testkey…",
            headers,
        }
    }

    #[test]
    fn slo_breach_payload_emitted_when_latency_exceeds_chart_sidereal_target() {
        let headers = HeaderMap::new();
        let entry = sample_log_entry("/chart/sidereal", 200, 201, &headers);
        assert!(should_emit_slo_breach(entry.path, entry.status, entry.latency_ms));
        let payload = slo_breach_payload(&entry, CHART_SIDEREAL_SLO_MS, "0.17.2", "kernel-digest");
        assert_eq!(payload["severity"].as_str(), Some("WARNING"));
        assert_eq!(payload["message"].as_str(), Some("slo_breach"));
        assert_eq!(payload["slo_breach"].as_bool(), Some(true));
        assert_eq!(payload["path"].as_str(), Some("/chart/sidereal"));
        assert_eq!(payload["target_ms"].as_u64(), Some(200));
        assert_eq!(payload["actual_ms"].as_u64(), Some(201));
        assert_eq!(payload["request_id"].as_str(), Some("req-slo-test"));
        assert_eq!(payload["engine_version"].as_str(), Some("0.17.2"));
        assert_eq!(payload["kernel_hash"].as_str(), Some("kernel-digest"));
    }

    #[test]
    fn slo_breach_not_emitted_at_exact_chart_sidereal_target() {
        let headers = HeaderMap::new();
        let entry = sample_log_entry("/chart/sidereal", 200, 200, &headers);
        assert!(!should_emit_slo_breach(entry.path, entry.status, entry.latency_ms));
    }

    #[test]
    fn slo_breach_not_emitted_for_non_success_status() {
        assert!(!should_emit_slo_breach("/chart/sidereal", 500, 500));
        assert!(!should_emit_slo_breach("/chart/sidereal", 404, 500));
    }

    #[test]
    fn slo_breach_not_emitted_for_untracked_paths() {
        assert!(!should_emit_slo_breach("/health", 200, 10_000));
    }

    #[test]
    fn slo_breach_payload_emitted_when_latency_exceeds_dasha_target() {
        let headers = HeaderMap::new();
        let entry = sample_log_entry("/dasha", 200, 301, &headers);
        assert!(should_emit_slo_breach(entry.path, entry.status, entry.latency_ms));
        let payload = slo_breach_payload(&entry, DASHA_SLO_MS, "0.17.2", "kernel-digest");
        assert_eq!(payload["path"].as_str(), Some("/dasha"));
        assert_eq!(payload["target_ms"].as_u64(), Some(300));
        assert_eq!(payload["actual_ms"].as_u64(), Some(301));
    }

    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn metrics_returns_503_when_token_unset() {
        let _serial = metrics_test_lock();
        std::env::remove_var(METRICS_TOKEN_ENV_VAR);

        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/metrics")
                    .body(Body::empty())
                    .expect("request must build"),
            )
            .await
            .expect("response must succeed");

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn metrics_returns_401_without_bearer_token() {
        let _serial = metrics_test_lock();
        std::env::set_var(METRICS_TOKEN_ENV_VAR, "test-metrics-secret");

        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/metrics")
                    .body(Body::empty())
                    .expect("request must build"),
            )
            .await
            .expect("response must succeed");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        std::env::remove_var(METRICS_TOKEN_ENV_VAR);
    }

    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn metrics_returns_403_with_invalid_bearer_token() {
        let _serial = metrics_test_lock();
        std::env::set_var(METRICS_TOKEN_ENV_VAR, "test-metrics-secret");

        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/metrics")
                    .header(AUTHORIZATION_HEADER, "Bearer wrong-token")
                    .body(Body::empty())
                    .expect("request must build"),
            )
            .await
            .expect("response must succeed");

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        std::env::remove_var(METRICS_TOKEN_ENV_VAR);
    }

    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn metrics_returns_prometheus_body_with_valid_bearer_token() {
        let _serial = metrics_test_lock();
        std::env::set_var(METRICS_TOKEN_ENV_VAR, "test-metrics-secret");

        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/metrics")
                    .header(AUTHORIZATION_HEADER, "Bearer test-metrics-secret")
                    .body(Body::empty())
                    .expect("request must build"),
            )
            .await
            .expect("response must succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.expect("body must be readable");
        let text = String::from_utf8(body.to_vec()).expect("metrics body must be utf-8");
        assert!(text.contains("astro_requests_total"));
        assert!(text.contains("astro_kernel_load_seconds"));
        std::env::remove_var(METRICS_TOKEN_ENV_VAR);
    }

    #[test]
    fn body_hash_is_stable_sha256() {
        assert_eq!(
            body_hash(br#"{"julian_day":2451545.0}"#),
            "sha256:1e79c420a159cd5ddbc419ed68b9fdf0c66e26814d66a5dd6a521658a2f86a80"
        );
    }

    #[test]
    fn api_key_prefix_for_log_never_includes_full_key() {
        let long = "abcdefghijklmnopqrstuvwxyz";
        assert_eq!(api_key_prefix_for_log(Some(long)), "abcdefgh…");
        assert!(!api_key_prefix_for_log(Some(long)).contains("ijkl"));
    }

    #[test]
    fn api_key_prefix_for_log_short_key_unchanged() {
        assert_eq!(api_key_prefix_for_log(Some("shorty")), "shorty");
    }

    #[test]
    fn api_key_prefix_for_log_none_maps_to_none_literal() {
        assert_eq!(api_key_prefix_for_log(None), "none");
        assert_eq!(api_key_prefix_for_log(Some("   ")), "none");
    }

    #[test]
    fn rate_limiter_allows_up_to_rpm_same_instant() {
        let limiter = RateLimiter::new(2);
        let now = Instant::now();
        assert!(limiter.try_acquire("key-a", now).is_ok());
        assert!(limiter.try_acquire("key-a", now).is_ok());
        assert!(limiter.try_acquire("key-a", now).is_err());
    }

    #[test]
    fn rate_limiter_tracks_keys_independently() {
        let limiter = RateLimiter::new(1);
        let now = Instant::now();
        assert!(limiter.try_acquire("k1", now).is_ok());
        assert!(limiter.try_acquire("k2", now).is_ok());
    }

    #[test]
    fn rate_limiter_expires_rolling_window() {
        let limiter = RateLimiter::new(1);
        let t0 = Instant::now();
        assert!(limiter.try_acquire("k", t0).is_ok());
        let t1 = t0 + Duration::from_secs(61);
        assert!(limiter.try_acquire("k", t1).is_ok());
    }

    #[tokio::test]
    async fn rate_limit_returns_429_with_retry_after() {
        {
            let _guard = env_lock();
            std::env::set_var(RATE_LIMIT_RPM_ENV_VAR, "2");
        }

        let app = test_app();

        let make_req = || {
            Request::builder()
                .method(Method::POST)
                .uri("/positions")
                .header(API_KEY_HEADER, TEST_API_KEY)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"julian_day":2451545.0,"bodies":["moon"]}"#))
                .expect("request must build")
        };

        let response = app.clone().oneshot(make_req()).await.expect("response must succeed");
        assert_eq!(response.status(), StatusCode::OK);

        let response = app.clone().oneshot(make_req()).await.expect("response must succeed");
        assert_eq!(response.status(), StatusCode::OK);

        let response = app.oneshot(make_req()).await.expect("response must succeed");
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        let retry_after = response
            .headers()
            .get(RETRY_AFTER_HEADER)
            .and_then(|value| value.to_str().ok())
            .expect("retry-after header must be set");
        let retry_secs: u64 = retry_after.parse().expect("retry-after must be numeric");
        assert!((1..=60).contains(&retry_secs));
        let body = to_bytes(response.into_body(), usize::MAX).await.expect("body must be readable");
        let json: Value = serde_json::from_slice(&body).expect("body must be valid json");
        assert_eq!(json, serde_json::json!({ "error": "rate_limit_exceeded" }));

        {
            let _guard = env_lock();
            std::env::remove_var(RATE_LIMIT_RPM_ENV_VAR);
        }
    }

    #[test]
    fn cloud_trace_context_builds_google_logging_fields() {
        std::env::set_var("GOOGLE_CLOUD_PROJECT", "daanyam-prod");
        let mut headers = HeaderMap::new();
        headers.insert(
            CLOUD_TRACE_CONTEXT_HEADER,
            HeaderValue::from_static("105445aa7843bc8bf206b120001000/123;o=1"),
        );

        let (trace, span_id, sampled) =
            cloud_logging_trace_fields(&headers).expect("cloud trace header must parse");

        assert_eq!(trace, "projects/daanyam-prod/traces/105445aa7843bc8bf206b120001000");
        assert_eq!(span_id.as_deref(), Some("123"));
        assert_eq!(sampled, Some(true));
        std::env::remove_var("GOOGLE_CLOUD_PROJECT");
    }

    #[tokio::test]
    async fn positions_endpoint_returns_metadata() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/positions")
                    .header(API_KEY_HEADER, TEST_API_KEY)
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
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/positions/sidereal")
                    .header(API_KEY_HEADER, TEST_API_KEY)
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
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/positions/sidereal")
                    .header(API_KEY_HEADER, TEST_API_KEY)
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
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/chart/sidereal")
                    .header(API_KEY_HEADER, TEST_API_KEY)
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
        assert!(json["data"]["extensions"]["yogas"].is_array());
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
        assert!(json["data"]["grahas"][0]["d3_rashi"].is_string());
        assert!(json["data"]["grahas"][0]["d9_rashi"].is_string());
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
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/chart/sidereal")
                    .header(API_KEY_HEADER, TEST_API_KEY)
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
        assert!(json["data"]["grahas"][0]["d3_rashi"].is_string());
        assert!(json["data"]["grahas"][0]["d9_rashi"].is_string());
        assert!(json["data"]["grahas"][0]["computation_meta"].is_null());
        assert!(json["data"]["grahas"][0]["moon_division"].is_null());
    }

    #[tokio::test]
    async fn sidereal_chart_sidereal_only_projection_omits_tropical_fields() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/chart/sidereal")
                    .header(API_KEY_HEADER, TEST_API_KEY)
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
        assert!(json["data"]["grahas"][0]["d3_rashi"].is_string());
        assert!(json["data"]["grahas"][0]["d9_rashi"].is_string());
        assert!(json["data"]["summary"]["houses"].is_array());
    }

    #[tokio::test]
    async fn sidereal_positions_compact_mode_omits_heavy_fields() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/positions/sidereal")
                    .header(API_KEY_HEADER, TEST_API_KEY)
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
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/positions/sidereal")
                    .header(API_KEY_HEADER, TEST_API_KEY)
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
        let app = test_app();
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
        assert_eq!(json["components"]["securitySchemes"]["ApiKeyAuth"]["type"], "apiKey");
        assert_eq!(json["components"]["securitySchemes"]["ApiKeyAuth"]["in"], "header");
        assert_eq!(json["components"]["securitySchemes"]["ApiKeyAuth"]["name"], "x-api-key");
        assert_eq!(json["components"]["securitySchemes"]["BearerAuth"]["type"], "http");
        assert_eq!(json["components"]["securitySchemes"]["BearerAuth"]["scheme"], "bearer");
        assert!(json["paths"]["/positions"]["post"]["security"].is_array());
        assert!(json["paths"]["/positions/sidereal"]["post"]["security"].is_array());
        assert!(json["paths"]["/chart/sidereal"]["post"]["security"].is_array());
        assert!(json["paths"]["/chart/sidereal"]["post"].is_object());
        assert!(json["paths"]["/positions"]["post"].is_object());
        assert!(json["paths"]["/positions/sidereal"]["post"].is_object());
        assert!(json["paths"]["/provenance"]["get"].is_object());
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
    async fn docs_route_serves_redoc_html_without_api_key() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/docs")
                    .body(Body::empty())
                    .expect("request must build"),
            )
            .await
            .expect("response must succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let content_type = response
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok());
        assert!(content_type.is_some_and(|ct| ct.starts_with("text/html")));

        let body = to_bytes(response.into_body(), usize::MAX).await.expect("body must be readable");
        let html = String::from_utf8(body.to_vec()).expect("docs body must be utf-8");
        assert!(html.contains("spec-url=\"/openapi.json\""));
        assert!(html.contains(REDOC_STANDALONE_JS), "expected pinned Redoc script URL in HTML",);
    }

    #[tokio::test]
    async fn provenance_route_is_public_and_returns_runtime_metadata() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/provenance")
                    .body(Body::empty())
                    .expect("request must build"),
            )
            .await
            .expect("response must succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let cache_control =
            response.headers().get(CACHE_CONTROL).and_then(|value| value.to_str().ok());
        assert_eq!(cache_control, Some("public, max-age=3600"));
        let body = to_bytes(response.into_body(), usize::MAX).await.expect("body must be readable");
        let json: Value = serde_json::from_slice(&body).expect("body must be valid json");
        assert_eq!(json["engine_semantic_version"], ENGINE_SEMANTIC_VERSION);
        assert!(json["version"].is_string());
        assert!(json["kernel_hash"].is_string());
        assert!(json["kernel_load_seconds"].is_number());
        assert_eq!(json["kernel_id"], "demo");
        assert_eq!(json["kernel_source"], "In-memory analytic ephemeris");
        assert_eq!(json["ayanamsa_id"], "lahiri");
        assert_eq!(json["ayanamsa_algorithm"], LAHIRI_ALGO_ID);
        assert_eq!(json["ayanamsa_version"], LAHIRI_ALGO_ID);
        assert!(json["git_commit"].is_string());
        assert!(json["build_date"].is_string());
        assert_eq!(json["tolerance_arcsec"], PROVENANCE_TOLERANCE_ARCSEC);
        assert_eq!(json["validation_baseline"], VALIDATION_BASELINE);
        assert_eq!(json["changelog_url"], DEFAULT_CHANGELOG_URL);
        assert_eq!(json["node_policy_id"], NODE_POLICY_ID);
        let bodies = json["supported_bodies"].as_array().expect("supported_bodies array");
        assert_eq!(bodies.len(), supported_celestial_bodies().len());
    }

    #[tokio::test]
    async fn protected_route_rejects_missing_api_key() {
        let app = test_app();
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

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = to_bytes(response.into_body(), usize::MAX).await.expect("body must be readable");
        let json: Value = serde_json::from_slice(&body).expect("body must be valid json");
        assert_eq!(json, serde_json::json!({ "error": "missing_api_key" }));
    }

    #[tokio::test]
    async fn protected_route_rejects_invalid_api_key() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/positions")
                    .header(API_KEY_HEADER, "wrong-key")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"julian_day":2451545.0,"bodies":["moon"]}"#))
                    .expect("request must build"),
            )
            .await
            .expect("response must succeed");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = to_bytes(response.into_body(), usize::MAX).await.expect("body must be readable");
        let json: Value = serde_json::from_slice(&body).expect("body must be valid json");
        assert_eq!(json, serde_json::json!({ "error": "invalid_api_key" }));
    }

    #[tokio::test]
    async fn protected_route_accepts_valid_bearer_token() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/positions")
                    .header(AUTHORIZATION_HEADER, format!("Bearer {TEST_API_KEY}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"julian_day":2451545.0,"bodies":["moon"]}"#))
                    .expect("request must build"),
            )
            .await
            .expect("response must succeed");

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn api_auth_config_parses_trimmed_csv_keys() {
        let auth = ApiAuthConfig::from_csv(Some(" first-key,second-key ,, third-key "));

        assert!(auth.is_valid_key("first-key"));
        assert!(auth.is_valid_key("second-key"));
        assert!(auth.is_valid_key("third-key"));
        assert!(!auth.is_valid_key(""));
    }

    #[test]
    fn app_router_reads_valid_api_keys_from_env() {
        let _guard = env_lock();
        std::env::set_var(VALID_API_KEYS_ENV_VAR, "env-key");

        let auth = ApiAuthConfig::from_env();

        assert!(auth.is_valid_key("env-key"));
        std::env::remove_var(VALID_API_KEYS_ENV_VAR);
    }

    #[tokio::test]
    async fn dasha_endpoint_returns_deterministic_payload() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/dasha")
                    .header(API_KEY_HEADER, TEST_API_KEY)
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

    #[tokio::test]
    async fn dasha_endpoint_omits_timeline_by_default() {
        // Backward compatibility: a dasha_v1 caller (no include_timeline) gets
        // the original payload shape; `timeline` is absent on the wire.
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/dasha")
                    .header(API_KEY_HEADER, TEST_API_KEY)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"moon_sidereal_longitude_deg":15.0,"birth_time_utc_rfc3339":"2024-01-01T00:00:00Z"}"#,
                    ))
                    .expect("request must build"),
            )
            .await
            .expect("response must succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.expect("body must be readable");
        let json: Value = serde_json::from_slice(&body).expect("body must be valid json");
        assert!(json["data"]["dasha"].is_object());
        assert_eq!(json["data"]["schema_version"], "dasha_v1");
        assert!(json["data"].get("timeline").is_none() || json["data"]["timeline"].is_null());
    }

    #[tokio::test]
    async fn dasha_endpoint_returns_timeline_when_requested() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/dasha")
                    .header(API_KEY_HEADER, TEST_API_KEY)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{
                            "moon_sidereal_longitude_deg": 99.586412769677720,
                            "birth_time_utc_rfc3339": "2000-01-01T12:00:00Z",
                            "as_of_utc_rfc3339": "2005-01-01T12:00:00Z",
                            "include_timeline": true
                        }"#,
                    ))
                    .expect("request must build"),
            )
            .await
            .expect("response must succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.expect("body must be readable");
        let json: Value = serde_json::from_slice(&body).expect("body must be valid json");
        assert_eq!(json["data"]["schema_version"], "dasha_v2");
        let timeline = &json["data"]["timeline"];
        assert!(timeline.is_object());
        assert_eq!(timeline["mahadashas"].as_array().unwrap().len(), 9);
        assert_eq!(timeline["antardashas"].as_array().unwrap().len(), 81);
        assert_eq!(timeline["current_antar_pratyantars"].as_array().unwrap().len(), 9);
        // The "current" field must agree with the standalone "dasha" field.
        assert_eq!(json["data"]["dasha"]["maha"]["lord"], json["data"]["current"]["maha"]["lord"]);
    }
}
