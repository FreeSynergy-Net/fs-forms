//! H4: the host reads a description from disk (blueprint folder, folder of descriptions, one file) and says what is wrong.
//!
//! Tests: example-based on real files in a temp folder under the target (never a user folder). `fixtures/board.toml` is a copy
//! of the real description of the frontend `board` of fs-tasks (Format 1); the host knows no program name.
//! Mutants: a missing description gives an empty view instead of an error; `..` in the layout path is followed; the first file
//! is taken whatever `--frontend` says; a broken file is read as empty.

use std::fs;
use std::path::PathBuf;

use fs_forms_run::{Blocks, FileLink, HostError, LayoutLink, Node, compose};

const BOARD: &str = include_str!("fixtures/board.toml");

fn dir(name: &str) -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("k5-file")
        .join(name);
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

fn resolve(root: &PathBuf, frontend: &str) -> Result<fs_forms_run::View, HostError> {
    FileLink::new(root, "en").resolve("", frontend)
}

#[test]
fn a_blueprint_folder_names_its_description() {
    let d = dir("blueprint");
    fs::create_dir_all(d.join("forms")).unwrap();
    fs::write(
        d.join("scaffold.toml"),
        "[frontend]\nstandard = \"board\"\nlayout = \"forms/board.toml\"\n",
    )
    .unwrap();
    fs::write(d.join("forms/board.toml"), BOARD).unwrap();
    let v = resolve(&d, "").unwrap();
    assert_eq!(v.frontend, "board");
    assert_eq!(v.title, "Tasks");
    assert_eq!(v.pages.len(), 1);
    let places: Vec<_> = v.pages[0]
        .places
        .iter()
        .map(|p| (p.place.as_str(), p.block.as_str()))
        .collect();
    assert_eq!(
        places,
        [
            ("side", "sidebar"),
            ("main", "board"),
            ("detail", "board-detail"),
            ("partial", "partial-notice"),
            ("empty", "empty-state")
        ]
    );
    // K3: the actions of the description are what unlocks a gesture of the board
    for (trigger, command) in [
        ("quick-add", "create"),
        ("drop", "move"),
        ("button", "column.create"),
        ("menu", "delete"),
    ] {
        assert!(
            v.actions
                .iter()
                .any(|a| a.trigger == trigger && a.command == command),
            "{trigger} {command}: {:?}",
            v.actions
        );
    }
    assert!(
        v.menu.iter().any(|m| m.id == "claim"),
        "commands of the menu are read: {:?}",
        v.menu
    );
    let claim = v.menu.iter().find(|m| m.id == "claim").unwrap();
    assert_eq!(
        claim.state,
        fs_forms_run::MenuState::Coming,
        "no provider is connected: a command is there but does not run"
    );
    assert!(
        !v.notices.is_empty(),
        "no provider is connected and the window says so"
    );
}

#[test]
fn the_reader_language_picks_the_words_and_english_is_the_fallback() {
    let d = dir("lang");
    fs::write(d.join("a.toml"), BOARD).unwrap();
    let de = FileLink::new(d.join("a.toml"), "de")
        .resolve("", "")
        .unwrap();
    assert_eq!(de.title, "Aufgaben");
    let fr = FileLink::new(d.join("a.toml"), "fr")
        .resolve("", "")
        .unwrap();
    assert_eq!(fr.title, "Tasks");
}

#[test]
fn a_missing_description_is_an_error_never_an_empty_view() {
    let d = dir("none");
    let gone = d.join("not-there");
    let e = resolve(&gone, "").unwrap_err();
    assert_eq!(e.code(), "layout-missing");
    assert!(
        e.params()
            .iter()
            .any(|(k, v)| *k == "path" && v.ends_with("not-there"))
    );
    // a folder with a blueprint that names a file that does not exist
    fs::write(
        d.join("scaffold.toml"),
        "[frontend]\nlayout = \"forms/x.toml\"\n",
    )
    .unwrap();
    assert_eq!(resolve(&d, "").unwrap_err().code(), "layout-missing");
    // an empty folder
    let empty = dir("empty");
    assert_eq!(resolve(&empty, "").unwrap_err().code(), "layout-missing");
    assert!(!e.fatal(), "it is shown in the window, not ended");
}

#[test]
fn a_broken_file_names_every_problem() {
    let d = dir("broken");
    fs::write(d.join("b.toml"), "schema = 1\nthis is not toml").unwrap();
    match resolve(&d.join("b.toml"), "").unwrap_err() {
        HostError::LayoutUnreadable { file, problems } => {
            assert!(file.ends_with("b.toml"));
            assert!(
                problems
                    .iter()
                    .any(|p| p.contains("layout-error-import-invalid") && p.contains("cause=toml")),
                "{problems:?}"
            );
        }
        other => panic!("wrong error {other:?}"),
    }
    fs::write(
        d.join("c.toml"),
        BOARD.replace("kind = \"frontend\"", "kind = \"frontend\"\nsurprise = 1"),
    )
    .unwrap();
    assert_eq!(
        resolve(&d.join("c.toml"), "").unwrap_err().code(),
        "layout-unreadable"
    );
}

