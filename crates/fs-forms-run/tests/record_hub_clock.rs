//! The clock of the data passes through the hub (the path of `HostWindows`): the grip and the shared cache say the moment of the records
//! behind them, not the default system clock.
//!
//! Test: example-based. Mutant: `Shared::clock` or `Inner::clock` removed -> red (the trait default is the system clock).

use std::sync::Arc;

use fs_forms_run::{
    Clock, FakeModule, FixedClock, FormRecords, HoldCounts, Module, Moment, RecordHub, RecordKey,
    Records,
};

fn moment() -> Moment {
    "2026-11-25T09:00:00Z".parse().expect("rfc3339")
}

#[test]
fn the_clock_of_the_records_reaches_the_view_through_the_hub() {
    let fixed: Arc<dyn Clock> = Arc::new(FixedClock::new(moment()));
    let made = FakeModule::new(
        Arc::new(HoldCounts::default()),
        Box::new(move |_| {
            Arc::new(FormRecords {
                clock: Some(Arc::clone(&fixed)),
                ..FormRecords::default()
            }) as Arc<dyn Records>
        }),
    );
    let module: Arc<dyn Module> = made;
    let hub = RecordHub::new();
    let key = RecordKey {
        range: "net.freesynergy.Tasks".into(),
        scope: Some("urn:a".into()),
    };
    let grip = hub.attach(&key, &module).expect("attached");
    assert_eq!(
        grip.records().clock().now(),
        moment(),
        "the view of the grip"
    );
    assert_eq!(grip.cache().clock().now(), moment(), "the shared cache");
}
