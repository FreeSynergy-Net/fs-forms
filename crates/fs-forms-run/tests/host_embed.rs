//! S6-7: the frontend node `settings-embed` of the board of `net.freesynergy.Tasks` places the frontend `settings` of ANOTHER program
//! (`net.freesynergy.Settings`, a fake module of its own with the groups data standing in for settings data). The same description is
//! read twice: `mode = panel` (drawn inside the page of the board) and `mode = window` (a window of its own, through the surface).
//!
//! Pattern: Fake over the three ports of `HostWindows`; two modules, each counting its OWN held connections.
//! Pictures: with `FS_PICTURE_OUT=<dir>` the pictures `stufe6-panel.png`, `stufe6-fenster.png` (the board, window mode) and
//! `stufe6-fenster-settings.png` (the settings window) are kept there; without a graphics adapter the picture test is skipped.

mod embed_common;

use embed_common::{SETTINGS, TASKS, child_id, groups, open_board, world};
use fs_forms_run::{Placement, WindowId, WindowOwner};

#[test]
fn a_panel_is_a_frontend_of_its_own_module_inside_the_page_of_its_parent() {
    let w = world("panel");
    let parent = open_board(&w);
    let now = w.owner.open_now();
    let modes: Vec<_> = now
        .iter()
        .map(|o| (o.id.clone(), o.mode, o.key.extends.clone()))
        .collect();
    assert_eq!(
        modes,
        [
            (parent.clone(), Placement::Window, TASKS.to_owned()),
            (child_id(&parent), Placement::InFrame, SETTINGS.to_owned()),
        ]
    );
    assert_eq!(
        now[1].parent.as_ref(),
        Some(&parent),
        "the panel hangs on its parent"
    );
    assert_eq!(w.shown.0.lock().unwrap().len(), 1, "a panel has no window");
    // each module saw exactly its own frontend: one connection each, never shared
    assert_eq!((w.tasks.get(), w.settings.get()), ((1, 0), (1, 0)));
    // the child speaks only to the foreign module: ITS records are the groups, the parent's are the board
    let names = |id: &WindowId| -> Vec<String> {
        let records = w.owner.grip_of(id).unwrap().records();
        records
            .forms()
            .iter()
            .map(|f| f.declared.form.name.clone())
            .collect()
    };
    let groups: Vec<String> = groups()
        .iter()
        .map(|f| f.declared.form.name.clone())
        .collect();
    let (child, board) = (names(&child_id(&parent)), names(&parent));
    assert_eq!(
        child, groups,
        "the panel shows the forms of the foreign module"
    );
    assert!(
        board.contains(&"cards-in-columns".to_owned())
            && !child.contains(&"cards-in-columns".to_owned()),
        "the board form is the parent's alone: board {board:?}, panel {child:?}"
    );
    // the head of the panel names the program it comes from
    let heads = w.owner.panels_of(&parent).unwrap();
    assert_eq!(heads.len(), 1);
    assert_eq!(heads[0].0, "detail");
    assert!(
        heads[0].1.contains(SETTINGS),
        "the head names the origin: {}",
        heads[0].1
    );
}

#[test]
fn closing_the_parent_signs_the_panel_off_at_its_module() {
    let w = world("panel");
    let parent = open_board(&w);
    assert_eq!(w.settings.get(), (1, 0));
    w.owner.close(&parent).unwrap();
    assert_eq!(w.owner.open_now(), []);
    assert_eq!(
        (w.tasks.get(), w.settings.get()),
        ((1, 1), (1, 1)),
        "the handle fell"
    );
}

#[test]
fn the_same_description_with_mode_window_opens_a_window_of_its_own() {
    let w = world("window");
    let parent = open_board(&w);
    let now = w.owner.open_now();
    assert_eq!(now.len(), 2);
    assert!(now.iter().all(|o| o.mode == Placement::Window));
    assert_eq!(w.shown.0.lock().unwrap().len(), 2, "two windows");
    assert!(
        w.owner.panels_of(&parent).unwrap().is_empty(),
        "nothing is drawn inside the board"
    );
    assert_eq!((w.tasks.get(), w.settings.get()), ((1, 0), (1, 0)));
    w.owner.close(&parent).unwrap();
    assert_eq!(
        w.owner.open_now(),
        [],
        "the window of the child goes with its parent"
    );
    assert_eq!(w.settings.get(), (1, 1));
}
