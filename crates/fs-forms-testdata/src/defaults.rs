//! The wish a source gets when the description says nothing (`[sample]` missing): made from the declaration of the form alone.
//!
//! Pattern: Factory method (declaration in, wish out) + Specification by kind of channel: no name of a channel, a form or a program.
//! Rules: `count` 8; every required channel filled (the builder does it); an optional channel without a pool is filled in every second
//! record (the builder does it); a text-only channel takes 3 different values (`distinct`) and, if optional, is left empty in every
//! second record (`empty`); a form with a `Parent` relation gets chains of depth 2.

use fs_interfaces::forms::{Art, Channel, Form, Relation as Declared};

use crate::case::Case;
use crate::error::{Failure, SampleCode, fail};
use crate::spec::{Relation, SetWish};

/// How many records a default wish makes.
pub const DEFAULT_COUNT: u32 = 8;
/// How many different values a text-only channel takes.
pub const DEFAULT_DISTINCT: u32 = 3;

/// The channel of art `text` that names the record: the LAST required channel that takes nothing else than text. The catalog marks
/// no "choice" channels, so this is the one rule by kind: every other text-only channel is a category (a column, a lane, a state).
fn naming(form: &Form) -> Option<&Channel> {
    form.channels
        .iter()
        .rev()
        .find(|c| c.required && c.arts == [Art::Text])
}

/// The default wish for `form` (asked as `requirement`, e.g. `bars-in-time@^1.0.0`) with the records of `data_type`.
#[must_use]
pub fn wish(requirement: &str, form: &Form, data_type: &str) -> SetWish {
    let name = naming(form).map(|c| c.name.as_str());
    let mut wish = SetWish::new(requirement, data_type, DEFAULT_COUNT);
    for channel in form
        .channels
        .iter()
        .filter(|c| c.arts == [Art::Text] && Some(c.name.as_str()) != name)
    {
        wish = wish.distinct(&channel.name, DEFAULT_DISTINCT);
        if !channel.required {
            wish = wish.empty(&channel.name, DEFAULT_COUNT / 2);
        }
    }
    if form
        .relations
        .iter()
        .any(|r| matches!(r, Declared::Parent { .. }))
    {
        wish = wish.case(Case::Tree {
            depth: 2,
            roots: u8::try_from(DEFAULT_COUNT / 2).unwrap_or(1),
        });
    }
    wish
}

/// The relations of the declaration as the builder wants them (until the builder reads `Form::relations` itself).
///
/// # Errors
/// `unsupported` for a kind of relation this crate does not know yet (the catalog may add kinds): never dropped silently.
pub fn relations(form: &Form) -> Result<Vec<Relation>, Failure> {
    form.relations
        .iter()
        .map(|r| match r {
            Declared::Interval {
                start,
                end,
                all_day,
            } => Ok(Relation::order(start, end, all_day.as_deref())),
            Declared::Parent { channel } => Ok(Relation::Parent {
                child: channel.clone(),
            }),
            other => Err(fail(
                SampleCode::Unsupported,
                "what",
                format!("relation {other:?}"),
            )),
        })
        .collect()
}
