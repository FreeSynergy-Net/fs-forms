//! S6-7 round 2 (the three MUST of the review and the cheap SHOULD): the id of a child is made of the id of its parent; a change of
//! the frontend drops the panels of the old frontend and builds those of the new one; `scope-from`, `shown = on-open` and a loop are
//! locked parts with their own codes (the reasons are data: `missing_of`).
//!
//! Pattern: Fake over the three ports of `HostWindows` (as `host_embed.rs`), the table of the open frontends read as data.

mod embed_common;

use embed_common::{SETTINGS, TASKS, open_frontend, world};
use fs_forms_run::{Placement, WindowId, WindowOpened, WindowOwner, session_locale, words};

fn new_id(r: Result<WindowOpened, fs_forms_run::Coded>) -> WindowId {
    match r {
        Ok(WindowOpened::New(id)) => id,
        other => panic!("a new frontend: {other:?}"),
    }
}

fn child_of(parent: &WindowId) -> WindowId {
    WindowId::of(parent.as_str(), "settings-embed")
}

#[test]
fn the_same_board_for_two_records_has_two_panels_that_fall_one_by_one() {
    let w = world("panel");
    let a = new_id(open_frontend(&w, "board-embed", Some("urn:a")));
    let b = new_id(open_frontend(&w, "board-embed", Some("urn:b")));
    assert_ne!(child_of(&a), child_of(&b));
    let ids: Vec<WindowId> = w.owner.open_now().into_iter().map(|o| o.id).collect();
    for id in [&a, &b, &child_of(&a), &child_of(&b)] {
        assert!(ids.contains(id), "{id:?} is open: {ids:?}");
    }
    assert_eq!(ids.len(), 4);
    assert_eq!(
        w.settings.get(),
        (2, 0),
        "each panel holds its own connection"
    );
    w.owner.close(&a).unwrap();
    assert_eq!(
        w.settings.get(),
        (2, 1),
        "only the panel of the first board fell"
    );
    assert_eq!(w.owner.open_now().len(), 2);
    assert!(w.owner.panels_of(&b).is_some_and(|p| p.len() == 1));
}

#[test]
fn in_window_mode_the_second_record_opens_its_own_child_window_never_raises_the_first() {
    let w = world("window");
    let a = new_id(open_frontend(&w, "board-embed", Some("urn:a")));
    let b = new_id(open_frontend(&w, "board-embed", Some("urn:b")));
    let now = w.owner.open_now();
    assert_eq!(now.len(), 4, "two boards and two settings windows: {now:?}");
    let parent_of = |id: &WindowId| {
        now.iter()
            .find(|o| &o.id == id)
            .and_then(|o| o.parent.clone())
    };
    assert_eq!(parent_of(&child_of(&a)), Some(a.clone()));
    assert_eq!(parent_of(&child_of(&b)), Some(b));
    w.owner.close(&a).unwrap();
    assert_eq!(
        w.owner.open_now().len(),
        2,
        "the child of the second board stays"
    );
}

#[test]
fn a_switch_of_the_record_keeps_the_child_ids_with_their_parent() {
    let w = world("panel");
    let a = new_id(open_frontend(&w, "board-embed", Some("urn:a")));
    let key = fs_forms_layout::FrontendKey {
        extends: TASKS.into(),
        frontend: "board-embed".into(),
    };
    let req = fs_forms_run::OpenFrontend::new(key, Placement::Window)
        .with_when_open(fs_forms_run::WhenOpen::Switch)
        .with_scope("urn:c");
    let Ok(WindowOpened::Switched { id, .. }) = w.owner.open(req) else {
        panic!("a switch");
    };
    assert_ne!(id, a);
    let now = w.owner.open_now();
    assert_eq!(now.len(), 2);
    assert!(
        now.iter()
            .any(|o| o.id == child_of(&id) && o.parent.as_ref() == Some(&id))
    );
}

