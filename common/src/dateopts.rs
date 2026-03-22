use std::ops::{Add, Sub};

use chrono::{DateTime, Datelike, Days, Months, TimeDelta, TimeZone, Utc};
use clap::Parser;
use eyre::Report;
use jiff::Timestamp;
use thiserror::Error;
use tracing::trace;

use crate::normalise_day;

/// Enum `DateOpts` provides various options for specifying date ranges or formats.
///
/// This allows you to specify a date or time range through different methods such as
/// specific days, weeks, months, or quick shortcuts like today or yesterday.
///
/// # Variants
///
/// * `From { begin: String, end: String }`
///    - Specifies a date range with a start date (`begin`) and an end date (`end`).
///
/// * `Day { date: String }`
///    - Specifies a single, specific day.
///
/// * `Week { num: i64 }`
///    - Specifies a specific week of the year, where `num` is the week number (1-53).
///
/// * `Month { num: u32 }`
///    - Specifies a specific month, where `num` is the month number (1-12).
///
/// * `Today`
///    - A shortcut to represent the current day as a date range.
///
/// * `Yesterday`
///    - A shortcut to represent the previous day as a date range.
///
/// ```rust
///
/// // Example: Using the Today option
/// use fetiche_common::DateOpts;
///
/// let today = DateOpts::Today;
///
/// // Example: Using the From option
/// let from_to = DateOpts::From {
///     begin: "2022-01-01".to_string(),
///     end: "2022-12-31".to_string(),
/// };
/// ```
///
#[derive(Clone, Debug, PartialEq, Parser)]
pub enum DateOpts {
    /// Basic from to
    From { begin: String, end: String },
    /// Specific day
    Day { date: String },
    /// Specific week
    Week { num: i64 },
    /// Specific month
    Month { num: u32 },
    /// Shortcut for today
    Today,
    /// Shortcut to yesterday
    Yesterday,
}

#[derive(Debug, Error)]
pub enum ErrDateOpts {
    #[error("bad date: {0}")]
    BadDate(String),
    #[error("bad week number: must be 1 <= {0} <= 53.")]
    BadWeekNumber(i64),
    #[error("cannot get date from system")]
    CannotGetDate,
}

impl From<Report> for ErrDateOpts {
    fn from(value: Report) -> Self {
        ErrDateOpts::BadDate(value.to_string())
    }
}

