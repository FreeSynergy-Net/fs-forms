//! Strategy for channels of art `number`.
//!
//! Pattern: Strategy (`ValueMaker`).

use fs_interfaces::forms::{Art, Channel};
use rand::RngExt;

use crate::value::{MakeCx, Value, ValueMaker};

/// A whole number from 0 to 100 (a percentage, a position).
pub struct NumberMaker;

impl ValueMaker for NumberMaker {
    fn art(&self) -> Art {
        Art::Number
    }

    fn make(&self, cx: &mut MakeCx<'_>, _channel: &Channel) -> Option<Value> {
        Some(Value::Number(cx.rng.random_range(0..=100)))
    }
}
