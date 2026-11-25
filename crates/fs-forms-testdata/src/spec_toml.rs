//! Reads the section `[sample]` of a description: what the author of the description wants as test data.
//!
//! ```toml
//! [sample]
//! seed = 1
//! anchor = { month = "2026-05", zone = "Europe/Berlin" }   # or week = "2026-W22", or day = "2026-05-12"
//! forbid = [ { command = "move" } ]                        # optional; with `record = "urn:fs-sample:..."` for one record
//! [[sample.set]]
//! source = 1                                               # the number of the `[[source]]`
//! form = "bars-in-time@^1.0.0"
//! type = "Event"                                           # optional; if given it must be the type of the source
//! count = 8                                                # optional, default 8
//! distinct = { lane = 2 }
//! empty = { lane = 1 }
//! cases = [ { case = "all-day", times = 2 }, { case = "multi-day", days = 3, times = 1 } ]
//! invalid = 1
//!
//! [[sample.set]]                                           # fixed data instead of the generator (a named source, see `fixed`)
//! source = 1
//! form = "bars-in-time@^1.1.0"
//! fixed = "events.sample"                                  # excludes count, distinct, empty, cases, invalid (`fixed-conflict`)
//! ```
//!
//! Pattern: Adapter (TOML text in, the value objects of `spec` out; serde with `deny_unknown_fields`: an unknown key is an error, never
//! ignored). Every problem is `section-bad` with the line; the rest of the description is not read here (the exchange reader does that).

use std::collections::BTreeMap;

use fs_types::Zone;
use serde::Deserialize;

use crate::case::{Case, IntervalCase};
use crate::error::{Failure, SampleCode, fail};
use crate::spec::{Anchor, Forbid, Period, SetWish};

/// What `[sample]` says. Every part may be missing: the defaults of the declaration fill it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SampleToml {
    /// The seed, if written.
    pub seed: Option<u64>,
    /// The anchor, if written.
    pub anchor: Option<Anchor>,
    /// The commands the module refuses.
    pub forbid: Vec<Forbid>,
    /// The wishes, in file order.
    pub sets: Vec<SourceWish>,
}

/// One `[[sample.set]]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceWish {
    /// The number of the `[[source]]` (counts from 1).
    pub source: u32,
    /// The type the wish names, if written; the wish itself carries `""` until the description fills it.
    pub data_type: Option<String>,
    /// The wish (`data_type` still empty).
    pub wish: SetWish,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    seed: Option<u64>,
    anchor: Option<RawAnchor>,
    #[serde(default)]
    forbid: Vec<RawForbid>,
    #[serde(default)]
    set: Vec<RawSet>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAnchor {
    month: Option<String>,
    week: Option<String>,
    day: Option<String>,
    zone: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawForbid {
    command: String,
    record: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSet {
    source: u32,
    form: String,
    #[serde(rename = "type")]
    data_type: Option<String>,
    count: Option<u32>,
    #[serde(default)]
    distinct: BTreeMap<String, u32>,
    #[serde(default)]
    empty: BTreeMap<String, u32>,
    #[serde(default)]
    cases: Vec<RawCase>,
    invalid: Option<u32>,
    fixed: Option<String>,
}

#[derive(Deserialize)]
#[serde(
    deny_unknown_fields,
    tag = "case",
    rename_all = "kebab-case",
    rename_all_fields = "kebab-case"
)]
enum RawCase {
    AllDay { times: u32 },
    MultiDay { days: u8, times: u32 },
    SameDay { per_day: u8, times: u32 },
    CrossPeriod { times: u32 },
    ZeroLength { times: u32 },
    LongText { n: u32 },
    NonLatin { n: u32 },
    Tree { depth: u8, roots: u8 },
    Empty {},
}

fn bad(what: impl Into<String>) -> Failure {
    fail(SampleCode::SectionBad, "what", what)
}

fn line_of(text: &str, at: usize) -> usize {
    text[..at.min(text.len())].matches('\n').count() + 1
}

/// Reads `[sample]` from the text of a description (the other keys are not looked at).
///
/// # Errors
/// `section-bad` when the text is no TOML, a key of `[sample]` is unknown, a value has the wrong kind, a number is out of range, the
/// anchor is not exactly one of month, week, day, or a case is not one of the closed list; `zone-unknown` for a zone the database lacks.
pub fn read_sample(text: &str) -> Result<Option<SampleToml>, Failure> {
    let mut table: toml::Table = text.parse().map_err(|e: toml::de::Error| {
        let line = e.span().map_or(0, |s| line_of(text, s.start));
        bad(format!("line {line}: {}", e.message()))
    })?;
    let Some(section) = table.remove("sample") else {
        return Ok(None);
    };
    let raw: Raw = section
        .try_into()
        .map_err(|e: toml::de::Error| bad(e.message().to_owned()))?;
    Ok(Some(SampleToml {
        seed: raw.seed,
        anchor: raw.anchor.as_ref().map(anchor).transpose()?,
        forbid: raw
            .forbid
            .into_iter()
            .map(|f| Forbid {
                command: f.command,
                record: f.record,
            })
            .collect(),
        sets: raw.set.into_iter().map(set).collect::<Result<_, _>>()?,
    }))
}