impl DateOpts {
    /// Parse the provided `DateOpts` and return a corresponding time interval `(begin, end)`.
    ///
    /// The returned interval will be a tuple of `jiff::Timestamp` values representing
    /// the start (`begin`) and end (`end`) of the specified range.
    ///
    /// # Arguments
    ///
    /// * `opts` - The `DateOpts` variant to be parsed.
    ///
    /// # Returns
    ///
    /// A `Result` containing:
    /// * `Ok((begin, end))` if the parsing succeeds, where `begin` and `end`
    ///   are `jiff::Timestamp` values representing the range.
    /// * `Err(ErrDateOpts)` if an error occurs during parsing, such as invalid date, week number,
    ///   or month value.
    ///
    /// # Errors
    ///
    /// Parsing errors may occur if:
    /// * The `DateOpts::Day` variant contains an invalid date string.
    /// * The `DateOpts::Week` variant specifies a week number outside the range of 1-53.
    /// * The `DateOpts::Month` variant specifies a month value outside the range of 1-12.
    /// * The `DateOpts::From` variant contains invalid date strings for either `begin` or `end`.
    ///
    /// # Examples
    ///
    /// Parsing "Today":
    /// ```rust
    /// use jiff::{Timestamp, ToSpan};
    /// use jiff::tz::TimeZone;
    /// use fetiche_common::DateOpts;
    ///
    /// let result = DateOpts::parse(DateOpts::Today).unwrap();
    /// let now = Timestamp::now().to_zoned(TimeZone::UTC);
    /// let begin = result.0.to_zoned(TimeZone::UTC);
    /// let end = result.1.to_zoned(TimeZone::UTC);
    ///
    /// assert_eq!(begin.date(), now.date());
    /// assert_eq!(result.1, result.0.checked_add(24.hours()).unwrap());
    /// ```
    ///
    /// Parsing "Yesterday":
    /// ```rust
    /// use jiff::{Timestamp, ToSpan};
    /// use jiff::tz::TimeZone;
    /// use fetiche_common::DateOpts;
    ///
    /// let result = DateOpts::parse(DateOpts::Yesterday).unwrap();
    /// let now = Timestamp::now().to_zoned(TimeZone::UTC);
    /// let begin = result.0.to_zoned(TimeZone::UTC);
    /// let end = result.1.to_zoned(TimeZone::UTC);
    ///
    /// assert_eq!(begin.date(), now.date().checked_sub(1.days()).unwrap());
    /// assert_eq!(end.date(), now.date());
    /// ```
    ///
    /// Parsing a specific date:
    /// ```rust
    /// use fetiche_common::DateOpts;
    ///
    /// let date = "2023-10-01";
    /// let result = DateOpts::parse(DateOpts::Day { date: date.into() }).unwrap();
    ///
    /// // Verify the parsed date range.
    /// use jiff::Timestamp;
    ///
    /// let begin: Timestamp = "2023-10-01T00:00:00Z".parse().unwrap();
    /// let expected_end: Timestamp = "2023-10-02T00:00:00Z".parse().unwrap();
    ///
    /// assert_eq!(result, (begin, expected_end));
    /// ```
    ///
    #[tracing::instrument]
    pub fn parse(opts: Self) -> Result<(Timestamp, Timestamp), ErrDateOpts> {
        Ok(match opts {
            DateOpts::Today => {
                trace!("got today true");
                let today = Utc::now();
                let begin = normalise_day(today)?;
                let end = begin.add(Days::new(1));
                trace!("today gives from {} to {}", begin, end);
                (to_timestamp(begin)?, to_timestamp(end)?)
            }
            DateOpts::Yesterday => {
                trace!("got yesterday true");
                let today = Utc::now();
                let yest = today.sub(Days::new(1));
                let begin = normalise_day(yest)?;
                let end = normalise_day(today)?;
                trace!("yesterday gives from {} to {}", begin, end);
                (to_timestamp(begin)?, to_timestamp(end)?)
            }
            DateOpts::Day { date } => {
                trace!("Got day {}", date);
                let begin = match dateparser::parse(&date) {
                    Ok(date) => date,
                    Err(_) => return Err(ErrDateOpts::BadDate(date)),
                };
                let begin = normalise_day(begin)?;
                let end = begin.add(Days::new(1));
                trace!("this day={} gives from {} to {}", date, begin, end);
                (to_timestamp(begin)?, to_timestamp(end)?)
            }
            DateOpts::Week { num } => {
                trace!("Got week {}", num);
                if num > 53 {
                    return Err(ErrDateOpts::BadWeekNumber(num).into());
                }
                let week = Utc::now();
                let begin: DateTime<Utc> =
                    Utc.with_ymd_and_hms(week.year(), 1, 1, 0, 0, 0).unwrap();
                let begin = begin
                    .checked_add_signed(TimeDelta::try_weeks(num - 1).unwrap())
                    .unwrap();
                let end = begin.add(Days::new(7));
                trace!("week={} is from {} to {}", num, begin, end);
                (to_timestamp(begin)?, to_timestamp(end)?)
            }
            DateOpts::From { begin, end } => {
                trace!("Got from {} to {}", begin, end);
                let begin = match dateparser::parse(&begin) {
                    Ok(date) => date,
                    Err(_) => return Err(ErrDateOpts::BadDate(begin)),
                };
                let end = match dateparser::parse(&end) {
                    Ok(date) => date,
                    Err(_) => return Err(ErrDateOpts::BadDate(end)),
                };
                let begin = normalise_day(begin)?;
                let end = normalise_day(end)?;

                trace!("begin={} end={}", begin, end);
                (to_timestamp(begin)?, to_timestamp(end)?)
            }
            DateOpts::Month { num } => {
                let now = Utc::now();
                let year = now.year();
                if num == 0 || num > 12 {
                    return Err(ErrDateOpts::BadDate(num.to_string()));
                }
                let begin: DateTime<Utc> = Utc.with_ymd_and_hms(year, num, 1, 0, 0, 0).unwrap();
                let end: DateTime<Utc> = begin.add(Months::new(1));

                trace!("begin={} end={}", begin, end);
                (to_timestamp(begin)?, to_timestamp(end)?)
            }
        })
    }
}

#[inline]
fn to_timestamp(dt: DateTime<Utc>) -> Result<Timestamp, ErrDateOpts> {
    Timestamp::from_second(dt.timestamp()).map_err(|e| ErrDateOpts::BadDate(e.to_string()))
}

#[cfg(test)]
mod test {
    use super::*;
    use test_pretty_log::test;

    fn to_ts(dt: DateTime<Utc>) -> Timestamp {
        Timestamp::from_second(dt.timestamp()).unwrap()
    }

    #[test]
    fn test_dateopts_parse_today() -> eyre::Result<()> {
        let opt = DateOpts::Today;
        let result = DateOpts::parse(opt);

        assert!(result.is_ok());
        let (begin, end) = result?;
        let now = Utc::now();
        let expected_begin = Utc
            .with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0)
            .unwrap();
        let expected_end = expected_begin + chrono::Duration::days(1);

        assert_eq!(begin, to_ts(expected_begin));
        assert_eq!(end, to_ts(expected_end));

