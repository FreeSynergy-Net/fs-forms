//! The real owner `HostWindows` (stage 6, O4a, `Window` only) over doubles of its three ports: two windows of two frontends of ONE
//! program show ONE record; each holds its own connection, learns of a change only through its own `watch`, and the question
//! "this place shows another record" goes through `Notifications::ask` (no dialog of its own).
//!
//! Tests: H1 two windows, one record: one cache, two holds, a move in A shows in B (and in A again, through its own `watch`) and both
//! windows are woken; H2 the answer of the person: `switch` keeps window and hold and shows the other record, `window` opens a second
//! one, an answer of another question or an unknown one opens nothing; H3 `close` lets grip and hold fall, a window the person closes
//! does the same (`on_closed`), a failure to show leaves nothing behind; H4 what is not built says so (`InFrame`, `use_frontend`).
//!
//! Mutants (each makes the named test fail): H1 the hub tells nobody but the writer -> H1; M-open the answer `window` opens as
//! `switch` -> H2; M-close `close` forgets to drop the entry -> H3; M-sig the observer reacts to every answer, not only its own -> H2.

mod host_common;

use std::sync::Arc;
use std::sync::atomic::Ordering;

use fs_components_kanban::Answer;
use fs_forms_run::{
    Asked, OpenAsk, OpenFrontend, Placement, WhenOpen, WindowId, WindowOpened as Opened,
    WindowOwner,
};
use fs_notification::{ActionId, Notification};
use host_common::{PROGRAM, done, key, notes, req, world};

#[test]
fn h1_two_windows_of_two_frontends_show_one_record() {
    let w = world();
    let a = match w.owner.open(req("board", "urn:a")).unwrap() {
        Opened::New(id) => id,
        o => panic!("{o:?}"),
    };
    let b = match w.owner.open(req("board-compact", "urn:a")).unwrap() {
        Opened::New(id) => id,
        o => panic!("{o:?}"),
    };
    assert_eq!(w.counts.get(), (2, 0), "two windows, two holds");
    assert_eq!(w.owner.records_alive(), 1, "one cache for the one record");
    let (ga, gb) = (w.owner.grip_of(&a).unwrap(), w.owner.grip_of(&b).unwrap());
    assert!(Arc::ptr_eq(&ga.cache(), &gb.cache()));
    let before = gb.records().forms();
    assert_eq!(ga.records().move_card("FS-1", "done"), Answer::Accepted);
    assert_ne!(gb.records().forms(), before, "B shows what A moved");
    assert_eq!(
        (ga.generation(), gb.generation()),
        (1, 1),
        "each learned it through its own watch"
    );
    assert_eq!(
        w.wakes.0.load(Ordering::SeqCst),
        2,
        "both windows were woken, the writer's own too"
    );
    assert_eq!(w.shown.widgets.lock().unwrap().len(), 2);
}