fn number<T: std::str::FromStr>(part: Option<&str>, what: &str) -> Result<T, Failure> {
    part.and_then(|p| p.parse().ok())
        .ok_or_else(|| bad(format!("anchor: {what}")))
}

/// All parts of the anchor are used: a rest (`2026-05-17` as a month) is an error, never dropped.
fn whole(parts: &mut std::str::Split<'_, char>, what: &str) -> Result<(), Failure> {
    match parts.next() {
        None => Ok(()),
        Some(_) => Err(bad(format!("anchor: {what}"))),
    }
}

fn anchor(raw: &RawAnchor) -> Result<Anchor, Failure> {
    let period = match (&raw.month, &raw.week, &raw.day) {
        (Some(m), None, None) => {
            let mut p = m.split('-');
            let period = Period::Month(
                number(p.next(), "month `YYYY-MM`")?,
                number(p.next(), "month `YYYY-MM`")?,
            );
            whole(&mut p, "month `YYYY-MM`")?;
            period
        }
        (None, Some(w), None) => {
            let (year, week) = w.split_once("-W").unzip();
            Period::Week(
                number(year, "week `YYYY-Www`")?,
                number(week, "week `YYYY-Www`")?,
            )
        }
        (None, None, Some(d)) => {
            let mut p = d.split('-');
            let period = Period::Day(
                number(p.next(), "day `YYYY-MM-DD`")?,
                number(p.next(), "day `YYYY-MM-DD`")?,
                number(p.next(), "day `YYYY-MM-DD`")?,
            );
            whole(&mut p, "day `YYYY-MM-DD`")?;
            period
        }
        _ => return Err(bad("anchor: exactly one of month, week, day")),
    };
    let zone = Zone::iana(&raw.zone)
        .map_err(|_| fail(SampleCode::ZoneUnknown, "zone", raw.zone.clone()))?;
    crate::content::zone_of(&raw.zone)?; // the zone database is asked now: a wish never carries a zone nobody can read
    period.span()?; // a month 13 or a day 31 of a short month is `anchor-bad` here, not later
    Ok(Anchor { period, zone })
}

/// `fixed` stands alone: every wish of the generator next to it is `fixed-conflict`, never ignored.
fn set(raw: RawSet) -> Result<SourceWish, Failure> {
    if raw.fixed.is_some() {
        let written = [
            ("count", raw.count.is_some()),
            ("distinct", !raw.distinct.is_empty()),
            ("empty", !raw.empty.is_empty()),
            ("cases", !raw.cases.is_empty()),
            ("invalid", raw.invalid.is_some()),
        ];
        if let Some((what, _)) = written.iter().find(|(_, w)| *w) {
            return Err(fail(SampleCode::FixedConflict, "what", *what));
        }
    }
    // A fixed set has no count (`fixed` excludes it), a generated one the default.
    let count = raw.count.unwrap_or(if raw.fixed.is_some() {
        0
    } else {
        crate::defaults::DEFAULT_COUNT
    });
    let mut wish = SetWish::new(&raw.form, "", count);
    wish.distinct = raw.distinct.into_iter().collect();
    wish.empty = raw.empty.into_iter().collect();
    wish.invalid = raw.invalid.unwrap_or(0);
    wish.cases = raw.cases.iter().map(case).collect();
    wish.fixed = raw.fixed;
    Ok(SourceWish {
        source: raw.source,
        data_type: raw.data_type,
        wish,
    })
}

fn case(raw: &RawCase) -> Case {
    match *raw {
        RawCase::AllDay { times } => Case::Interval(IntervalCase::AllDay, times),
        RawCase::MultiDay { days, times } => Case::Interval(IntervalCase::MultiDay(days), times),
        RawCase::SameDay { per_day, times } => {
            Case::Interval(IntervalCase::SameDay(per_day), times)
        }
        RawCase::CrossPeriod { times } => Case::Interval(IntervalCase::CrossPeriod, times),
        RawCase::ZeroLength { times } => Case::Interval(IntervalCase::ZeroLength, times),
        RawCase::LongText { n } => Case::LongText(n),
        RawCase::NonLatin { n } => Case::NonLatin(n),
        RawCase::Tree { depth, roots } => Case::Tree { depth, roots },
        RawCase::Empty {} => Case::Empty,
    }
}
