//! The clock of the data as the clock of the components: the host's `Clock` (a `Moment`) seen through the port of
//! `fs_components_core::clock::Clock` (Unix seconds, UTC).
//!
//! Pattern: Adapter – `ClockPort` wraps the clock of the records (`Records::clock`, sample data or `--sample-now` included) so a
//! component reads "now" and "today" from the same clock as the host, never from the system time. The day is the UTC day.

use std::sync::Arc;

use fs_components_core::clock::Clock as ComponentClock;

use crate::clock::Clock;

/// The clock of the host, as the clock a component reads.
#[derive(Debug)]
pub(crate) struct ClockPort(pub(crate) Arc<dyn Clock>);

impl ComponentClock for ClockPort {
    fn now_unix(&self) -> i64 {
        self.0.now().timestamp()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use fs_components_core::clock::Clock as ComponentClock;

    use super::ClockPort;
    use crate::clock::FixedClock;

    #[test]
    fn the_component_clock_says_the_moment_of_the_host_clock() {
        let moment = "2025-11-25T09:00:00Z".parse().expect("rfc3339");
        let port = ClockPort(Arc::new(FixedClock::new(moment)));
        assert_eq!(port.now_unix(), 1_764_061_200);
        // 2025-11-25 is day 20417 since 1970-01-01
        assert_eq!(port.today_days(), 20_417);
    }
}
