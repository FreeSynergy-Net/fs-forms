//! Finds the block contract of the `fs-components-catalog` this crate is built with and hands its files to the crate
//! (`OUT_DIR/contract.rs`): the host and the catalog can then never disagree about the contract version. The catalog crate of
//! `v0.0.30` does not embed the files itself (its doc says it should); until it does, the files are read from the checkout the
//! crate itself comes from (the whole repository is there), located through `cargo metadata` of this very build.
//!
//! No JSON reader is pulled in for this: the one path is cut out of the text.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The directory `contract/` of the repository of the catalog crate.
fn contract_dir() -> PathBuf {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let target = std::env::var("TARGET").expect("TARGET");
    let out = Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--locked", "--offline"])
        .args(["--filter-platform", &target])
        .arg("--manifest-path")
        .arg(Path::new(&manifest).join("Cargo.toml"))
        .output()
        .expect("cargo metadata starts");
    assert!(
        out.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8(out.stdout).expect("metadata is UTF-8");
    // `"manifest_path":"<...>/crates/fs-components-catalog/Cargo.toml"` (backslashes are escaped on Windows)
    let end = [
        "/fs-components-catalog/Cargo.toml\"",
        "\\\\fs-components-catalog\\\\Cargo.toml\"",
    ]
    .iter()
    .filter_map(|needle| text.find(needle).map(|i| i + needle.len() - 1))
    .min()
    .expect("fs-components-catalog is a dependency");
    let start = text[..end].rfind('"').expect("a quoted path") + 1;
    let manifest_path = PathBuf::from(text[start..end].replace("\\\\", "\\"));
    manifest_path
        .parent() // fs-components-catalog
        .and_then(Path::parent) // crates
        .and_then(Path::parent) // the repository
        .expect("the catalog crate sits in crates/ of its repository")
        .join("contract")
}

fn literal(path: &Path) -> String {
    // forward slashes read on every system; the path holds no quote
    format!(
        "include_str!(\"{}\")",
        path.to_string_lossy().replace('\\', "/")
    )
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.lock");
    let dir = contract_dir();
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir.join("blocks"))
        .expect("contract/blocks is there")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .collect();
    files.sort(); // the contract reads the files in file name order
    let mut code = format!(
        "/// `contract/contract.toml` of the catalog's repository.\npub(crate) const CONTRACT: &str = {};\n",
        literal(&dir.join("contract.toml"))
    );
    code.push_str("/// `contract/blocks/*.toml`, in file name order.\npub(crate) const BLOCK_FILES: &[(&str, &str)] = &[\n");
    for f in &files {
        println!("cargo:rerun-if-changed={}", f.display());
        let name = f.file_name().expect("a file name").to_string_lossy();
        writeln!(code, "    (\"{name}\", {}),", literal(f)).expect("write to a String");
    }
    code.push_str("];\n");
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR")).join("contract.rs");
    std::fs::write(out, code).expect("contract.rs is written");
}
