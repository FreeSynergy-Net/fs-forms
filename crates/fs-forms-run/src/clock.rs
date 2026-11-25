//! The clock of a view: the ONE way a window learns what "now" is. A view reads the time through the port [`Clock`] and nowhere
//! else (the test `tests/no_wall_clock.rs` keeps the host free of direct reads), so a picture or a test can say what "now" is
//! and the view never knows the difference.
//!
//! Pattern: Strategy / Port (`Clock`) with two adapters: [`SystemClock`] (the real wall clock, the default of every window) and
//! [`FixedClock`] (one moment, for test data and pictures). `Records::clock` hands the clock of the data to the window, so sample
//! data bring their own "now" the way a real module brings the system clock.

use std::fmt::Debug;
use std::time::SystemTime;

use chrono::{DateTime, Utc};

/// A point in time, as an instant in UTC (RFC 3339 on the command line, `UTCDateTime` of JSCalendar in the data).
pub type Moment = DateTime<Utc>;

/// Pattern: Strategy / Port – what "now" is for a view.
pub trait Clock: Send + Sync + Debug {
    /// The moment the view takes as now.
    fn now(&self) -> Moment;
}

/// The real wall clock. The only place of the host that reads it.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Moment {
        #[allow(clippy::disallowed_methods)]
        // the one place that reads the wall clock (clippy.toml of this crate)
        Moment::from(SystemTime::now())
    }
}

/// One fixed moment: always the same "now" (the data of a sample set say when they were made, `--sample-now` says another one).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedClock(Moment);

impl FixedClock {
    /// A clock that always says `moment`.
    #[must_use]
    pub fn new(moment: Moment) -> Self {
        Self(moment)
    }
}

impl Clock for FixedClock {
    fn now(&self) -> Moment {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::{Clock, FixedClock, Moment, SystemClock};

    #[test]
    fn a_fixed_clock_says_its_moment_every_time() {
        let m: Moment = "2026-11-25T09:00:00Z".parse().expect("rfc3339");
        let c = FixedClock::new(m);
        assert_eq!((c.now(), c.now()), (m, m));
    }

    #[test]
    fn the_system_clock_is_after_the_year_2025() {
        let y2025: Moment = "2025-01-01T00:00:00Z".parse().expect("rfc3339");
        assert!(SystemClock.now() > y2025);
    }
}
