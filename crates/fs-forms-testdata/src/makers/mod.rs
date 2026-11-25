//! The strategies of the five kinds of channel that carry values.
//!
//! Pattern: Strategy (one `ValueMaker` per `Art`) + Registry (`standard` lists them).

mod flag;
mod number;
mod reference;
mod text;
mod time;

pub use text::word;

use crate::value::ValueMaker;

/// The strategies of text, flag, number, time and reference.
#[must_use]
pub fn standard() -> Vec<Box<dyn ValueMaker>> {
    vec![
        Box::new(text::TextMaker),
        Box::new(flag::FlagMaker),
        Box::new(number::NumberMaker),
        Box::new(time::TimeMaker),
        Box::new(reference::ReferenceMaker),
    ]
}
