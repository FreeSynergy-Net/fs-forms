//! A view reads the time only through the port `Clock` (`src/clock.rs`): no direct read of the wall clock anywhere else in the host.
//!
//! Test: reads the sources of this crate. Mutant: a `SystemTime::now()` in `src/draw.rs` -> red, and it names the file.
//! (Why no clippy `disallowed-methods`: the layout store and a test of `fs-forms-core` read the clock too, in other crates; a ban for
//! the workspace needs an exception per site, and this crate is the one that draws. Card: widen it when those two take a port.)

use std::path::{Path, PathBuf};

const FORBIDDEN: &[&str] = &[
    "SystemTime::now",
    "Instant::now",
    "Utc::now",
    "Local::now",
    "OffsetDateTime::now",
];

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("dir") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn only_the_system_clock_reads_the_wall_clock() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_files(&src, &mut files);
    assert!(files.len() > 20, "the sources were found");
    let mut found = Vec::new();
    for file in files {
        if file.file_name().is_some_and(|n| n == "clock.rs") {
            continue;
        }
        let text = std::fs::read_to_string(&file).expect("read");
        for (n, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or_default();
            if let Some(hit) = FORBIDDEN.iter().find(|f| code.contains(**f)) {
                found.push(format!("{}:{}: {hit}", file.display(), n + 1));
            }
        }
    }
    assert!(
        found.is_empty(),
        "read the time through `Clock`: {found:#?}"
    );
}
