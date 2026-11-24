//! `--frontend` is honoured also when `scaffold.toml` names a standard layout (ursache `URSACHE-frontend-wahl`).
//!
//! Pattern: example-based tests on real files (a blueprint folder with two frontends); the loaded FILE is checked, not only the view.
//! Mutants: the layout of `scaffold.toml` is returned before `asked_for` is asked (the old stand); an unknown id gives the
//! standard or an empty view; an empty `--frontend` no longer gives the standard.

use std::fs;
use std::path::PathBuf;

use fs_forms_run::{FileLink, HostError, LayoutLink};

const BOARD: &str = include_str!("fixtures/board.toml");

/// A program with the standard `board` (named by the blueprint) and a second frontend `board-wip`.
fn program(name: &str) -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("fwahl")
        .join(name);
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(d.join("forms")).unwrap();
    fs::write(
        d.join("scaffold.toml"),
        "[frontend]\nstandard = \"board\"\nlayout = \"forms/board.toml\"\n",
    )
    .unwrap();
    fs::write(d.join("forms/board.toml"), BOARD).unwrap();
    let wip = BOARD
        .replacen("id = \"board\"", "id = \"board-wip\"", 1)
        .replacen("\"Tasks\"", "\"Tasks WIP\"", 1);
    fs::write(d.join("forms/board-wip.toml"), wip).unwrap();
    d
}

#[test]
fn each_frontend_loads_its_own_file_though_the_blueprint_names_a_standard() {
    let d = program("two");
    let link = FileLink::new(&d, "en");
    let standard = link.load("", "board").unwrap();
    let wip = link.load("", "board-wip").unwrap();
    assert_eq!(standard.file, PathBuf::from("forms/board.toml"));
    assert_eq!(wip.file, PathBuf::from("forms/board-wip.toml"));
    assert_eq!(wip.view.frontend, "board-wip");
    assert_eq!(wip.view.title, "Tasks WIP");
    assert_ne!(standard.view.title, wip.view.title);
    assert_eq!(wip.line(), "frontend board-wip from forms/board-wip.toml");
    // the plain view agrees with the loaded one
    assert_eq!(link.resolve("", "board-wip").unwrap().title, "Tasks WIP");
}

#[test]
fn an_unknown_frontend_is_an_error_with_the_ids_that_exist() {
    let d = program("unknown");
    let e = FileLink::new(&d, "en").load("", "nope").unwrap_err();
    assert_eq!(e.code(), "frontend-unknown");
    assert!(e.fatal(), "exit 3: the starter shows it and falls back");
    assert_eq!(
        e.to_string(),
        "host-error-frontend-unknown id=nope available=[board, board-wip]"
    );
    assert!(matches!(e, HostError::FrontendUnknown { .. }));
    assert_eq!(FileLink::new(&d, "en").resolve("", "nope").unwrap_err(), e);
}

#[test]
fn an_empty_frontend_is_the_standard_of_the_blueprint() {
    let d = program("standard");
    let l = FileLink::new(&d, "en").load("", "").unwrap();
    assert_eq!(l.file, PathBuf::from("forms/board.toml"));
    assert_eq!(l.view.frontend, "board");
}
