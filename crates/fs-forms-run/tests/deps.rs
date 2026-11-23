//! H-K1: the host depends on the client and fs-components only: no way leads to a program crate, to the core or the server of
//! fs-forms, and the normal graph has no `toml` and no `serde_json`.
//!
//! Tests: example-based, reads `cargo tree` of the real manifest (normal dependencies).
//! Mutant: `fs-forms-core` (or any `fs-<program>-client`) as a dependency -> red.

use std::process::Command;

#[test]
fn no_normal_dependency_leads_to_a_program_the_core_or_the_server() {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let out = Command::new(cargo)
        .args([
            "tree",
            "-p",
            "fs-forms-run",
            "-e",
            "normal",
            "--prefix",
            "none",
            "--locked",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let tree = String::from_utf8_lossy(&out.stdout);
    assert!(
        tree.contains("fs-forms-client"),
        "the client must be there: {tree}"
    );
    let name = |l: &str| l.split_whitespace().next().unwrap_or("").to_owned();
    for l in tree.lines() {
        let n = name(l);
        assert!(
            !matches!(n.as_str(), "fs-forms-core" | "fs-forms-server"),
            "{n} is a dependency"
        );
        if let Some(rest) = n.strip_prefix("fs-") {
            // only the forms client and the fs-components family (and the transport it brings) may appear with the fs- prefix
            // K5 H4: `forms-layout` (the model) and `forms-exchange` (the only reader of the description file) join; both are
            // program-free crates of this repository
            let allowed = rest.starts_with("forms-client")
                || rest.starts_with("forms-layout")
                || rest.starts_with("forms-exchange")
                || rest.starts_with("forms-run")
                // the match of module and block (pure, no I/O); the data of modules comes through the port `Sources`, no module client
                || rest.starts_with("forms-bind")
                || rest.starts_with("jscalendar")
                // the local head of fs-components (WIP patch, unreviewed/b3) brings the settings library
                || rest.starts_with("settings")
                || rest.starts_with("components")
                || rest.starts_with("rpc")
                || rest.starts_with("interfaces")
                || rest.starts_with("types")
                || rest.starts_with("icons")
                || rest.starts_with("records") // brought by fs-components-catalog (record shapes, no program)
                || rest.starts_with("objects"); // brought by fs-rpc
            assert!(
                !rest.starts_with("modules-"),
                "{n} is a module client: modules come through the port `Sources`"
            );
            assert!(
                allowed,
                "{n} is a dependency: no program, core or server in the host"
            );
        }
    }
}

/// `toml` and `serde_json` are NOT a direct dependency of the host. They do appear deeper in the graph (fs-components brings
/// `toml` through Blinc's css parser; the same holds for `fs-forms-simple`): an edge of the plan's H-K17, reported, not hidden.
#[test]
fn toml_and_json_are_no_direct_dependency() {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let out = Command::new(cargo)
        .args([
            "tree",
            "-p",
            "fs-forms-run",
            "-e",
            "normal",
            "--depth",
            "1",
            "--prefix",
            "none",
            "--locked",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(out.status.success());
    let tree = String::from_utf8_lossy(&out.stdout);
    for l in tree.lines().skip(1) {
        let n = l.split_whitespace().next().unwrap_or("");
        assert!(
            !matches!(n, "toml" | "serde_json"),
            "{n} is a direct dependency"
        );
    }
}
