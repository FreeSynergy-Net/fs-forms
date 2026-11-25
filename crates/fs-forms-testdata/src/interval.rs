//! Places the pair of times of an `Order` relation: the hand-made cases and the ordinary record.
//!
//! Pattern: Strategy (one placement per `IntervalCase`, chosen by `match`; the list of cases is closed).

use chrono::{Duration, NaiveDate};
use rand::RngExt;
use rand::rngs::ChaCha8Rng;

use crate::case::IntervalCase;
use crate::error::{Failure, SampleCode, fail};
use crate::spec::Span;
use crate::value::Moment;

/// A start and an end, half open (`[from, to)`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pair {
    /// The start.
    pub from: Moment,
    /// The end.
    pub to: Moment,
    /// Whether the pair covers whole days (both are `full-date`).
    pub whole_days: bool,
}

fn at(day: NaiveDate, hour: u32, minute: u32) -> Moment {
    Moment::Local(day.and_hms_opt(hour, minute, 0).unwrap_or_default())
}

/// The day (offset from the first day of the period) where a group of `case` starts.
///
/// # Errors
/// `case-impossible` when the period is too short for the case.
pub fn day_for(case: IntervalCase, span: Span, rng: &mut ChaCha8Rng) -> Result<i64, Failure> {
    let last_start = match case {
        IntervalCase::CrossPeriod => return Ok(span.days - 1),
        IntervalCase::MultiDay(n) => span.days - i64::from(n),
        _ => span.days - 1,
    };
    if last_start < 0 {
        return Err(fail(
            SampleCode::CaseImpossible,
            "what",
            format!("{case:?} does not fit the period"),
        ));
    }
    Ok(rng.random_range(0..=last_start))
}

/// The pair of the `nth` record of a group of `case` that starts on day offset `day`.
#[must_use]
pub fn pair(case: IntervalCase, day: i64, nth: u32, span: Span) -> Pair {
    let first = span.first + Duration::days(day);
    let timed = |from: Moment, to: Moment| Pair {
        from,
        to,
        whole_days: false,
    };
    match case {
        IntervalCase::AllDay => Pair {
            from: Moment::Date(first),
            to: Moment::Date(first + Duration::days(1)),
            whole_days: true,
        },
        IntervalCase::MultiDay(n) => timed(
            at(first, 9, 0),
            at(first + Duration::days(i64::from(n) - 1), 17, 0),
        ),
        IntervalCase::SameDay(_) => timed(at(first, 6 + nth % 12, 0), at(first, 6 + nth % 12, 45)),
        IntervalCase::CrossPeriod => timed(at(first, 22, 0), at(first + Duration::days(1), 2, 0)),
        IntervalCase::ZeroLength => timed(at(first, 12, 0), at(first, 12, 0)),
    }
}

/// An ordinary record: one to three hours on one day of the period, never over midnight.
pub fn ordinary(span: Span, rng: &mut ChaCha8Rng) -> Pair {
    let day = span.first + Duration::days(rng.random_range(0..span.days));
    let hour = rng.random_range(8..=17);
    let hours = rng.random_range(1..=3);
    Pair {
        from: at(day, hour, 0),
        to: at(day, hour + hours, 0),
        whole_days: false,
    }
}
