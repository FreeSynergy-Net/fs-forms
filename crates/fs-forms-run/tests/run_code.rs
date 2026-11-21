//! `run_code` gives the REAL exit number (`ExitCode` has no accessor); `run` keeps its behaviour on top of it.
//!
//! Tests: example-based, only paths that end before a window opens. Mutant: `run_code` returns 0/1 only -> `usage_error_is_2` red.

use std::process::ExitCode;

use fs_forms_run::{EXIT_CONTRACT, run, run_code};

fn words(text: &str) -> Vec<String> {
    text.split_whitespace().map(str::to_owned).collect()
}

fn same(text: &str) -> i32 {
    let args = words(text);
    let code = run_code(&args);
    assert_eq!(
        format!("{:?}", run(&args)),
        format!("{:?}", ExitCode::from(u8::try_from(code).unwrap())),
        "run and run_code differ for `{text}`"
    );
    code
}

#[test]
fn usage_error_is_2() {
    assert_eq!(same("--frontend a"), 2);
    assert_eq!(same("--nonsense"), 2);
}

#[test]
fn help_and_version_are_0() {
    assert_eq!(same("--help"), 0);
    assert_eq!(same("--version"), 0);
}

#[test]
fn a_missing_description_is_1() {
    assert_eq!(
        same("--check --description /nonexistent/fs-forms-run-none"),
        1
    );
}

#[test]
fn a_missing_socket_is_the_contract_code() {
    let code = same("--check --frontend a --extends b --forms-socket /nonexistent/forms.sock");
    assert_eq!(code, i32::from(EXIT_CONTRACT));
}
