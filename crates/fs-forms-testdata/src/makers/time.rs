//! Strategy for channels of art `time` that no `Order` relation places.
//!
//! Pattern: Strategy (`ValueMaker`).

use fs_interfaces::forms::{Art, Channel};
use rand::RngExt;

use crate::value::{MakeCx, Moment, Value, ValueMaker};

/// A wall-clock time on a day of the period, between 08:00 and 17:00.
pub struct TimeMaker;

impl ValueMaker for TimeMaker {
    fn art(&self) -> Art {
        Art::Time
    }

    fn make(&self, cx: &mut MakeCx<'_>, _channel: &Channel) -> Option<Value> {
        let day = cx.rng.random_range(0..cx.span.days);
        let hour = cx.rng.random_range(8..=17);
        let at = (cx.span.first + chrono::Duration::days(day)).and_hms_opt(hour, 0, 0)?;
        Some(Value::Time(Moment::Local(at)))
    }
}
