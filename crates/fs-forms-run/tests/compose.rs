//! The host's rules, without a window (Zielfall tasks-board: pages board, list, detail of `PLAN-forms-host.md` 3.1).
//!
//! Tests: H-K3 (a source that is missing locks the part with its code and text, the rest runs), H-K14 (Tab order = `seq`), "nothing
//! is dropped" (every lock of the plan is in the tree), the catalog decides what is built (H-K2: the host never picks a block),
//! the failures that end the host or are shown, the words.
//!
//! Mutants (each makes the named test fail): M1 a locked placement is left out -> `a_missing_source_locks_the_part_and_the_rest_runs`;
//! M2 a block the catalog does not build is drawn anyway -> `a_block_that_is_not_built_is_locked_with_its_code`; M3 order by creation ->
//! `tab_order_is_seq`; M4 a page lock is dropped -> `every_lock_of_the_plan_is_in_the_tree`; M5 an empty plan is `Ready` ->
//! `a_plan_without_pages_is_refused_and_said`; M6 `layout-missing` ends the process -> `the_connection_errors_end_and_the_layout_errors_show`;
//! M7 a code without a sentence is dropped -> `window::tests::a_code_without_a_sentence_is_shown_as_its_code`.

use fs_forms_run::{
    Blocks, Catalog, Coded, FixtureLink, HostError, HostState, LayoutLink, MenuItem, MenuState,
    Node, PageView, PlaceView, SocketLink, TEXTS, TestCatalog, View, compose, words,
};

fn place(name: &str, seq: u32, block: &str) -> PlaceView {
    PlaceView {
        title: None,
        place: name.into(),
        seq,
        block: block.into(),
        locks: Vec::new(),
        slot_locks: Vec::new(),
    }
}

fn page(name: &str, seq: u32, places: Vec<PlaceView>) -> PageView {
    PageView {
        page: name.into(),
        seq,
        frame: "workbench".into(),
        locks: Vec::new(),
        places,
    }
}

fn tasks_board() -> View {
    View {
        frontend: "board".into(),
        title: "Tasks".into(),
        pages: vec![
            page(
                "board",
                1,
                vec![
                    place("detail", 2, "board-detail"),
                    place("tabs", 0, "tablist"),
                    place("board", 1, "board"),
                ],
            ),
            page("list", 2, vec![place("table", 1, "table")]),
        ],
        menu: vec![
            MenuItem {
                id: "new".into(),
                label: "New task".into(),
                state: MenuState::Coming,
            },
            MenuItem {
                id: "purge".into(),
                label: "Purge".into(),
                state: MenuState::Hidden,
            },
        ],
        notices: Vec::new(),
    }
}

fn everything() -> TestCatalog {
    TestCatalog::building(&[
        "workbench",
        "tablist",
        "board",
        "board-detail",
        "table",
        "partial-notice",
    ])
}

fn nodes<'a>(c: &'a fs_forms_run::Composition, page: &str) -> &'a [Node] {
    &c.screens.iter().find(|s| s.page == page).unwrap().nodes
}

#[test]
fn a_ready_plan_draws_every_placement() {
    let c = compose(&tasks_board(), &everything());
    assert_eq!(c.state, HostState::Ready);
    assert_eq!(nodes(&c, "board").len(), 3);
    assert!(
        nodes(&c, "board")
            .iter()
            .all(|n| matches!(n, Node::Block { .. }))
    );
}

#[test]
fn tab_order_is_seq() {
    let c = compose(&tasks_board(), &everything());
    let order: Vec<_> = nodes(&c, "board")
        .iter()
        .map(|n| match n {
            Node::Block { place, .. } => place.clone(),
            _ => String::new(),
        })
        .collect();
    assert_eq!(
        order,
        ["tabs", "board", "detail"],
        "the plan lists detail first; the Tab order is `seq`"
    );
}

#[test]
fn a_missing_source_locks_the_part_and_the_rest_runs() {
    let mut v = tasks_board();
    v.pages[0]
        .places
        .iter_mut()
        .find(|p| p.place == "board")
        .unwrap()
        .locks = vec![Coded::new("bind-error-provider-unreachable").with("capability", "tasks")];
    let c = compose(&v, &everything());
    assert_eq!(c.state, HostState::Partial);
    let n = nodes(&c, "board");
    assert!(
        matches!(&n[1], Node::Locked { place: Some(p), codes } if p == "board" && codes[0].code == "bind-error-provider-unreachable")
    );
    assert!(
        matches!(&n[0], Node::Block { .. }),
        "the rest of the page is drawn"
    );
    assert!(
        matches!(&nodes(&c, "list")[0], Node::Block { .. }),
        "the other page runs"
    );
}

#[test]
fn a_block_that_is_not_built_is_locked_with_its_code() {
    let c = compose(
        &tasks_board(),
        &TestCatalog::building(&["workbench", "tablist"]).knowing(&["board"]),
    );
    assert!(nodes(&c, "board").iter().any(|n| matches!(n,
        Node::Locked { codes, .. } if codes[0].code == "host-error-block-not-built" && codes[0].params == [("block".into(), "board".into())])));
    assert_eq!(c.state, HostState::Partial);
}

