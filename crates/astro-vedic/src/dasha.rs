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

/// Full Vimshottari timeline for a chart.
///
/// Maha + Antar periods are returned for the entire 120-year cycle starting at
/// `birth_time`. Pratyantars are returned only for the currently-active Antar
/// (the one containing `as_of`), keeping the payload small enough for a single
/// HTTP response without sacrificing user-visible precision.
///
/// Schema version: `dasha_v2`. The original `VimshottariDasha` snapshot
/// (`dasha_v1`) remains available via the `current` field on the API payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VimshottariTimeline {
    /// All 9 Mahadashas, in chronological order starting from the birth Maha.
    pub mahadashas: Vec<DashaPeriod>,
    /// All 81 Antardashas (9 per Mahadasha), in chronological order.
    pub antardashas: Vec<DashaPeriod>,
    /// The 9 Pratyantars within the currently-active Antar (or empty if the
    /// `as_of` instant falls outside the cycle, which shouldn't happen given
    /// the 120-year span but is handled defensively).
    pub current_antar_pratyantars: Vec<DashaPeriod>,
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

/// Build the full Vimshottari timeline for a chart.
///
/// Returns:
///   - The current snapshot (Maha + Antar + Pratyantar at `as_of`).
///   - The full timeline (all Mahas + all Antars for the 120-yr cycle, plus
///     Pratyantars within the current Antar).
///
/// The two outputs share the same period instances where they overlap, so a
/// caller can render "current period" cards and "timeline" lists without
/// worrying about start/end drift between them.
pub fn vimshottari_timeline(
    moon_sidereal_longitude_deg: f64,
    birth_time: DateTime<Utc>,
    as_of: DateTime<Utc>,
) -> (VimshottariDasha, VimshottariTimeline) {
    let division = sidereal_division(moon_sidereal_longitude_deg);
    let start_index = nakshatra_to_dasha_index(division.nakshatra);

    // Mahadashas: 9 contiguous Mahas starting from the birth nakshatra's lord.
    let mut mahadashas = Vec::with_capacity(ORDER.len());
    let mut cursor = birth_time;
    for step in 0..ORDER.len() {
        let idx = (start_index + step) % ORDER.len();
        let duration_days = YEARS[idx] * DAYS_PER_YEAR;
        let start = cursor;
        let end = start + Duration::days(duration_days);
        mahadashas.push(DashaPeriod { lord: ORDER[idx], start, end });
        cursor = end;
    }

    // Antardashas: every Maha is sub-divided into 9 Antars. We pre-compute the
    // full 81-period list to give Patrika rendering a single contiguous array.
    let mut antardashas = Vec::with_capacity(ORDER.len() * ORDER.len());
    for maha in &mahadashas {
        let parent_days = (maha.end - maha.start).num_days();
        let mut antar_cursor = maha.start;
        let maha_lord_index = period_index(maha.lord);
        for step in 0..ORDER.len() {
            let idx = (maha_lord_index + step) % ORDER.len();
            // For the last Antar, snap to the Maha's end so rounding errors
            // never produce gaps or overlaps between adjacent Mahas.
            let antar_end = if step == ORDER.len() - 1 {
                maha.end
            } else {
                antar_cursor + Duration::days(subperiod_duration_days(parent_days, YEARS[idx]))
            };
            antardashas.push(DashaPeriod {
                lord: ORDER[idx],
                start: antar_cursor,
                end: antar_end,
            });
            antar_cursor = antar_end;
        }
    }

    // Resolve the current Maha + Antar + Pratyantar using the existing snapshot
    // function. This guarantees the current period objects are byte-identical
    // to one of the entries in `mahadashas` / `antardashas` (modulo the final
    // Antar's snap-to-end), so consumers can compare by (lord, start) keys.
    let current = vimshottari_dasha_at(moon_sidereal_longitude_deg, birth_time, as_of);

    // Pratyantars: derive only for the current Antar (full cycle would be 729
    // entries, an order of magnitude more than the consumer needs today).
    let mut current_antar_pratyantars = Vec::with_capacity(ORDER.len());
    let antar_days = (current.antar.end - current.antar.start).num_days();
    let mut pratyantar_cursor = current.antar.start;
    let antar_lord_index = period_index(current.antar.lord);
    for step in 0..ORDER.len() {
        let idx = (antar_lord_index + step) % ORDER.len();
        let pratyantar_end = if step == ORDER.len() - 1 {
            current.antar.end
        } else {
            pratyantar_cursor + Duration::days(subperiod_duration_days(antar_days, YEARS[idx]))
        };
        current_antar_pratyantars.push(DashaPeriod {
            lord: ORDER[idx],
            start: pratyantar_cursor,
            end: pratyantar_end,
        });
        pratyantar_cursor = pratyantar_end;
    }

    let timeline = VimshottariTimeline {
        mahadashas,
        antardashas,
        current_antar_pratyantars,
    };

    (current, timeline)
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

    #[test]
    fn timeline_has_nine_mahadashas_in_birth_order() {
        let birth_time = Utc.with_ymd_and_hms(2000, 1, 1, 12, 0, 0).unwrap();
        let as_of = Utc.with_ymd_and_hms(2005, 1, 1, 12, 0, 0).unwrap();
        let (_current, timeline) =
            vimshottari_timeline(99.586_412_769_677_72, birth_time, as_of);

        assert_eq!(timeline.mahadashas.len(), 9);
        // Mahas must be contiguous in time.
        for pair in timeline.mahadashas.windows(2) {
            assert_eq!(pair[0].end, pair[1].start);
        }
        // First Maha starts at birth.
        assert_eq!(timeline.mahadashas[0].start, birth_time);
        // Total cycle length is 120 years.
        let last_end = timeline.mahadashas.last().unwrap().end;
        let cycle_days = (last_end - birth_time).num_days();
        assert_eq!(cycle_days, 120 * 365);
    }

    #[test]
    fn timeline_antars_form_contiguous_eighty_one_period_sequence() {
        let birth_time = Utc.with_ymd_and_hms(1990, 6, 15, 0, 0, 0).unwrap();
        let as_of = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let (_current, timeline) = vimshottari_timeline(180.0, birth_time, as_of);

        assert_eq!(timeline.antardashas.len(), 9 * 9);
        // Each block of 9 Antars must fit exactly inside the corresponding Maha.
        for (maha_index, maha) in timeline.mahadashas.iter().enumerate() {
            let block = &timeline.antardashas[maha_index * 9..(maha_index + 1) * 9];
            assert_eq!(block.first().unwrap().start, maha.start);
            assert_eq!(block.last().unwrap().end, maha.end);
            // First Antar of any Maha is ruled by the Maha lord itself.
            assert_eq!(block.first().unwrap().lord, maha.lord);
        }
    }

    #[test]
    fn timeline_pratyantars_cover_current_antar_exactly() {
        let birth_time = Utc.with_ymd_and_hms(1985, 9, 20, 5, 30, 0).unwrap();
        let as_of = Utc.with_ymd_and_hms(2026, 5, 11, 0, 0, 0).unwrap();
        let (current, timeline) = vimshottari_timeline(245.7, birth_time, as_of);

        assert_eq!(timeline.current_antar_pratyantars.len(), 9);
        // First Pratyantar is the Antar lord itself.
        assert_eq!(
            timeline.current_antar_pratyantars.first().unwrap().lord,
            current.antar.lord
        );
        // Pratyantars must tile the current Antar exactly.
        assert_eq!(
            timeline.current_antar_pratyantars.first().unwrap().start,
            current.antar.start
        );
        assert_eq!(
            timeline.current_antar_pratyantars.last().unwrap().end,
            current.antar.end
        );
        for pair in timeline.current_antar_pratyantars.windows(2) {
            assert_eq!(pair[0].end, pair[1].start);
        }
    }

    #[test]
    fn timeline_current_matches_snapshot() {
        // The (Maha, Antar) returned by the timeline's "current" must match
        // what vimshottari_dasha_at returns standalone for the same as_of.
        let birth_time = Utc.with_ymd_and_hms(2000, 1, 1, 12, 0, 0).unwrap();
        let as_of = Utc.with_ymd_and_hms(2005, 1, 1, 12, 0, 0).unwrap();
        let standalone = vimshottari_dasha_at(99.586_412_769_677_72, birth_time, as_of);
        let (current, _) = vimshottari_timeline(99.586_412_769_677_72, birth_time, as_of);
        assert_eq!(standalone.maha.lord, current.maha.lord);
        assert_eq!(standalone.antar.lord, current.antar.lord);
        assert_eq!(standalone.pratyantar.lord, current.pratyantar.lord);
    }
}
