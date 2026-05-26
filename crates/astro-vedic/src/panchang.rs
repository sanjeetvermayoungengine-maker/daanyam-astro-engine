//! Daily Panchang limbs from sidereal Sun/Moon longitudes and solar times.

use chrono::{DateTime, Datelike, TimeZone, Timelike, Utc};
use serde::{Deserialize, Serialize};

use astro_core::GeolocationInput;

use crate::sidereal::{sidereal_division, Nakshatra};

const YOGA_NAMES: [&str; 27] = [
    "Vishkumbha",
    "Preeti",
    "Ayushman",
    "Saubhagya",
    "Shobhana",
    "Atiganda",
    "Sukarma",
    "Dhriti",
    "Shoola",
    "Ganda",
    "Vriddhi",
    "Dhruva",
    "Vyaghata",
    "Harshana",
    "Vajra",
    "Siddhi",
    "Vyatipata",
    "Variyan",
    "Parigha",
    "Shiva",
    "Siddha",
    "Sadhya",
    "Shubha",
    "Shukla",
    "Brahma",
    "Indra",
    "Vaidhriti",
];

const KARANA_REPEATING: [&str; 7] =
    ["Bava", "Balava", "Kaulava", "Taitila", "Gara", "Vanija", "Vishti"];
const KARANA_FIXED: [&str; 4] = ["Kimstughna", "Shakuni", "Chatushpada", "Naga"];

const VARA_NAMES: [&str; 7] = [
    "Ravivara",
    "Somavara",
    "Mangalavara",
    "Budhavara",
    "Guruvara",
    "Shukravara",
    "Shanivara",
];