#[test]
fn use_frontend_drops_the_old_panels_and_builds_those_of_the_new_frontend() {
    let w = world("panel");
    let id = new_id(open_frontend(&w, "board-plain", None));
    assert_eq!(w.owner.panels_of(&id).unwrap(), []);
    assert_eq!(w.settings.get(), (0, 0));
    w.owner.use_frontend(&id, "board-embed").unwrap();
    let heads = w.owner.panels_of(&id).unwrap();
    assert_eq!(heads.len(), 1, "the new frontend starts a panel: {heads:?}");
    assert!(heads[0].1.contains(SETTINGS));
    assert_eq!(w.settings.get(), (1, 0));
    assert_eq!(w.owner.open_now().len(), 2);
    // and back: the old child falls with its hold
    w.owner.use_frontend(&id, "board-plain").unwrap();
    assert_eq!(w.settings.get(), (1, 1), "the grip of the old child fell");
    assert_eq!(w.owner.panels_of(&id).unwrap(), []);
    assert_eq!(w.owner.open_now().len(), 1);
    // and again: a fresh child (the id was free)
    w.owner.use_frontend(&id, "board-embed").unwrap();
    assert_eq!(w.settings.get(), (2, 1));
    assert_eq!(w.owner.panels_of(&id).unwrap().len(), 1);
    w.owner.close(&id).unwrap();
    assert_eq!(w.settings.get(), (2, 2));
}

fn only_missing(frontend: &str) -> Vec<(String, String)> {
    let w = world("panel");
    let id = new_id(open_frontend(&w, frontend, None));
    assert_eq!(
        w.owner.open_now().len(),
        1,
        "no child was built for {frontend}"
    );
    assert_eq!(w.settings.get(), (0, 0));
    w.owner.missing_of(&id).unwrap()
}

#[test]
fn a_node_with_scope_from_is_a_locked_part_not_a_child_without_a_record() {
    assert_eq!(
        only_missing("board-scope-from"),
        [(
            "detail".to_owned(),
            "host-notice-scope-from-not-built".to_owned()
        )]
    );
}

#[test]
fn a_node_that_opens_on_open_says_so() {
    assert_eq!(
        only_missing("board-on-open"),
        [(
            "detail".to_owned(),
            "host-notice-frontend-on-open-not-built".to_owned()
        )]
    );
}

#[test]
fn a_frontend_that_contains_itself_is_refused_with_its_own_code() {
    let loop_code = |w: &embed_common::World, id: &WindowId| w.owner.missing_of(id).unwrap();
    // A in A
    let w = world("panel");
    let a = new_id(open_frontend(&w, "board-loop", None));
    assert_eq!(w.owner.open_now().len(), 1);
    assert_eq!(
        loop_code(&w, &a),
        [("detail".to_owned(), "host-notice-frontend-loop".to_owned())]
    );
    // A in B in A: the second A is refused, the first B is built
    let w = world("panel");
    let a = new_id(open_frontend(&w, "board-a", None));
    assert_eq!(w.owner.open_now().len(), 2, "B is inside A");
    let b = child_of(&a);
    assert_eq!(
        loop_code(&w, &b),
        [("detail".to_owned(), "host-notice-frontend-loop".to_owned())]
    );
    // five levels: the fifth is too deep
    let w = world("panel");
    let c1 = new_id(open_frontend(&w, "chain-1", None));
    assert_eq!(w.owner.open_now().len(), 4);
    let c4 = child_of(&child_of(&child_of(&c1)));
    assert_eq!(
        loop_code(&w, &c4),
        [("detail".to_owned(), "host-notice-frontend-loop".to_owned())]
    );
}

#[test]
fn the_new_codes_have_a_sentence_in_both_languages() {
    assert_eq!(fs_components_mf2::check_pairs(fs_forms_run::TEXTS).len(), 0);
    for code in [
        "host-notice-scope-from-not-built",
        "host-notice-frontend-on-open-not-built",
        "host-notice-frontend-loop",
    ] {
        let sentence = words(&session_locale()).plain(code);
        assert_ne!(sentence, code, "{code} has a sentence");
    }
}