#[test]
fn the_blueprint_may_not_point_outside_its_folder() {
    let d = dir("escape");
    fs::write(d.join("outside.toml"), BOARD).unwrap();
    let inner = d.join("bp");
    fs::create_dir_all(&inner).unwrap();
    fs::write(
        inner.join("scaffold.toml"),
        "[frontend]\nlayout = \"../outside.toml\"\n",
    )
    .unwrap();
    assert_eq!(resolve(&inner, "").unwrap_err().code(), "layout-unreadable");
    fs::write(
        inner.join("scaffold.toml"),
        "[frontend]\nlayout = \"/etc/passwd\"\n",
    )
    .unwrap();
    assert_eq!(resolve(&inner, "").unwrap_err().code(), "layout-unreadable");
}

#[test]
fn several_descriptions_need_the_frontend_to_choose() {
    let d = dir("several");
    fs::create_dir_all(d.join("forms")).unwrap();
    fs::write(d.join("forms/board.toml"), BOARD).unwrap();
    fs::write(
        d.join("forms/other.toml"),
        BOARD.replace("id = \"board\"", "id = \"other\""),
    )
    .unwrap();
    assert_eq!(resolve(&d, "other").unwrap().frontend, "other");
    assert_eq!(resolve(&d, "board").unwrap().frontend, "board");
    assert_eq!(
        resolve(&d, "").unwrap_err().code(),
        "layout-missing",
        "two files and no choice is not a guess"
    );
    assert_eq!(resolve(&d, "nope").unwrap_err().code(), "layout-missing");
}

#[test]
fn what_the_host_cannot_draw_is_locked_visibly() {
    let d = dir("draw");
    fs::write(d.join("a.toml"), BOARD).unwrap();
    let v = resolve(&d.join("a.toml"), "").unwrap();
    let c = compose(&v, &Blocks);
    let nodes = &c.screens[0].nodes;
    let locked: Vec<&Node> = nodes
        .iter()
        .filter(|n| matches!(n, Node::Locked { .. }))
        .collect();
    // `board` and `sidebar` are drawn (components of fs-components); only `board-detail` has no component mapped yet
    assert_eq!(locked.len(), 1, "{nodes:?}");
    assert!(matches!(locked[0], Node::Locked { place: Some(p), .. } if p == "detail"));
    for n in &locked {
        if let Node::Locked { codes, .. } = n {
            assert_eq!(codes[0].code, "host-error-block-not-built");
        }
    }
}

#[test]
fn the_blueprint_decides_which_file_when_there_are_several() {
    let d = dir("scaffold-decides");
    fs::create_dir_all(d.join("forms")).unwrap();
    fs::write(d.join("forms/board.toml"), BOARD).unwrap();
    fs::write(
        d.join("forms/other.toml"),
        BOARD.replace("id = \"board\"", "id = \"other\""),
    )
    .unwrap();
    fs::write(
        d.join("scaffold.toml"),
        "[frontend]\nlayout = \"forms/other.toml\"\n",
    )
    .unwrap();
    assert_eq!(resolve(&d, "").unwrap().frontend, "other");
    // a scaffold without a layout line falls back to the folder, which is then ambiguous
    fs::write(d.join("scaffold.toml"), "[program]\nname = \"x\"\n").unwrap();
    assert_eq!(resolve(&d, "").unwrap_err().code(), "layout-missing");
    // a scaffold that is not TOML is said, not skipped
    fs::write(d.join("scaffold.toml"), "[frontend\n").unwrap();
    assert_eq!(resolve(&d, "").unwrap_err().code(), "layout-unreadable");
}

#[test]
fn the_style_of_the_description_is_kept_and_a_name_that_is_no_style_is_said() {
    let write = |name: &str, line: &str| {
        let d = dir(name);
        let f = d.join("board.toml");
        fs::write(&f, BOARD.replace("style = \"default\"", line)).unwrap();
        resolve(&f, "").unwrap()
    };
    let said = |v: &fs_forms_run::View| {
        v.notices
            .iter()
            .any(|n| n.code == "host-notice-style-unknown")
    };
    let plain = write("style-default", "style = \"default\"");
    assert_eq!(plain.style.as_deref(), Some("default"));
    assert!(!said(&plain), "`default` is no unknown style");
    let named = write("style-apple", "style = \"apple\"");
    assert_eq!(named.style.as_deref(), Some("apple"));
    assert!(!said(&named));
    let wrong = write("style-neon", "style = \"neon\"");
    assert!(said(&wrong), "a style this version does not have is said");
    assert!(
        wrong
            .notices
            .iter()
            .any(|n| n.params == [("style".to_owned(), "neon".to_owned())]),
        "the notice names the style"
    );
}
