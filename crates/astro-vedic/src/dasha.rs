use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::{sidereal_division, Nakshatra};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DashaLord {
    Ketu,
    Venus,
    Sun,
    Moon,
    Mars,
    Rahu,
    Jupiter,
    Saturn,
    Mercury,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DashaPeriod {
    pub lord: DashaLord,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VimshottariDasha {
    pub maha: DashaPeriod,
    pub antar: DashaPeriod,
    pub pratyantar: DashaPeriod,
}

const ORDER: [DashaLord; 9] = [
    DashaLord::Ketu,
    DashaLord::Venus,
    DashaLord::Sun,
    DashaLord::Moon,
    DashaLord::Mars,
    DashaLord::Rahu,
    DashaLord::Jupiter,
    DashaLord::Saturn,
    DashaLord::Mercury,
];

const YEARS: [i64; 9] = [7, 20, 6, 10, 7, 18, 16, 19, 17];
const DAYS_PER_YEAR: i64 = 365;

pub fn vimshottari_dasha(
    moon_sidereal_longitude_deg: f64,
    birth_time: DateTime<Utc>,
) -> VimshottariDasha {
    vimshottari_dasha_at(moon_sidereal_longitude_deg, birth_time, birth_time)
}

pub fn vimshottari_dasha_at(
    moon_sidereal_longitude_deg: f64,
    birth_time: DateTime<Utc>,
    as_of: DateTime<Utc>,
) -> VimshottariDasha {
    let division = sidereal_division(moon_sidereal_longitude_deg);
    let maha_index = nakshatra_to_dasha_index(division.nakshatra);
    let cycle_days = YEARS.iter().sum::<i64>() * DAYS_PER_YEAR;
    let elapsed_days = (as_of - birth_time).num_days().max(0);
    let cycle_offset_days = elapsed_days.rem_euclid(cycle_days);
    let maha = locate_period(maha_index, cycle_offset_days, cycle_days, birth_time);
    let maha_days = (maha.end - maha.start).num_days();
    let antar_offset_days = (as_of - maha.start).num_days().clamp(0, maha_days.saturating_sub(1));
    let antar = locate_subperiod(period_index(maha.lord), antar_offset_days, maha_days, maha.start);
    let antar_days = (antar.end - antar.start).num_days();
    let pratyantar_offset_days =
        (as_of - antar.start).num_days().clamp(0, antar_days.saturating_sub(1));
    let pratyantar =
        locate_subperiod(period_index(antar.lord), pratyantar_offset_days, antar_days, antar.start);

    VimshottariDasha { maha, antar, pratyantar }
}

fn locate_period(
    start_index: usize,
    offset_days: i64,
    _cycle_days: i64,
    cycle_start: DateTime<Utc>,
) -> DashaPeriod {
    let mut accumulated_days = 0;

    for step in 0..ORDER.len() {
        let index = (start_index + step) % ORDER.len();
        let duration_days = YEARS[index] * DAYS_PER_YEAR;
        if offset_days < accumulated_days + duration_days {
            let start = cycle_start + Duration::days(accumulated_days);
            let end = start + Duration::days(duration_days);
            return DashaPeriod { lord: ORDER[index], start, end };
        }
        accumulated_days += duration_days;
    }

    unreachable!("vimshottari cycle lookup must resolve within one cycle")
}

fn locate_subperiod(
    start_index: usize,
    offset_days: i64,
    parent_days: i64,
    parent_start: DateTime<Utc>,
) -> DashaPeriod {
    let mut accumulated_days = 0;

    for step in 0..ORDER.len() {
        let index = (start_index + step) % ORDER.len();
        let duration_days = subperiod_duration_days(parent_days, YEARS[index]);
        if offset_days < accumulated_days + duration_days || step == ORDER.len() - 1 {
            let start = parent_start + Duration::days(accumulated_days);
            let end = if step == ORDER.len() - 1 {
                parent_start + Duration::days(parent_days)
            } else {
                start + Duration::days(duration_days)
            };
            return DashaPeriod { lord: ORDER[index], start, end };
        }
        accumulated_days += duration_days;
    }

    unreachable!("vimshottari subperiod lookup must resolve within one parent period")
}

fn subperiod_duration_days(parent_days: i64, lord_years: i64) -> i64 {
    let duration_days = (parent_days * lord_years) / 120;
    duration_days.max(1)
}

fn period_index(lord: DashaLord) -> usize {
    ORDER
        .iter()
        .position(|candidate| *candidate == lord)
        .expect("dasha order must contain every lord")
}

fn nakshatra_to_dasha_index(nakshatra: Nakshatra) -> usize {
    match nakshatra {
        Nakshatra::Ashwini | Nakshatra::Magha | Nakshatra::Mula => 0,
        Nakshatra::Bharani | Nakshatra::PurvaPhalguni | Nakshatra::PurvaAshadha => 1,
        Nakshatra::Krittika | Nakshatra::UttaraPhalguni | Nakshatra::UttaraAshadha => 2,
        Nakshatra::Rohini | Nakshatra::Hasta | Nakshatra::Shravana => 3,
        Nakshatra::Mrigashira | Nakshatra::Chitra | Nakshatra::Dhanishta => 4,
        Nakshatra::Ardra | Nakshatra::Swati | Nakshatra::Shatabhisha => 5,
        Nakshatra::Punarvasu | Nakshatra::Vishakha | Nakshatra::PurvaBhadrapada => 6,
        Nakshatra::Pushya | Nakshatra::Anuradha | Nakshatra::UttaraBhadrapada => 7,
        Nakshatra::Ashlesha | Nakshatra::Jyeshtha | Nakshatra::Revati => 8,
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn computes_deterministic_vimshottari_sequence() {
        let birth_time = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let dasha = vimshottari_dasha(15.0, birth_time);

        assert_eq!(dasha.maha.lord, DashaLord::Venus);
        assert_eq!(dasha.antar.lord, DashaLord::Venus);
        assert_eq!(dasha.pratyantar.lord, DashaLord::Venus);
        assert!(dasha.maha.end > dasha.maha.start);
    }

    #[test]
    fn computes_current_dasha_for_as_of_instant() {
        let birth_time = Utc.with_ymd_and_hms(2000, 1, 1, 12, 0, 0).unwrap();
        let as_of = Utc.with_ymd_and_hms(2005, 1, 1, 12, 0, 0).unwrap();
        let dasha = vimshottari_dasha_at(99.586_412_769_677_72, birth_time, as_of);

        assert_eq!(dasha.maha.lord, DashaLord::Saturn);
        assert_eq!(dasha.antar.lord, DashaLord::Mercury);
        assert_eq!(dasha.pratyantar.lord, DashaLord::Jupiter);
        assert!(dasha.maha.start <= as_of);
        assert!(dasha.maha.end > as_of);
    }
}