#[test]
fn a_block_the_catalog_does_not_know_is_locked_as_unknown_not_as_not_built() {
    let c = compose(
        &tasks_board(),
        &TestCatalog::building(&["workbench", "tablist"]),
    );
    assert!(nodes(&c, "board").iter().any(|n| matches!(n,
        Node::Locked { codes, .. } if codes[0].code == "host-error-block-unknown" && codes[0].params == [("block".into(), "board".into())])));
    assert!(
        !nodes(&c, "board").iter().any(|n| matches!(n,
            Node::Locked { codes, .. } if codes[0].code == "host-error-block-not-built")),
        "an unknown block is not a block that is merely not built"
    );
}

#[test]
fn every_lock_of_the_plan_is_in_the_tree() {
    let mut v = tasks_board();
    v.pages[1].locks = vec![Coded::new("bind-error-page-refused")];
    v.notices = vec![Coded::new("bind-error-settings-defaults")];
    v.pages[0].places[0].slot_locks = vec![Coded::new("bind-error-field-component-missing")];
    let c = compose(&v, &everything());
    let mut codes: Vec<String> = c
        .screens
        .iter()
        .flat_map(|s| &s.nodes)
        .flat_map(|n| match n {
            Node::Locked { codes, .. } => codes.iter().map(|c| c.code.clone()).collect(),
            Node::Block { slot_locks, .. } => slot_locks.iter().map(|c| c.code.clone()).collect(),
            Node::Empty { .. } => Vec::new(),
        })
        .chain(c.notices.iter().map(|c| c.code.clone()))
        .collect();
    codes.sort();
    assert_eq!(
        codes,
        [
            "bind-error-field-component-missing",
            "bind-error-page-refused",
            "bind-error-settings-defaults"
        ]
    );
}

#[test]
fn a_plan_without_pages_is_refused_and_said() {
    let c = compose(&View::default(), &everything());
    assert_eq!(c.state, HostState::Refused);
    assert_eq!(c.notices[0].code, "host-error-no-pages");
}

#[test]
fn an_empty_page_says_so() {
    let mut v = tasks_board();
    v.pages.push(page("empty", 3, Vec::new()));
    assert!(matches!(
        nodes(&compose(&v, &everything()), "empty")[0],
        Node::Empty { .. }
    ));
}

#[test]
fn the_real_registry_draws_the_board_and_the_state_blocks_and_locks_the_rest() {
    assert!(Blocks.built("workbench") && Blocks.built("partial-notice") && Blocks.built("board"));
    assert!(Blocks.draws("board") && Blocks.draws("sidebar") && Blocks.draws("empty-state"));
    assert!(
        !Blocks.draws("workbench"),
        "a frame is a screen, no element"
    );
    assert!(
        !Blocks.built("board-detail") && !Blocks.draws("board-detail"),
        "no component is mapped yet"
    );
    assert!(
        Blocks.known("board-detail") && !Blocks.known("no-such-block"),
        "the contract of the catalog knows the block; the host's own list does not decide that"
    );
    let c = compose(&tasks_board(), &Blocks);
    assert_eq!(
        c.state,
        HostState::Partial,
        "the board is drawn, the rest is locked"
    );
    let nodes = &c.screens[0].nodes;
    assert!(
        nodes
            .iter()
            .any(|n| matches!(n, Node::Block { block, .. } if block == "board"))
    );
    assert!(
        nodes
            .iter()
            .filter(|n| matches!(n, Node::Locked { .. }))
            .count()
            >= 2,
        "board-detail and tablist stay locked, said"
    );
}

#[test]
fn the_connection_errors_end_and_the_layout_errors_show() {
    let gone = SocketLink::open(Some(std::path::Path::new("/nonexistent/forms.sock"))).unwrap_err();
    assert!(gone.fatal() && gone.code() == "layouts-unreachable");
    let missing = HostError::LayoutMissing {
        extends: "x".into(),
        frontend: "y".into(),
    };
    assert!(!missing.fatal());
    let store = HostError::LayoutStore {
        code: "layout-error-store-broken".into(),
    };
    assert!(!store.fatal());
    assert_eq!(
        FixtureLink(Err(missing.clone())).resolve("x", "y"),
        Err(missing)
    );
}

#[test]
fn the_socket_without_a_layouts_contract_says_so() {
    let dir = std::env::temp_dir();
    let link = SocketLink::open(Some(&dir)).unwrap(); // any existing path: the contract is what is missing
    let e = link.resolve("a", "b").unwrap_err();
    assert_eq!(e.code(), "layouts-contract-missing");
    assert!(e.fatal());
}

#[test]
fn both_languages_have_the_same_ids_and_a_code_has_a_sentence() {
    let unpaired = fs_components_mf2::check_pairs(TEXTS);
    assert_eq!(unpaired.len(), 0, "{unpaired:?}");
    let en = words(&fs_components_words::Locale::english()).plain("host-error-block-not-built");
    let de = words(&fs_components_words::Locale::parse("de").unwrap())
        .plain("host-error-block-not-built");
    assert!(!en.is_empty() && !de.is_empty() && en != de);
}
