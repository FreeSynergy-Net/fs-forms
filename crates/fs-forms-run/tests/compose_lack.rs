//! Karte (a), Ehrlichkeit: the strip "some parts are locked" is for a real lack only, and it names the parts.
//!
//! Pattern: none (tests; the rule under test is the Specification `lacking_of` of `compose.rs`).
//!
//! Mutants: M8 every `Locked`/`Empty` counts as a lack again (`_ => true`) -> `only_empty_parts_make_no_lack_strip`;
//! M9 the lacking list is left empty -> `a_real_lack_makes_the_strip_and_names_the_part`.

use fs_forms_run::{Coded, HostState, PageView, PlaceView, TestCatalog, View, compose};

fn place(name: &str, seq: u32, locks: Vec<Coded>) -> PlaceView {
    PlaceView {
        title: None,
        place: name.into(),
        seq,
        block: "board".into(),
        locks,
        slot_locks: Vec::new(),
        info: fs_forms_run::PlaceInfo::default(),
        elem: String::new(),
        embed: None,
    }
}

fn view(pages: Vec<PageView>) -> View {
    View {
        wants: Vec::new(),
        feeds: Vec::new(),
        pages,
        ..View::default()
    }
}

fn page(name: &str, seq: u32, locks: Vec<Coded>, places: Vec<PlaceView>) -> PageView {
    PageView {
        page: name.into(),
        seq,
        frame: "workbench".into(),
        locks,
        places,
    }
}

fn catalog() -> TestCatalog {
    TestCatalog::building(&["workbench", "board"])
}

#[test]
fn a_real_lack_makes_the_strip_and_names_the_part() {
    let unreachable = Coded::new("bind-error-provider-unreachable").with("capability", "tasks");
    let v = view(vec![page(
        "board",
        1,
        Vec::new(),
        vec![
            place("main", 0, Vec::new()),
            place("detail", 1, vec![unreachable]),
        ],
    )]);
    let c = compose(&v, &catalog());
    assert_eq!(c.state, HostState::Partial);
    assert_eq!(
        c.lacking,
        ["detail"],
        "the part that is not reachable is named"
    );
}

#[test]
fn only_empty_parts_make_no_lack_strip() {
    // the plan locks a page without placements with `page-empty`; the host composes an empty page: that lacks nothing
    let empty = Coded::new("bind-error-locked").with("cause", "bind-error-page-empty");
    let v = view(vec![
        page("board", 1, Vec::new(), vec![place("main", 0, Vec::new())]),
        page("empty", 2, vec![empty], Vec::new()),
    ]);
    let c = compose(&v, &catalog());
    assert!(
        c.lacking.is_empty(),
        "an empty page lacks nothing: {:?}",
        c.lacking
    );
    assert_eq!(c.state, HostState::Ready, "nothing lacks, so no strip");
}

#[test]
fn a_notice_alone_is_no_lack_and_no_locked_status() {
    let mut v = view(vec![page(
        "board",
        1,
        Vec::new(),
        vec![place("main", 0, Vec::new())],
    )]);
    v.notices.push(Coded::new("host-notice-style-unknown"));
    let c = compose(&v, &catalog());
    assert!(c.lacking.is_empty(), "{:?}", c.lacking);
    assert_eq!(c.state, HostState::Ready, "a notice is no lock");
    assert_eq!(c.notices.len(), 1, "the notice stays its own line");
}
