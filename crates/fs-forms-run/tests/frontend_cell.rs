//! `use_frontend` of the real owner `HostWindows` (stage 6, C1) and the data of the menu `View`.
//!
//! Tests: gc1h the surface stays (id, scope, the very same grip cache, holds, the window itself), only the key changes; gc2h an unknown
//! name is refused with `available` and nothing changes; gc3h after the change `open(board-compact, A)` is `Raised`, not `New`;
//! gc4h the menu data are the frontends the port lists and the open one is the radio that is on, a click on a line changes the
//! frontend (the same function), a refusal is said in the status line; no drawing code in the cell and the menus (grep).
//!
//! Mutants (run, red as named): M1 `use_frontend` forgets to set the key -> gc1h, gc3h; M2 it makes a new hold -> gc1h; M3 the menu
//! marks the first row as on -> gc4h; M4 the click is not wired -> gc4h.

mod host_common;

use std::sync::Arc;

use fs_components_app::Status;
use fs_forms_run::{WindowId, WindowOpened as Opened, WindowOwner};
use host_common::{key, req, world};

const A: &str = "urn:a";

fn opened(frontend: &str) -> (host_common::World, WindowId) {
    let host = world();
    let Opened::New(id) = host.owner.open(req(frontend, A)).unwrap() else {
        panic!("a new window")
    };
    (host, id)
}

#[test]
fn gc1h_the_surface_stays_only_the_frontend_changes() {
    let (host, id) = opened("board");
    let cache = host.owner.grip_of(&id).unwrap().cache();
    let (holds, windows) = (host.counts.get(), host.shown.widgets.lock().unwrap().len());
    host.owner.use_frontend(&id, "board-compact").unwrap();
    let now = host.owner.open_now();
    assert_eq!(now.len(), 1);
    assert_eq!((&now[0].id, now[0].scope.as_deref()), (&id, Some(A)));
    assert_eq!(now[0].key, key("board-compact"));
    assert!(Arc::ptr_eq(
        &cache,
        &host.owner.grip_of(&id).unwrap().cache()
    ));
    assert_eq!(host.counts.get(), holds, "no new hold, none dropped");
    assert_eq!(
        host.shown.widgets.lock().unwrap().len(),
        windows,
        "the window stays the one window"
    );
    assert_eq!(host.owner.records_alive(), 1);
    host.owner.use_frontend(&id, "board").unwrap();
    assert_eq!(host.owner.open_now()[0].key, key("board"));
    assert_eq!(host.counts.get(), holds);
}

#[test]
fn gc2h_an_unknown_name_is_refused_with_the_names_that_exist_and_nothing_changes() {
    let (host, id) = opened("board");
    let (now, holds) = (host.owner.open_now(), host.counts.get());
    let e = host.owner.use_frontend(&id, "nope").unwrap_err();
    assert_eq!(e.code, "host-notice-frontend-unknown");
    assert!(
        e.params
            .contains(&("available".into(), "[board, board-compact]".into())),
        "{e:?}"
    );
    let e = host
        .owner
        .use_frontend(&WindowId("a/b".into()), "board")
        .unwrap_err();
    assert_eq!(e.code, "host-notice-window-unknown");
    assert_eq!((host.owner.open_now(), host.counts.get()), (now, holds));
}

#[test]
fn gc3h_after_the_change_the_place_is_found_by_its_key() {
    let (host, id) = opened("board");
    host.owner.use_frontend(&id, "board-compact").unwrap();
    match host.owner.open(req("board-compact", A)).unwrap() {
        Opened::Raised(r) => assert_eq!(r, id, "the id stays the name of the place"),
        o => panic!("not raised: {o:?}"),
    }
    assert_eq!(host.owner.open_now().len(), 1);
}

#[test]
fn gc4h_the_menu_data_are_the_listed_frontends_and_a_click_changes_the_frontend() {
    let (host, id) = opened("board");
    let (rows, on) = host.owner.frontend_rows(&id).unwrap();
    let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(
        ids,
        ["board", "board-compact"],
        "what the port lists, in its order"
    );
    assert_eq!(on, "board");
    let window = host.shown.widgets.lock().unwrap()[0].1.clone();
    let radios = |w: &fs_components_app::AppWidget| {
        w.with_shell(|s| {
            let view = s
                .menus()
                .iter()
                .find(|m| {
                    m.entries
                        .iter()
                        .any(|e| e.id.starts_with("layout.frontend."))
                })
                .expect("menu View");
            view.entries
                .iter()
                .map(|e| (e.id.clone(), Some(e.checked)))
                .collect::<Vec<_>>()
        })
    };
    let before = radios(&window);
    assert_eq!(
        before,
        [
            ("layout.frontend.board".into(), Some(true)),
            ("layout.frontend.board-compact".into(), Some(false))
        ]
    );
    assert!(
        window.choose("layout.frontend.board-compact"),
        "the line is there and enabled"
    );
    assert_eq!(host.owner.open_now()[0].key, key("board-compact"));
    assert_eq!(host.owner.frontend_rows(&id).unwrap().1, "board-compact");
    assert_eq!(
        radios(&window),
        [
            ("layout.frontend.board".into(), Some(false)),
            ("layout.frontend.board-compact".into(), Some(true))
        ],
        "the radio follows"
    );
    assert!(
        host.wakes.0.load(std::sync::atomic::Ordering::SeqCst) >= 1,
        "the window was woken"
    );
}

#[test]
fn gc4h_a_refused_click_is_said_in_the_status_line_and_changes_nothing() {
    let (host, id) = opened("board");
    let Opened::New(other) = host.owner.open(req("board-compact", A)).unwrap() else {
        panic!("a second window")
    };
    let window = host.shown.widgets.lock().unwrap()[0].1.clone();
    assert!(window.choose("layout.frontend.board-compact"));
    let said = window.with_shell(|s| s.status().cloned());
    assert!(
        matches!(&said, Some(Status::Problem(c)) if c == "host-notice-frontend-open-elsewhere"),
        "{said:?}"
    );
    let keys: Vec<_> = host
        .owner
        .open_now()
        .into_iter()
        .map(|o| (o.id, o.key))
        .collect();
    assert!(keys.contains(&(id, key("board"))) && keys.contains(&(other, key("board-compact"))));
}

#[test]
fn gc4h_the_cell_and_the_menus_draw_nothing() {
    for file in [
        "src/frontend_cell.rs",
        "src/menus.rs",
        "src/window_owner/host/switch.rs",
    ] {
        let text =
            std::fs::read_to_string(format!("{}/{file}", env!("CARGO_MANIFEST_DIR"))).unwrap();
        let code: String = text
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect();
        for drawn in ["div()", "Div::", "text(", ".bg(", ".child("] {
            assert!(!code.contains(drawn), "{file} draws: {drawn}");
        }
    }
}
