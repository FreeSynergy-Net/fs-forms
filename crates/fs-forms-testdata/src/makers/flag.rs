//! Strategy for channels of art `flag`.
//!
//! Pattern: Strategy (`ValueMaker`).

use fs_interfaces::forms::{Art, Channel};
use rand::RngExt;

use crate::value::{MakeCx, Value, ValueMaker};

/// Yes or no, evenly.
pub struct FlagMaker;

impl ValueMaker for FlagMaker {
    fn art(&self) -> Art {
        Art::Flag
    }

    fn make(&self, cx: &mut MakeCx<'_>, _channel: &Channel) -> Option<Value> {
        Some(Value::Flag(cx.rng.random_bool(0.5)))
    }
}
