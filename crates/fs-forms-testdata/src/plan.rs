//! The plan of a set: which record is which case, before any value is made.
//!
//! Pattern: Builder step (a pure function from the wish to a list of `Plan`; the cases are placed first, the rest is ordinary).

use rand::rngs::ChaCha8Rng;

use crate::case::{Case, IntervalCase};
use crate::error::{Failure, SampleCode, fail};
use crate::interval::day_for;
use crate::spec::{SetWish, Span};
use crate::value::Flavor;

/// What one record is.
#[derive(Clone, Copy, Debug)]
pub struct Plan {
    /// An interval case: the case, the day offset of its group and the place in the group.
    pub slot: Option<(IntervalCase, i64, u32)>,
    /// The look of the text.
    pub flavor: Flavor,
    /// The place (in the plan) of the parent record of a tree.
    pub parent: Option<usize>,
    /// The time pair is broken on purpose.
    pub invalid: bool,
}

impl Plan {
    fn plain() -> Self {
        Self {
            slot: None,
            flavor: Flavor::Plain,
            parent: None,
            invalid: false,
        }
    }
}

fn impossible(what: &str) -> Failure {
    fail(SampleCode::CaseImpossible, "what", what)
}

/// The records of a wish in order: cases first, then ordinary records up to `count`, then the broken ones.
///
/// # Errors
/// `case-impossible` for a case the form cannot take, `count-too-small` when the cases need more than `count`.
pub fn plan(
    wish: &SetWish,
    span: Span,
    rng: &mut ChaCha8Rng,
    has_order: bool,
    has_flag: bool,
    has_parent: bool,
) -> Result<Vec<Plan>, Failure> {
    if wish.cases.contains(&Case::Empty) {
        let alone = wish.cases.len() == 1 && wish.invalid == 0;
        return if alone {
            Ok(Vec::new())
        } else {
            Err(impossible("Empty with other cases or invalid rows"))
        };
    }
    let mut rows: Vec<Plan> = Vec::new();
    for case in &wish.cases {
        match *case {
            Case::Interval(kind, times) => {
                if !has_order {
                    return Err(impossible(
                        "an interval case needs an Order relation of the form",
                    ));
                }
                if kind == IntervalCase::AllDay && !has_flag {
                    return Err(impossible(
                        "AllDay needs the flag channel of the Order relation",
                    ));
                }
                for _ in 0..times {
                    let day = day_for(kind, span, rng)?;
                    rows.extend((0..kind.records()).map(|nth| Plan {
                        slot: Some((kind, day, nth)),
                        ..Plan::plain()
                    }));
                }
            }
            Case::LongText(n) => rows.extend((0..n).map(|_| Plan {
                flavor: Flavor::Long,
                ..Plan::plain()
            })),
            Case::NonLatin(n) => rows.extend((0..n).map(|_| Plan {
                flavor: Flavor::NonLatin,
                ..Plan::plain()
            })),
            Case::Tree { depth, roots } => {
                if !has_parent || depth == 0 || roots == 0 {
                    return Err(impossible(
                        "a tree needs a Parent relation, depth and roots of at least 1",
                    ));
                }
                for _ in 0..roots {
                    let mut before: Option<usize> = None;
                    for _ in 0..depth {
                        let this = rows.len();
                        rows.push(Plan {
                            parent: before,
                            ..Plan::plain()
                        });
                        before = Some(this);
                    }
                }
            }
            Case::Empty => {}
        }
    }
    let needed = u32::try_from(rows.len()).unwrap_or(u32::MAX);
    if needed > wish.count {
        return Err(
            fail(SampleCode::CountTooSmall, "needed", needed.to_string())
                .with_number("count", u64::from(wish.count)),
        );
    }
    rows.resize(usize::try_from(wish.count).unwrap_or(0), Plan::plain());
    rows.extend((0..wish.invalid).map(|_| Plan {
        invalid: true,
        ..Plan::plain()
    }));
    Ok(rows)
}
