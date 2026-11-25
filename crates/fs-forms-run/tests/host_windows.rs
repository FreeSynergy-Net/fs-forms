//! The real owner `HostWindows` (stage 6, O4a, `Window` only) over doubles of its three ports: two windows of two frontends of ONE
//! program show ONE record; each holds its own connection, learns of a change only through its own `watch`, and the question
//! "this place shows another record" goes through `Notifications::ask` (no dialog of its own).
//!
//! Tests: H1 two windows, one record: one cache, two holds, a move in A shows in B (and in A again, through its own `watch`) and both
//! windows are woken; H2 the answer of the person: `switch` keeps window and hold and shows the other record, `window` opens a second
//! one, an answer of another question or an unknown one opens nothing; H3 `close` lets grip and hold fall, a window the person closes
//! does the same (`on_closed`), a failure to show leaves nothing behind; H4 what is not built says so (`InFrame`; `use_frontend` is built since C1: `frontend_cell.rs`).
//!
//! Mutants (all four run, each red as named): M-wake the sink does not wake the window -> H1; M-open the answer `window` opens as
//! `switch` -> H2; M-sig the observer reacts to every answer, not only to its own question -> H2; M-close `close` does not drop the
//! entries -> H3.

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
    let host = world();
    let first = match host.owner.open(req("board", "urn:a")).unwrap() {
        Opened::New(id) => id,
        o => panic!("{o:?}"),
    };
    let second = match host.owner.open(req("board-compact", "urn:a")).unwrap() {
        Opened::New(id) => id,
        o => panic!("{o:?}"),
    };
    assert_eq!(host.counts.get(), (2, 0), "two windows, two holds");
    assert_eq!(
        host.owner.records_alive(),
        1,
        "one cache for the one record"
    );
    let (ga, gb) = (
        host.owner.grip_of(&first).unwrap(),
        host.owner.grip_of(&second).unwrap(),
    );
    assert!(Arc::ptr_eq(&ga.cache(), &gb.cache()));
    let before = gb.records().forms();
    assert_eq!(
        fs_forms_run::sample_move_card(&*ga.records(), "FS-1", "done"),
        Answer::Accepted
    );
    assert_ne!(gb.records().forms(), before, "B shows what A moved");
    assert_eq!(
        (ga.generation(), gb.generation()),
        (1, 1),
        "each learned it through its own watch"
    );
    assert_eq!(
        host.wakes.0.load(Ordering::SeqCst),
        2,
        "both windows were woken, the writer's own too"
    );
    assert_eq!(host.shown.widgets.lock().unwrap().len(), 2);
}

#[test]
fn h2_the_answer_of_the_person_decides_and_only_for_its_own_question() {
    let host = world();
    host.owner.open(req("board", "urn:a")).unwrap();
    let want = req("board", "urn:b").with_when_open(WhenOpen::Ask);
    assert!(
        host.owner.open(want.clone()).is_err(),
        "the owner itself never asks"
    );
    let (bell, toast) = notes();
    let (seen, d) = done();
    let Asked::Pending(ask_id) = host.owner.open_asking(want.clone(), &bell, 0, d).unwrap() else {
        panic!("a question")
    };
    assert_eq!(
        toast.0.lock().unwrap().as_slice(),
        std::slice::from_ref(&ask_id),
        "the question was shown"
    );
    // another question with the same actions is answered: this one stays open and nothing opens
    let other = bell
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
    bell.answered(&other, &ActionId::new("window").unwrap())
        .unwrap();
    assert!(seen.lock().unwrap().is_empty() && host.owner.open_now().len() == 1);
    bell.answered(&ask_id, &ActionId::new("window").unwrap())
        .unwrap();
    assert!(
        matches!(seen.lock().unwrap().as_slice(), [Ok(Opened::New(_))]),
        "window: a second window"
    );
    assert_eq!(host.owner.open_now().len(), 2);
    assert_eq!(host.counts.get(), (2, 0));
    // the same with `switch`: no new window, no new hold, the place shows the other record
    let host = world();
    let first = match host.owner.open(req("board", "urn:a")).unwrap() {
        Opened::New(id) => id,
        o => panic!("{o:?}"),
    };
    let (bell, _) = notes();
    let (seen, d) = done();
    let Asked::Pending(ask_id) = host.owner.open_asking(want, &bell, 0, d).unwrap() else {
        panic!("a question")
    };
    bell.answered(&ask_id, &ActionId::new("switch").unwrap())
        .unwrap();
    let got = seen.lock().unwrap().remove(0).unwrap();
    assert!(
        matches!(&got, Opened::Switched { from: Some(f), .. } if f == "urn:a"),
        "{got:?}"
    );
    assert_eq!(
        (host.owner.open_now().len(), host.counts.get()),
        (1, (1, 0))
    );
    assert!(
        host.owner.grip_of(&first).is_none()
            && host
                .owner
                .grip_of(&WindowId::of_record(PROGRAM, "board", Some("urn:b")))
                .is_some()
    );
    assert!(
        bell.answered(&ask_id, &ActionId::new("switch").unwrap())
            .is_err(),
        "a second answer finds nothing"
    );
}

#[test]
fn h3_closing_lets_everything_fall_and_so_does_the_person_closing_the_window() {
    let host = world();
    let first = match host.owner.open(req("board", "urn:a")).unwrap() {
        Opened::New(id) => id,
        o => panic!("{o:?}"),
    };
    let second = match host.owner.open(req("board-compact", "urn:a")).unwrap() {
        Opened::New(id) => id,
        o => panic!("{o:?}"),
    };
    host.owner.close(&first).unwrap();
    assert_eq!(
        (host.owner.open_now().len(), host.counts.get()),
        (1, (2, 1))
    );
    assert_eq!(host.owner.records_alive(), 1, "B still shows the record");
    // the person closes B: the widget quits, Blinc drops the window and tells `on_closed`
    let widget = host.shown.widgets.lock().unwrap()[0].1.clone();
    widget.quit();
    drop(widget); // the test held the last clone
    assert_eq!(host.owner.open_now(), []);
    assert_eq!((host.counts.get(), host.owner.records_alive()), ((2, 2), 0));
    assert!(host.owner.close(&second).is_err());
    // a window that cannot be shown leaves nothing behind
    *host.shown.broken.lock().unwrap() = true;
    assert!(host.owner.open(req("board", "urn:c")).is_err());
    assert_eq!(
        (host.owner.open_now().len(), host.owner.records_alive()),
        (0, 0)
    );
    assert_eq!(
        host.counts.get().0,
        host.counts.get().1,
        "every hold that was made fell"
    );
}

#[test]
fn h4_what_is_not_built_says_so_and_changes_nothing() {
    let host = world();
    let r = OpenFrontend::new(key("board"), Placement::InFrame).with_scope("urn:a");
    assert_eq!(
        host.owner.open(r).unwrap_err().code,
        "host-notice-placement-unsupported"
    );
    let first = match host.owner.open(req("board", "urn:a")).unwrap() {
        Opened::New(id) => id,
        o => panic!("{o:?}"),
    };
    assert_eq!(
        host.owner
            .place(&first, Placement::InFrame)
            .unwrap_err()
            .code,
        "host-notice-placement-unsupported"
    );
    assert!(host.owner.place(&first, Placement::Window).is_ok());
    assert_eq!(
        host.owner
            .open(OpenFrontend::new(key("nope"), Placement::Window))
            .unwrap_err()
            .code,
        "host-notice-frontend-missing"
    );
    assert_eq!(host.owner.open_now().len(), 1);
}