        Ok(())
    }

    #[test]
    fn test_dateopts_parse_yesterday() -> eyre::Result<()> {
        let opt = DateOpts::Yesterday;
        let result = DateOpts::parse(opt);

        assert!(result.is_ok());
        let (begin, end) = result?;
        let now = Utc::now();
        let expected_begin = Utc
            .with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0)
            .unwrap()
            - chrono::Duration::days(1);
        let expected_end = expected_begin + chrono::Duration::days(1);

        assert_eq!(begin, to_ts(expected_begin));
        assert_eq!(end, to_ts(expected_end));

        Ok(())
    }

    #[test]
    fn test_dateopts_parse_specific_day() -> eyre::Result<()> {
        let opt = DateOpts::Day {
            date: "2023-10-15".into(),
        };
        let result = DateOpts::parse(opt);

        assert!(result.is_ok());
        let (begin, end) = result?;
        let expected_begin = dateparser::parse("2023-10-15 00:00:00 UTC").unwrap();
        let expected_end = expected_begin + chrono::Duration::days(1);

        assert_eq!(begin, to_ts(expected_begin));
        assert_eq!(end, to_ts(expected_end));

        Ok(())
    }

    #[test]
    fn test_dateopts_parse_invalid_day() {
        let opt = DateOpts::Day {
            date: "invalid-date".into(),
        };
        let result = DateOpts::parse(opt);

        assert!(result.is_err());
        if let Err(ErrDateOpts::BadDate(date)) = result {
            assert_eq!(date, "invalid-date");
        } else {
            panic!("Expected ErrDateOpts::BadDate error");
        }
    }

    #[test]
    fn test_dateopts_parse_week() -> eyre::Result<()> {
        let opt = DateOpts::Week { num: 5 };
        let result = DateOpts::parse(opt);

        assert!(result.is_ok());
        let (begin, end) = result?;

        let now = Utc::now();
        // Find the Monday of ISO week 5 using chrono
        //
        let jan1 = Utc.with_ymd_and_hms(now.year(), 1, 1, 0, 0, 0).unwrap();
        let jan1_weekday = jan1.weekday().num_days_from_monday();

        // ISO week 1 is the week with the first Thursday (i.e., the week containing Jan 4)
        //
        let week1_monday = jan1 - chrono::Duration::days(jan1_weekday as i64)
            + if jan1_weekday <= 3 {
                chrono::Duration::days(0)
            } else {
                chrono::Duration::days(7)
            };
        let expected_begin = week1_monday + chrono::Duration::weeks(4);
        let expected_end = expected_begin + chrono::Duration::days(7);

        assert_eq!(begin, to_ts(expected_begin));
        assert_eq!(end, to_ts(expected_end));

        Ok(())
    }

    #[test]
    fn test_dateopts_parse_invalid_week() {
        let opt = DateOpts::Week { num: 54 };
        let result = DateOpts::parse(opt);

        assert!(result.is_err());
        if let Err(ErrDateOpts::BadWeekNumber(week)) = result {
            assert_eq!(week, 54_i64);
        } else {
            panic!("Expected ErrDateOpts::BadWeekNumber error");
        }
    }

    #[test]
    fn test_dateopts_parse_month() -> eyre::Result<()> {
        let opt = DateOpts::Month { num: 3 };
        let result = DateOpts::parse(opt);

        assert!(result.is_ok());
        let (begin, end) = result?;
        let now = Utc::now();
        let expected_begin = Utc.with_ymd_and_hms(now.year(), 3, 1, 0, 0, 0).unwrap();
        let expected_end = Utc.with_ymd_and_hms(now.year(), 4, 1, 0, 0, 0).unwrap();

        assert_eq!(begin, to_ts(expected_begin));
        assert_eq!(end, to_ts(expected_end));

        Ok(())
    }

    #[test]
    fn test_dateopts_parse_invalid_month() {
        let opt = DateOpts::Month { num: 13 };
        let result = DateOpts::parse(opt);

        assert!(result.is_err());
        if let Err(ErrDateOpts::BadDate(month)) = result {
            assert_eq!(month, "13");
        } else {
            panic!("Expected ErrDateOpts::BadDate error");
        }
    }

    #[test]
    fn test_dateopts_parse() -> eyre::Result<()> {
        let opt = DateOpts::From {
            begin: "2022-06-14 00:00:00 UTC".into(),
            end: "2023-02-28 00:00:00 UTC".into(),
        };
        let r = DateOpts::parse(opt);

        assert!(r.is_ok());
        let (b, e) = r.unwrap();
        assert_eq!(
            to_ts(dateparser::parse("2022-06-14 00:00:00 UTC").unwrap()),
            b
        );
        assert_eq!(
            to_ts(dateparser::parse("2023-02-28 00:00:00 UTC").unwrap()),
            e
        );
        Ok(())
    }
}
