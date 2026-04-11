use chrono::{DateTime, FixedOffset, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EngineMode {
    Vedic,
    Western,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AyanamsaModel {
    Lahiri,
    Raman,
    Krishnamurti,
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HouseSystem {
    WholeSign,
    Placidus,
    Equal,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NodeMode {
    True,
    Mean,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocalDateTimeInput {
    pub local: NaiveDateTime,
    pub timezone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UtcDateTimeInput {
    pub utc: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OffsetDateTimeInput {
    pub datetime: DateTime<FixedOffset>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DateTimeInput {
    Local(LocalDateTimeInput),
    Utc(UtcDateTimeInput),
    Offset(OffsetDateTimeInput),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GeolocationInput {
    pub latitude_deg: f64,
    pub longitude_deg: f64,
    pub elevation_m: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct EngineConfig {
    pub mode: EngineMode,
    pub ayanamsa: AyanamsaModel,
    pub house_system: HouseSystem,
    pub node_mode: NodeMode,
    pub gravitational_deflection: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            mode: EngineMode::Vedic,
            ayanamsa: AyanamsaModel::Lahiri,
            house_system: HouseSystem::WholeSign,
            node_mode: NodeMode::True,
            gravitational_deflection: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CoordinateFrame {
    EclipticGeocentric,
    EquatorialGeocentric,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CelestialBody {
    Sun,
    Moon,
    Mercury,
    Venus,
    Mars,
    Jupiter,
    Saturn,
    Rahu,
    Ketu,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Observer {
    pub geo: GeolocationInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BodyPosition {
    pub body: CelestialBody,
    pub longitude_deg: f64,
    pub latitude_deg: f64,
    pub distance_au: Option<f64>,
    pub frame: CoordinateFrame,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct BodyComputationMeta {
    pub frame: String,
    pub observer: String,
    pub topocentric_applied: bool,
    pub kernel: String,
    pub kernel_notes: Option<String>,
    pub crate_version: String,
    pub light_time: bool,
    pub stellar_aberration: bool,
    pub gravitational_deflection: bool,
    pub motion_model: Option<String>,
    pub node_policy: Option<NodeMode>,
    pub ayanamsa_algorithm: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PositionResult {
    pub position: BodyPosition,
    pub computation_meta: BodyComputationMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HouseSet {
    pub system: HouseSystem,
    pub cusps_deg: Vec<f64>,
    pub ascendant_deg: f64,
    pub midheaven_deg: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResultMetadata {
    pub engine_mode: EngineMode,
    pub ayanamsa_used: AyanamsaModel,
    pub house_system: HouseSystem,
    pub gravitational_deflection: bool,
    pub engine_semantic_version: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComputationResult<T> {
    pub data: T,
    pub metadata: ResultMetadata,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_config_default_is_vedic_primary() {
        assert_eq!(EngineConfig::default().mode, EngineMode::Vedic);
    }

    #[test]
    fn contracts_serialize_stably() {
        let config = EngineConfig::default();
        let json = serde_json::to_string(&config).expect("config must serialize");
        assert_eq!(
            json,
            r#"{"mode":"vedic","ayanamsa":"lahiri","house_system":"whole_sign","node_mode":"true","gravitational_deflection":true}"#
        );
    }
}
