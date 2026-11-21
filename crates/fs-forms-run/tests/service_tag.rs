//! The tag the planning service program is built from in the tests equals the tag of the pin `fs-modules-planning-client`.
//!
//! Tests: example-based, reads the two source files. Mutant: one of the two is lifted alone and the test fails.

use std::fs;
use std::path::PathBuf;

fn read(rel: &str) -> String {
    fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)).unwrap()
}

/// The text between the quotes after `marker` on the line that contains it.
fn quoted_after(text: &str, marker: &str, key: &str) -> String {
    let line = text
        .lines()
        .find(|l| l.contains(marker))
        .unwrap_or_else(|| panic!("no line with {marker}"));
    let rest = line
        .split(key)
        .nth(1)
        .unwrap_or_else(|| panic!("no {key} in {line}"));
    rest.split('"').nth(1).expect("a quoted value").to_string()
}

#[test]
fn the_service_tag_of_the_tests_is_the_tag_of_the_planning_client_pin() {
    let pin = quoted_after(&read("Cargo.toml"), "fs-modules-planning-client =", "tag =");
    let service = quoted_after(&read("tests/support/service.rs"), "const SERVICE_TAG", "=");
    assert_eq!(
        service, pin,
        "lift both together (service.rs SERVICE_TAG and Cargo.toml)"
    );
}
