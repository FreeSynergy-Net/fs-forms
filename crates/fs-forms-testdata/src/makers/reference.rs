//! Strategy for channels of art `reference`.
//!
//! Pattern: Strategy (`ValueMaker`).

use fs_interfaces::forms::{Art, Channel};
use rand::RngExt;

use crate::value::{MakeCx, Value, ValueMaker};

/// Points at a record made earlier (so a reference can never be a cycle); the first record has none.
pub struct ReferenceMaker;

impl ValueMaker for ReferenceMaker {
    fn art(&self) -> Art {
        Art::Reference
    }

    fn make(&self, cx: &mut MakeCx<'_>, _channel: &Channel) -> Option<Value> {
        if cx.earlier.is_empty() {
            return None;
        }
        let at = cx.rng.random_range(0..cx.earlier.len());
        Some(Value::Reference(cx.earlier[at].clone()))
    }
}
