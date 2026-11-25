//! The registry of the standard strategies.
//!
//! Pattern: Registry (`standard` lists one `ValueMaker` per kind of channel).

use super::{flag, number, reference, text, time};
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