/// Rahu Kaal slot index (1–8) by weekday: Sun=0 … Sat=6.
const RAHU_KAAL_PART: [u8; 7] = [8, 2, 7, 5, 6, 4, 3];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PanchangTimeWindow {
    pub start_utc_rfc3339: String,
    pub end_utc_rfc3339: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PanchangDay {
    pub datetime_utc_rfc3339: String,
    pub tithi_num: u8,
    pub tithi_name: String,
    pub nakshatra: Nakshatra,
    pub nakshatra_pada: u8,
    pub yoga_index: u8,
    pub yoga_name: String,
    pub karana_name: String,
    pub vara_index: u8,
    pub vara_name: String,
    pub sunrise_utc_rfc3339: String,
    pub sunset_utc_rfc3339: String,
    pub solar_noon_utc_rfc3339: String,
    pub rahu_kaal: PanchangTimeWindow,
    pub abhijit_muhurat: PanchangTimeWindow,
    pub abhijit_valid: bool,
    pub moon_sidereal_longitude_deg: f64,
    pub sun_sidereal_longitude_deg: f64,
}

#[derive(Debug, Clone)]
pub struct SolarTimes {
    pub sunrise: DateTime<Utc>,
    pub sunset: DateTime<Utc>,
    pub solar_noon: DateTime<Utc>,
}

/// Compute Panchang for one instant using sidereal longitudes at that moment.
pub fn compute_panchang_day(
    utc: DateTime<Utc>,
    geo: &GeolocationInput,
    moon_sidereal_longitude_deg: f64,
    sun_sidereal_longitude_deg: f64,
) -> PanchangDay {
    let solar = solar_times_for_date(utc, geo.latitude_deg, geo.longitude_deg);
    let tithi = tithi_from_longitudes(moon_sidereal_longitude_deg, sun_sidereal_longitude_deg);
    let division = sidereal_division(moon_sidereal_longitude_deg);
    let yoga = yoga_from_longitudes(moon_sidereal_longitude_deg, sun_sidereal_longitude_deg);
    let karana = karana_from_tithi(tithi.num);
    let vara = vara_for_ist_weekday(utc);
    let rahu = rahu_kaal(&solar, vara.index);
    let abhijit = abhijit_muhurat(&solar, vara.index);

    PanchangDay {
        datetime_utc_rfc3339: utc.to_rfc3339(),
        tithi_num: tithi.num,
        tithi_name: tithi.name,
        nakshatra: division.nakshatra,
        nakshatra_pada: division.pada.0,
        yoga_index: yoga.index,
        yoga_name: yoga.name,
        karana_name: karana,
        vara_index: vara.index,
        vara_name: vara.name,
        sunrise_utc_rfc3339: solar.sunrise.to_rfc3339(),
        sunset_utc_rfc3339: solar.sunset.to_rfc3339(),
        solar_noon_utc_rfc3339: solar.solar_noon.to_rfc3339(),
        rahu_kaal: rahu,
        abhijit_muhurat: abhijit.window,
        abhijit_valid: abhijit.valid,
        moon_sidereal_longitude_deg,
        sun_sidereal_longitude_deg,
    }
}

#[derive(Debug, Clone, PartialEq)]
struct TithiInfo {
    num: u8,
    name: String,
}

#[derive(Debug, Clone, PartialEq)]
struct YogaInfo {
    index: u8,
    name: String,
}

#[derive(Debug, Clone, PartialEq)]
struct VaraInfo {
    index: u8,
    name: String,
}

fn tithi_from_longitudes(moon_lon: f64, sun_lon: f64) -> TithiInfo {
    let mut diff = moon_lon - sun_lon;
    if diff < 0.0 {
        diff += 360.0;
    }
    let num = (diff / 12.0).floor() as u8 + 1;
    let num = num.clamp(1, 30);
    let name = match num {
        1 => "Pratipada",
        2 => "Dvitiya",
        3 => "Tritiya",
        4 => "Chaturthi",
        5 => "Panchami",
        6 => "Shashthi",
        7 => "Saptami",
        8 => "Ashtami",
        9 => "Navami",
        10 => "Dashami",
        11 => "Ekadashi",
        12 => "Dwadashi",
        13 => "Trayodashi",
        14 => "Chaturdashi",
        15 => "Purnima",
        16 => "Pratipada (Kr)",
        17 => "Dvitiya (Kr)",
        18 => "Tritiya (Kr)",
        19 => "Chaturthi (Kr)",
        20 => "Panchami (Kr)",
        21 => "Shashthi (Kr)",
        22 => "Saptami (Kr)",
        23 => "Ashtami (Kr)",
        24 => "Navami (Kr)",
        25 => "Dashami (Kr)",
        26 => "Ekadashi (Kr)",
        27 => "Dwadashi (Kr)",
        28 => "Trayodashi (Kr)",
        29 => "Chaturdashi (Kr)",
        _ => "Amavasya",
    }
    .to_string();
    TithiInfo { num, name }
}

fn yoga_from_longitudes(moon_lon: f64, sun_lon: f64) -> YogaInfo {
    let mut sum = moon_lon + sun_lon;
    if sum >= 360.0 {
        sum -= 360.0;
    }
    let index = (sum / (360.0 / 27.0)).floor() as u8;
    let index = index.min(26);
    YogaInfo {
        index,
        name: YOGA_NAMES[index as usize].to_string(),
    }
}

fn karana_from_tithi(tithi_num: u8) -> String {
    let karana_index = (tithi_num as i32 - 1) * 2 + 1;
    if karana_index <= 0 {
        return KARANA_REPEATING[0].to_string();
    }
    if karana_index == 1 {
        return KARANA_FIXED[0].to_string();
    }
    if karana_index >= 58 {
        return KARANA_FIXED[(karana_index - 57) as usize].to_string();
    }
    KARANA_REPEATING[((karana_index - 2) % 7) as usize].to_string()
}

fn vara_for_ist_weekday(utc: DateTime<Utc>) -> VaraInfo {
    let ist_offset = chrono::FixedOffset::east_opt(5 * 3600 + 30 * 60).expect("IST offset");
    let ist = utc.with_timezone(&ist_offset);
    let index = ist.weekday().num_days_from_sunday() as u8;
    VaraInfo { index, name: VARA_NAMES[index as usize].to_string() }
}

/// NOAA-style solar times (ported from webapp `muhuratUtils.ts`).
pub fn solar_times_for_date(
    date: DateTime<Utc>,
    lat: f64,
    lon: f64,
) -> SolarTimes {
    let jd = julian_day_from_datetime(date);
    let n = jd - 2451545.0 + 0.0008;
    let j_star = n - lon / 360.0;
    let m = ((357.5291 + 0.98560028 * j_star) % 360.0 + 360.0) % 360.0;
    let c = 1.9148 * (m.to_radians()).sin()
        + 0.02 * (2.0 * m.to_radians()).sin()
        + 0.0003 * (3.0 * m.to_radians()).sin();
    let lambda = ((m + c + 180.0 + 102.9372) % 360.0 + 360.0) % 360.0;
    let j_transit = 2451545.0 + j_star + 0.0053 * (m.to_radians()).sin()
        - 0.0069 * (2.0 * lambda.to_radians()).sin();
    let sin_dec = (lambda.to_radians()).sin() * (23.4397_f64.to_radians()).sin();
    let cos_h = ((-0.833_f64.to_radians()).sin()
        - (lat.to_radians()).sin() * sin_dec)
        / ((lat.to_radians()).cos() * (sin_dec.asin()).cos());
    let h = cos_h.clamp(-1.0, 1.0).acos().to_degrees();
    let j_rise = j_transit - h / 360.0;
    let j_set = j_transit + h / 360.0;

    SolarTimes {
        sunrise: julian_to_utc(j_rise),
        sunset: julian_to_utc(j_set),
        solar_noon: julian_to_utc(j_transit),
    }
}

fn julian_day_from_datetime(dt: DateTime<Utc>) -> f64 {
    let y = dt.year();
    let m = dt.month() as i32;
    let d = dt.day() as i32;
    let (y, m) = if m <= 2 { (y - 1, m + 12) } else { (y, m) };
    let a = y / 100;
    let b = 2 - a + a / 4;
    let day_fraction =
        (dt.hour() as f64 + dt.minute() as f64 / 60.0 + dt.second() as f64 / 3600.0) / 24.0;
    (365.25 * (y + 4716) as f64).floor()
        + (30.6001 * (m + 1) as f64).floor()
        + d as f64
        + day_fraction
        + b as f64
        - 1524.5
}

fn julian_to_utc(jd: f64) -> DateTime<Utc> {
    let z = (jd + 0.5).floor();
    let f = jd + 0.5 - z;
    let mut a = z as i64;
    if z >= 2299161.0 {
        let alpha = ((z - 1867216.25) / 36524.25).floor();
        a = (z + 1.0 + alpha - (alpha / 4.0).floor()) as i64;
    }
    let b = a + 1524;
    let c = ((b as f64 - 122.1) / 365.25).floor() as i64;
    let d = (365.25 * c as f64).floor() as i64;
    let e = ((b - d) as f64 / 30.6001).floor() as i64;
    let day = b - d - (30.6001 * e as f64).floor() as i64;
    let month = if e < 14 { e - 1 } else { e - 13 };
    let year = if month > 2 { c - 4716 } else { c - 4715 };
    let hours = f * 24.0;
    let hour = hours.floor() as u32;
    let minutes = ((hours - hour as f64) * 60.0).floor() as u32;
    let seconds = ((((hours - hour as f64) * 60.0) - minutes as f64) * 60.0).round() as u32;
    Utc.with_ymd_and_hms(year as i32, month as u32, day as u32, hour, minutes, seconds)
        .single()
        .expect("valid julian conversion")
}

struct AbhijitResult {
    window: PanchangTimeWindow,
    valid: bool,
}

fn rahu_kaal(solar: &SolarTimes, weekday: u8) -> PanchangTimeWindow {
    let day_ms = (solar.sunset - solar.sunrise).num_milliseconds() as f64;
    let part_ms = day_ms / 8.0;
    let part_index = RAHU_KAAL_PART[weekday as usize] as f64;
    let start = solar.sunrise + chrono::Duration::milliseconds(((part_index - 1.0) * part_ms) as i64);
    let end = solar.sunrise + chrono::Duration::milliseconds((part_index * part_ms) as i64);
    PanchangTimeWindow {
        start_utc_rfc3339: start.to_rfc3339(),
        end_utc_rfc3339: end.to_rfc3339(),
        label: format!("{} – {}", format_ist(start), format_ist(end)),
    }
}

fn abhijit_muhurat(solar: &SolarTimes, weekday: u8) -> AbhijitResult {
    let start = solar.solar_noon - chrono::Duration::minutes(24);
    let end = solar.solar_noon + chrono::Duration::minutes(24);
    AbhijitResult {
        window: PanchangTimeWindow {
            start_utc_rfc3339: start.to_rfc3339(),
            end_utc_rfc3339: end.to_rfc3339(),
            label: format!("{} – {}", format_ist(start), format_ist(end)),
        },
        valid: weekday != 3,
    }
}

fn format_ist(dt: DateTime<Utc>) -> String {
    let ist_offset = chrono::FixedOffset::east_opt(5 * 3600 + 30 * 60).expect("IST offset");
    let ist = dt.with_timezone(&ist_offset);
    let h = ist.hour();
    let m = ist.minute();
    let ampm = if h < 12 { "AM" } else { "PM" };
    let h12 = if h % 12 == 0 { 12 } else { h % 12 };
    format!("{h12}:{m:02} {ampm}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tithi_first_day_when_moon_ahead_of_sun() {
        let t = tithi_from_longitudes(15.0, 0.0);
        assert_eq!(t.num, 2);
    }

    #[test]
    fn yoga_index_in_range() {
        let y = yoga_from_longitudes(100.0, 50.0);
        assert!(y.index < 27);
    }
}
