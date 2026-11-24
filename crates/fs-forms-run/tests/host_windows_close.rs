//! What happens when a window goes, and when a question is not answered (review of O4a, M1 and S2).
//!
//! Tests: C1 the person closes a window AFTER a switch (the entry lives under a new id): entry, grip and hold fall, and the place can
//! be opened again; C2 a window that `close` took away reports `on_closed` LATE (as Blinc does) after the same place was opened
//! again: the late report must not touch the new window (ABA); C3 a question that is dismissed (or closed by the program) without an
//! answer ends with `done(Err(open-unanswered))` exactly once and lets the subscription and the owner go.
//!
//! Mutants (each run, each red as named): M1 `on_closed` finds the entry by the id it was shown under (as before) -> C1 and C2;
//! M2 the observer of the question ignores `Event::Closed` -> C3.

mod host_common;

use std::sync::{Arc, Mutex};

use fs_components_app::AppWidget;
use fs_forms_run::{
    Asked, Coded, OpenAsk, Surfaces, WhenOpen, WindowOpened as Opened, WindowOwner,
};
use host_common::{done, notes, req, world, world_with};

#[test]
fn c1_the_person_closes_a_window_after_a_switch() {
    let host = world();
    host.owner.open(req("board", "urn:a")).unwrap();
    let switched = host
        .owner
        .open(req("board", "urn:b").with_when_open(WhenOpen::Switch))
        .unwrap();
    assert!(matches!(switched, Opened::Switched { .. }), "{switched:?}");
    assert_eq!(
        (host.owner.open_now().len(), host.counts.get()),
        (1, (1, 0))
    );
    let widget = host.shown.widgets.lock().unwrap()[0].1.clone();
    widget.quit(); // the person closes the window
    drop(widget);
    assert_eq!(host.owner.open_now(), [], "no entry of a dead window");
    assert_eq!(
        (host.counts.get(), host.owner.records_alive()),
        ((1, 1), 0),
        "the hold and the cache are gone"
    );
    assert!(
        matches!(
            host.owner.open(req("board", "urn:b")).unwrap(),
            Opened::New(_)
        ),
        "the place opens anew, it is not `Raised` for a window that is not there"
    );
}

/// Windows whose `on_closed` the test runs when it wants (Blinc runs it after the window went, not inside `quit`).
#[derive(Default)]
struct Late(Mutex<Vec<Box<dyn FnOnce() + Send>>>);
impl Surfaces for Late {
    fn show(&self, _app: AppWidget, on_closed: Box<dyn FnOnce() + Send>) -> Result<(), Coded> {
        self.0.lock().unwrap().push(on_closed);
        Ok(())
    }
}

#[test]
fn c2_a_late_report_of_a_closed_window_does_not_touch_the_window_that_came_after() {
    let late = Arc::new(Late::default());
    let (owner, counts, _) = world_with(Arc::clone(&late) as Arc<dyn Surfaces>);
    let Opened::New(first) = owner.open(req("board", "urn:a")).unwrap() else {
        panic!("new")
    };
    owner.close(&first).unwrap();
    let Opened::New(again) = owner.open(req("board", "urn:a")).unwrap() else {
        panic!("new")
    };
    assert_eq!(first, again, "the same place, the same id");
    let first_report = late.0.lock().unwrap().remove(0);
    first_report(); // the first window reports now
    assert_eq!(owner.open_now().len(), 1, "the new window stays");
    assert_eq!((counts.get(), owner.records_alive()), ((2, 1), 1));
    let second_report = late.0.lock().unwrap().remove(0);
    second_report();
    assert_eq!((owner.open_now().len(), counts.get()), (0, (2, 2)));
}

#[test]
fn c3_a_question_that_is_dismissed_ends_with_unanswered_and_lets_go() {
    let host = world();
    host.owner.open(req("board", "urn:a")).unwrap();
    let want = req("board", "urn:b").with_when_open(WhenOpen::Ask);
    assert_eq!(Arc::strong_count(&host.owner), 1);
    let (bell, _toast) = notes();
    let (seen, d) = done();
    let Asked::Pending(id) = host.owner.open_asking(want, &bell, 0, d).unwrap() else {
        panic!("a question")
    };
    assert_eq!(
        Arc::strong_count(&host.owner),
        2,
        "the question holds the owner"
    );
    bell.close(&id).unwrap(); // nobody will answer any more
    let got = seen.lock().unwrap();
    assert!(
        matches!(got.as_slice(), [Err(c)] if c.code == "host-notice-open-unanswered"),
        "done said it once: {got:?}"
    );
    assert_eq!(host.owner.open_now().len(), 1, "nothing was opened");
    drop(got);
    assert_eq!(Arc::strong_count(&host.owner), 1, "the subscription let go");
}
