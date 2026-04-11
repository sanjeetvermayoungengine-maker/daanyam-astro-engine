use chrono::{
    DateTime, Datelike, FixedOffset, LocalResult, NaiveDate, NaiveDateTime, TimeZone, Timelike, Utc,
};
use chrono_tz::Tz;
use thiserror::Error;

use crate::contracts::DateTimeInput;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TimeError {
    #[error("invalid timezone: {0}")]
    InvalidTimezone(String),
    #[error("ambiguous local datetime in timezone: {timezone}")]
    AmbiguousLocalDateTime { timezone: String },
    #[error("nonexistent local datetime in timezone: {timezone}")]
    NonexistentLocalDateTime { timezone: String },
}

pub fn resolve_datetime_input(input: &DateTimeInput) -> Result<DateTime<Utc>, TimeError> {
    match input {
        DateTimeInput::Utc(value) => Ok(value.utc.to_owned()),
        DateTimeInput::Offset(value) => Ok(value.datetime.with_timezone(&Utc)),
        DateTimeInput::Local(value) => resolve_local_datetime(&value.local, &value.timezone),
    }
}

pub fn resolve_local_datetime(
    local: &NaiveDateTime,
    timezone_name: &str,
) -> Result<DateTime<Utc>, TimeError> {
    let tz: Tz =
        timezone_name.parse().map_err(|_| TimeError::InvalidTimezone(timezone_name.to_owned()))?;

    match tz.from_local_datetime(local) {
        LocalResult::Single(dt) => Ok(dt.with_timezone(&Utc)),
        LocalResult::Ambiguous(_, _) => {
            Err(TimeError::AmbiguousLocalDateTime { timezone: timezone_name.to_owned() })
        }
        LocalResult::None => {
            Err(TimeError::NonexistentLocalDateTime { timezone: timezone_name.to_owned() })
        }
    }
}

pub fn utc_to_fixed_offset(datetime: DateTime<Utc>, offset_seconds: i32) -> DateTime<FixedOffset> {
    datetime.with_timezone(&FixedOffset::east_opt(offset_seconds).expect("valid fixed offset"))
}

pub fn julian_day(datetime: DateTime<Utc>) -> f64 {
    let y = datetime.year();
    let m = i32::try_from(datetime.month()).expect("month fits within i32");
    let d = i32::try_from(datetime.day()).expect("day fits within i32");

    let (year, month) = if m <= 2 { (y - 1, m + 12) } else { (y, m) };
    let a = year.div_euclid(100);
    let b = 2 - a + a.div_euclid(4);

    let hour = f64::from(datetime.hour());
    let minute = f64::from(datetime.minute());
    let second = f64::from(datetime.second());
    let nanos = f64::from(datetime.nanosecond());
    let fractional_day =
        (hour + (minute / 60.0) + ((second + nanos / 1_000_000_000.0) / 3600.0)) / 24.0;

    (365.25 * f64::from(year + 4716)).floor()
        + (30.6001 * f64::from(month + 1)).floor()
        + f64::from(d)
        + f64::from(b)
        - 1524.5
        + fractional_day
}

pub fn utc_julian_day_to_tdb_julian_day(jd_utc: f64) -> f64 {
    let jd_tt = jd_utc + (tai_minus_utc_seconds(jd_utc) + 32.184) / 86_400.0;
    let mean_anomaly = (357.53 + 0.985_600_3 * (jd_tt - 2_451_545.0)).to_radians();
    let tdb_offset_days =
        (0.001_658 * mean_anomaly.sin() + 0.000_014 * (2.0 * mean_anomaly).sin()) / 86_400.0;
    jd_tt + tdb_offset_days
}

pub fn naive_datetime(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(year, month, day)
        .expect("valid date")
        .and_hms_opt(hour, minute, second)
        .expect("valid time")
}

fn tai_minus_utc_seconds(jd_utc: f64) -> f64 {
    const LEAP_SECONDS: &[(f64, f64)] = &[
        (2_441_317.5, 10.0),
        (2_441_499.5, 11.0),
        (2_441_683.5, 12.0),
        (2_442_048.5, 13.0),
        (2_442_413.5, 14.0),
        (2_442_778.5, 15.0),
        (2_443_144.5, 16.0),
        (2_443_509.5, 17.0),
        (2_443_874.5, 18.0),
        (2_444_239.5, 19.0),
        (2_444_786.5, 20.0),
        (2_445_151.5, 21.0),
        (2_445_516.5, 22.0),
        (2_446_247.5, 23.0),
        (2_447_161.5, 24.0),
        (2_447_892.5, 25.0),
        (2_448_257.5, 26.0),
        (2_448_804.5, 27.0),
        (2_449_169.5, 28.0),
        (2_449_534.5, 29.0),
        (2_450_083.5, 30.0),
        (2_450_630.5, 31.0),
        (2_451_179.5, 32.0),
        (2_453_736.5, 33.0),
        (2_454_832.5, 34.0),
        (2_456_109.5, 35.0),
        (2_457_204.5, 36.0),
        (2_457_754.5, 37.0),
    ];

    let mut offset = 0.0;
    for &(effective_jd_utc, tai_minus_utc) in LEAP_SECONDS {
        if jd_utc >= effective_jd_utc {
            offset = tai_minus_utc;
        } else {
            break;
        }
    }
    offset
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::contracts::{DateTimeInput, LocalDateTimeInput};

    #[test]
    fn resolves_valid_timezone_deterministically() {
        let input = DateTimeInput::Local(LocalDateTimeInput {
            local: naive_datetime(2024, 1, 15, 10, 30, 0),
            timezone: "Asia/Kolkata".to_owned(),
        });
        let utc = resolve_datetime_input(&input).expect("must resolve");
        assert_eq!(utc, Utc.with_ymd_and_hms(2024, 1, 15, 5, 0, 0).unwrap());
    }

    #[test]
    fn rejects_invalid_timezone() {
        let err = resolve_local_datetime(&naive_datetime(2024, 1, 15, 10, 30, 0), "Mars/Olympus")
            .expect_err("must reject invalid timezone");
        assert_eq!(err, TimeError::InvalidTimezone("Mars/Olympus".to_owned()));
    }

    #[test]
    fn rejects_ambiguous_dst_local_datetime() {
        let err =
            resolve_local_datetime(&naive_datetime(2024, 11, 3, 1, 30, 0), "America/New_York")
                .expect_err("must reject ambiguous local datetime");
        assert_eq!(
            err,
            TimeError::AmbiguousLocalDateTime { timezone: "America/New_York".to_owned() }
        );
    }

    #[test]
    fn rejects_nonexistent_dst_local_datetime() {
        let err =
            resolve_local_datetime(&naive_datetime(2024, 3, 10, 2, 30, 0), "America/New_York")
                .expect_err("must reject nonexistent local datetime");
        assert_eq!(
            err,
            TimeError::NonexistentLocalDateTime { timezone: "America/New_York".to_owned() }
        );
    }

    #[test]
    fn julian_day_matches_j2000_epoch() {
        let dt = Utc.with_ymd_and_hms(2000, 1, 1, 12, 0, 0).unwrap();
        let jd = julian_day(dt);
        assert!((jd - 2_451_545.0).abs() < 1e-9);
    }

    #[test]
    fn converts_utc_julian_day_to_tdb() {
        let jd_tdb = utc_julian_day_to_tdb_julian_day(2_451_545.0);
        assert!(jd_tdb > 2_451_545.0);
        assert!(jd_tdb < 2_451_545.001);
    }
}