#[test]
fn h2_the_answer_of_the_person_decides_and_only_for_its_own_question() {
    let w = world();
    w.owner.open(req("board", "urn:a")).unwrap();
    let want = req("board", "urn:b").with_when_open(WhenOpen::Ask);
    assert!(
        w.owner.open(want.clone()).is_err(),
        "the owner itself never asks"
    );
    let (n, toast) = notes();
    let (seen, d) = done();
    let Asked::Pending(q) = w.owner.open_asking(want.clone(), &n, 0, d).unwrap() else {
        panic!("a question")
    };
    assert_eq!(
        toast.0.lock().unwrap().as_slice(),
        [q.clone()],
        "the question was shown"
    );
    // another question with the same actions is answered: this one stays open and nothing opens
    let (n2, _t) = notes();
    let other = n2
        .ask(
            Notification::ask(
                fs_notification::Message::new("host-open-ask").unwrap(),
                vec![fs_notification::Action::new(
                    ActionId::new("window").unwrap(),
                    fs_notification::Message::new("host-open-ask-window").unwrap(),
                )],
            )
            .unwrap(),
            0,
        )
        .unwrap();
    n2.answered(&other, &ActionId::new("window").unwrap())
        .unwrap();
    assert!(seen.lock().unwrap().is_empty() && w.owner.open_now().len() == 1);
    n.answered(&q, &ActionId::new("window").unwrap()).unwrap();
    assert!(
        matches!(seen.lock().unwrap().as_slice(), [Ok(Opened::New(_))]),
        "window: a second window"
    );
    assert_eq!(w.owner.open_now().len(), 2);
    assert_eq!(w.counts.get(), (2, 0));
    // the same with `switch`: no new window, no new hold, the place shows the other record
    let w = world();
    let a = match w.owner.open(req("board", "urn:a")).unwrap() {
        Opened::New(id) => id,
        o => panic!("{o:?}"),
    };
    let (n, _) = notes();
    let (seen, d) = done();
    let Asked::Pending(q) = w.owner.open_asking(want, &n, 0, d).unwrap() else {
        panic!("a question")
    };
    n.answered(&q, &ActionId::new("switch").unwrap()).unwrap();
    let got = seen.lock().unwrap().remove(0).unwrap();
    assert!(
        matches!(&got, Opened::Switched { from: Some(f), .. } if f == "urn:a"),
        "{got:?}"
    );
    assert_eq!((w.owner.open_now().len(), w.counts.get()), (1, (1, 0)));
    assert!(
        w.owner.grip_of(&a).is_none()
            && w.owner
                .grip_of(&WindowId::of_record(PROGRAM, "board", Some("urn:b")))
                .is_some()
    );
    assert!(
        n.answered(&q, &ActionId::new("switch").unwrap()).is_err(),
        "a second answer finds nothing"
    );
}

#[test]
fn h3_closing_lets_everything_fall_and_so_does_the_person_closing_the_window() {
    let w = world();
    let a = match w.owner.open(req("board", "urn:a")).unwrap() {
        Opened::New(id) => id,
        o => panic!("{o:?}"),
    };
    let b = match w.owner.open(req("board-compact", "urn:a")).unwrap() {
        Opened::New(id) => id,
        o => panic!("{o:?}"),
    };
    w.owner.close(&a).unwrap();
    assert_eq!((w.owner.open_now().len(), w.counts.get()), (1, (2, 1)));
    assert_eq!(w.owner.records_alive(), 1, "B still shows the record");
    // the person closes B: the widget quits, Blinc drops the window and tells `on_closed`
    let widget = w.shown.widgets.lock().unwrap()[0].1.clone();
    widget.quit();
    drop(widget); // the test held the last clone
    assert!(w.owner.open_now().is_empty());
    assert_eq!((w.counts.get(), w.owner.records_alive()), ((2, 2), 0));
    assert!(w.owner.close(&b).is_err());
    // a window that cannot be shown leaves nothing behind
    *w.shown.broken.lock().unwrap() = true;
    assert!(w.owner.open(req("board", "urn:c")).is_err());
    assert_eq!((w.owner.open_now().len(), w.owner.records_alive()), (0, 0));
    assert_eq!(
        w.counts.get().0,
        w.counts.get().1,
        "every hold that was made fell"
    );
}

#[test]
fn h4_what_is_not_built_says_so_and_changes_nothing() {
    let w = world();
    let r = OpenFrontend::new(key("board"), Placement::InFrame).with_scope("urn:a");
    assert_eq!(
        w.owner.open(r).unwrap_err().code,
        "host-notice-placement-unsupported"
    );
    let a = match w.owner.open(req("board", "urn:a")).unwrap() {
        Opened::New(id) => id,
        o => panic!("{o:?}"),
    };
    assert_eq!(
        w.owner.use_frontend(&a, "board-compact").unwrap_err().code,
        "host-notice-use-frontend-not-built"
    );
    assert_eq!(
        w.owner.place(&a, Placement::InFrame).unwrap_err().code,
        "host-notice-placement-unsupported"
    );
    assert!(w.owner.place(&a, Placement::Window).is_ok());
    assert_eq!(
        w.owner
            .open(OpenFrontend::new(key("nope"), Placement::Window))
            .unwrap_err()
            .code,
        "host-notice-frontend-missing"
    );
    assert_eq!(w.owner.open_now().len(), 1);
}
