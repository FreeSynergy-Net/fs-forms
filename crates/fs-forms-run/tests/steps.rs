//! `--steps FILE` on the binary: the words and the refusals (exit 2, a message that names the cause, never a picture after a
//! refused input). The refusals of the file come before the window is built, so they need no graphics adapter; the move of a card
//! is tested in `src/steps/tests.rs` on the fake module.
//!
//! Mutants (by hand, red): the picture written before the plan is read; `--steps` without `--picture` accepted; the exit code of a
//! refused file 1; the help without `--steps`.
//!
//! Pattern: none (tests).

mod support;
use std::process::{Command, Output};

use fs_forms_run::{HELP, parse};
use support::{board, out_dir};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fs-forms-run"))
        .args(args)
        .env("LANG", "en_US.UTF-8")
        .output()
        .unwrap()
}

fn words(text: &[&str]) -> Vec<String> {
    text.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn steps_belong_to_a_picture_and_the_help_says_so() {
    let alone = parse(&words(&["--description", "x", "--steps", "a.json"])).unwrap_err();
    assert!(alone.contains("--steps needs --picture"), "{alone}");
    assert!(parse(&words(&["--description", "x", "--steps"])).is_err());
    assert!(
        parse(&words(&[
            "--description",
            "x",
            "--picture",
            "p.png",
            "--steps",
            "a.json"
        ]))
        .is_ok()
    );
    assert!(HELP.contains("--steps FILE"));
    let out = run(&["--help"]);
    assert!(String::from_utf8_lossy(&out.stdout).contains("--steps FILE"));
}

#[test]
fn a_file_that_cannot_be_played_is_exit_2_with_the_cause_and_no_picture() {
    let (_keep, dir) = out_dir();
    let board = board();
    let touch = dir.join("touch.json");
    std::fs::write(
        &touch,
        r#"{"actions":[{"type":"pointer","id":"p","parameters":{"pointerType":"touch"},"actions":[]}]}"#,
    )
    .unwrap();
    let broken = dir.join("broken.json");
    std::fs::write(&broken, "{").unwrap();
    let cases = [
        (dir.join("not-there.json"), "cannot read"),
        (broken, "steps-invalid"),
        (touch, "steps-unsupported"),
    ];
    for (steps, says) in cases {
        let png = dir.join("refused.png");
        let _ = std::fs::remove_file(&png);
        let out = run(&[
            "--description",
            &board,
            "--steps",
            steps.to_str().unwrap(),
            "--picture",
            png.to_str().unwrap(),
        ]);
        let err = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(2), "{err}");
        assert!(err.contains(says) && err.contains("--steps"), "{err}");
        assert!(!png.exists(), "no picture after a refused input");
    }
}
