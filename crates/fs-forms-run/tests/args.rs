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

#[test]
fn the_style_set_for_the_program_is_a_style_name_and_a_wrong_one_is_a_message() {
    let Ok(Command::Window(c)) = parse(&words("--description x --style apple")) else {
        panic!("not a window");
    };
    assert_eq!(
        c.style.map(fs_components_styles::StyleId::name),
        Some("apple")
    );
    let Ok(Command::Window(none)) = parse(&words("--description x")) else {
        panic!("not a window");
    };
    assert_eq!(
        none.style, None,
        "no word: the preset (admin) is the host's, not the parser's"
    );
    let Err(message) = parse(&words("--description x --style neon")) else {
        panic!("a wrong style is accepted");
    };
    assert!(
        message.contains("neon") && message.contains("admin") && message.contains("craft"),
        "{message}"
    );
    assert!(
        parse(&words("--description x --style")).is_err(),
        "a missing value"
    );
}

#[test]
fn scope_is_one_record_per_start_and_never_empty() {
    let Ok(Command::Window(c)) = parse(&words("--frontend a --extends b --scope urn:uuid:7"))
    else {
        panic!("not a window");
    };
    assert_eq!(c.scope.as_deref(), Some("urn:uuid:7"));
    let Ok(Command::Window(c)) = parse(&words("--frontend a --extends b")) else {
        panic!();
    };
    assert_eq!(c.scope, None, "no --scope: no record");
    assert!(
        parse(&words("--frontend a --extends b --scope"))
            .unwrap_err()
            .contains("needs a value")
    );
    let empty = ["--frontend", "a", "--extends", "b", "--scope", " "].map(str::to_owned);
    assert!(parse(&empty).unwrap_err().contains("empty"));
    assert!(
        parse(&words("--frontend a --extends b --scope x --scope y"))
            .unwrap_err()
            .contains("twice")
    );
    assert!(fs_forms_run::HELP.contains("--scope AS2-ID"));
}

#[test]
fn sample_now_comes_with_sample_data_and_is_read_as_rfc_3339() {
    let Ok(Command::Window(c)) = parse(&words(
        "--description x.toml --sample-data --sample-now 2026-11-25T10:00:00+01:00",
    )) else {
        panic!("not a window");
    };
    let want: fs_forms_run::Moment = "2026-11-25T09:00:00Z".parse().expect("rfc3339");
    assert_eq!(
        c.sample_now,
        Some(want),
        "an offset is an instant like any other"
    );
    let Ok(Command::Window(c)) = parse(&words("--description x.toml --sample-data")) else {
        panic!("not a window");
    };
    assert_eq!(
        c.sample_now, None,
        "the data say their own now unless the switch does"
    );
}

#[test]
fn sample_now_without_sample_data_is_an_error_never_a_default() {
    let Err(message) = parse(&words(
        "--description x.toml --sample-now 2026-11-25T09:00:00Z",
    )) else {
        panic!("accepted without --sample-data");
    };
    assert!(
        message.contains("--sample-now needs --sample-data"),
        "{message}"
    );
}

#[test]
fn a_sample_now_that_is_no_rfc_3339_is_an_error() {
    for bad in [
        "tomorrow",
        "2026-11-25",
        "2026-11-25 09:00:00",
        "25.11.2026",
    ] {
        let all = vec![
            "--description".to_owned(),
            "x.toml".to_owned(),
            "--sample-data".to_owned(),
            "--sample-now".to_owned(),
            bad.to_owned(),
        ];
        let Err(message) = parse(&all) else {
            panic!("{bad} was accepted");
        };
        assert!(
            message.contains("--sample-now wants an RFC 3339"),
            "{message}"
        );
    }
    assert!(parse(&words("--description x.toml --sample-data --sample-now")).is_err());
}
