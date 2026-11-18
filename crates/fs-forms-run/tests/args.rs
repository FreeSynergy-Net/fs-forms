//! The words the starter hands over; wrong words are a message for exit code 2, never a guess.
//!
//! Tests: example-based. Mutant: a missing `--extends` is accepted -> `frontend_and_extends_are_needed` red.

use std::path::PathBuf;

use fs_forms_run::{Command, parse};

fn words(text: &str) -> Vec<String> {
    text.split_whitespace().map(str::to_owned).collect()
}

#[test]
fn the_starters_words_are_read() {
    let Ok(Command::Window(c)) = parse(&words(
        "--frontend board --extends net.freesynergy.Tasks --forms-socket /x/forms.sock --socket /x/t.sock --resumed",
    )) else {
        panic!("not a window");
    };
    assert_eq!(
        (c.frontend.as_str(), c.extends.as_str()),
        ("board", "net.freesynergy.Tasks")
    );
    assert_eq!(c.forms_socket, Some(PathBuf::from("/x/forms.sock")));
    assert_eq!(c.socket, Some(PathBuf::from("/x/t.sock")));
    assert!(c.resumed);
}

#[test]
fn fresh_is_the_default_and_the_last_word_wins() {
    let Ok(Command::Window(c)) = parse(&words("--frontend a --extends b --resumed --fresh")) else {
        panic!();
    };
    assert!(!c.resumed);
}

#[test]
fn frontend_and_extends_are_needed() {
    assert!(
        parse(&words("--frontend a"))
            .unwrap_err()
            .contains("--extends")
    );
    assert!(parse(&[]).is_err());
}

#[test]
fn unknown_words_and_missing_values_are_errors() {
    assert!(
        parse(&words("--nope"))
            .unwrap_err()
            .contains("unknown option --nope")
    );
    assert!(parse(&words("--frontend a --extends b --forms-socket")).is_err());
    assert!(parse(&words("--frontend a --extends b --size 0x5 --picture p")).is_err());
}

#[test]
fn check_picture_version_and_help() {
    assert!(matches!(
        parse(&words("--frontend a --extends b --check")),
        Ok(Command::Check(_))
    ));
    assert_eq!(parse(&words("--version")), Ok(Command::Version));
    assert_eq!(parse(&words("-h")), Ok(Command::Help));
    let Ok(Command::Window(c)) = parse(&words(
        "--frontend a --extends b --picture p.png --size 800x600",
    )) else {
        panic!();
    };
    assert_eq!(c.picture, Some((PathBuf::from("p.png"), (800, 600))));
}
