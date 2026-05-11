//! Classical Vedic yoga detection — Phase 4 (engine v0.18+).
//!
//! Status: SCAFFOLD. Two example yogas are implemented as a pattern
//! (Gajakesari, Chandra-Mangal) and serve as the template for the remaining
//! 28+ yogas listed in `docs/engine/yogas-roadmap.md`. Each yoga is added by:
//!
//!   1. Creating `yogas/<yoga_name>.rs` that implements the `Yoga` trait.
//!   2. Registering the detector in `registry::all_yogas()`.
//!   3. Adding a golden-file test against a known chart in
//!      `tests/golden/yogas_<chart_name>.json`.
//!
//! Voice rules: each detected yoga returns a Daanyam-voice short description
//! suitable for inline rendering on the explainer + Patrika. No prediction;
//! Sanskrit nouns + everyday English verbs (see V3 strategy doc).

use serde::{Deserialize, Serialize};

use crate::Rashi;

mod chandra_mangal;
mod gajakesari;
mod registry;

pub use registry::{detect_yogas, all_yoga_keys};

/// A single detected yoga (or absence thereof at low strength).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DetectedYoga {
    /// Stable machine key, snake_case (e.g. "gajakesari", "chandra_mangal").
    pub key: String,
    /// Human-readable Sanskrit-rooted name.
    pub name: String,
    /// Planets that triggered the detection.
    pub planets_involved: Vec<String>,
    /// Houses (1-12) the planets sit in, when relevant.
    pub houses_involved: Vec<u8>,
    /// 0.0–1.0 confidence in the detection. 1.0 = textbook formation; lower
    /// values mark partial / probational forms that still pass the detector
    /// but warrant softer voice in the rendered Patrika.
    pub strength: f64,
    /// Daanyam-voice one-line description, ready for inline rendering.
    pub voice_line: String,
}

/// Minimal chart facts a yoga detector reads from. Constructed once per chart
/// request inside the API handler and passed by reference to every detector,
/// so each one stays O(N) in the chart size and tests can spin up fakes
/// without touching ephemeris code.
#[derive(Debug, Clone)]
pub struct YogaChartFacts {
    pub lagna_rashi: Rashi,
    /// Sidereal longitude (0..360) of each graha. None for grahas not computed.
    pub planet_longitudes: PlanetLongitudes,
    /// House number (1..12, whole-sign) for each graha.
    pub planet_houses: PlanetHouses,
}

#[derive(Debug, Clone, Default)]
pub struct PlanetLongitudes {
    pub sun: Option<f64>,
    pub moon: Option<f64>,
    pub mars: Option<f64>,
    pub mercury: Option<f64>,
    pub jupiter: Option<f64>,
    pub venus: Option<f64>,
    pub saturn: Option<f64>,
    pub rahu: Option<f64>,
    pub ketu: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct PlanetHouses {
    pub sun: Option<u8>,
    pub moon: Option<u8>,
    pub mars: Option<u8>,
    pub mercury: Option<u8>,
    pub jupiter: Option<u8>,
    pub venus: Option<u8>,
    pub saturn: Option<u8>,
    pub rahu: Option<u8>,
    pub ketu: Option<u8>,
}

/// Trait every individual yoga detector implements. The registry collects
/// trait objects and runs them in a fixed order so the response order is
/// stable across requests.
pub trait Yoga: Send + Sync {
    fn key(&self) -> &'static str;
    fn detect(&self, facts: &YogaChartFacts) -> Option<DetectedYoga>;
}

// ─── Helpers used by multiple detectors ─────────────────────────────────────

/// Rashi distance from `from` to `to`, counting `from` itself as 1
/// (Vedic convention). Returns 1..=12.
pub(crate) fn rashi_distance(from: Rashi, to: Rashi) -> u8 {
    let a = rashi_index(from) as i16;
    let b = rashi_index(to) as i16;
    ((b - a).rem_euclid(12) + 1) as u8
}

pub(crate) fn rashi_index(r: Rashi) -> u8 {
    match r {
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

/// House distance (1..=12), counting `from` as 1.
pub(crate) fn house_distance(from: u8, to: u8) -> u8 {
    let a = from as i16 - 1;
    let b = to as i16 - 1;
    ((b - a).rem_euclid(12) + 1) as u8
}

/// True if `a` and `b` are mutually kendra (1, 4, 7, 10 from each other).
pub(crate) fn is_mutual_kendra(a: u8, b: u8) -> bool {
    let d = house_distance(a, b);
    matches!(d, 1 | 4 | 7 | 10)
}
