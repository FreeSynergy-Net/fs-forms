//! Every description of the repository (the fixtures of the host and of the test data, the calendar frontends) passes the check of its
//! gestures against the REAL block catalog (`Blocks::rules()`): each `[[action]]` with `at` names a command slot of the block at that
//! place, within its `repeat`, and no gesture is left without a place (`action-invalid`). More directories (e.g. the forms of a program
//! repository) are checked when `FS_DESCRIPTION_DIRS` names them (separated like `PATH`); there every file must read as a description and
//! every directory must exist (a read error is a failure). Not set = only the own directories, whose non-description fixtures are skipped.
//!
//! Mutant: removing a `slot` rename in a description (or the `at` of an action) makes this test fail with the file and the cause.
//!
//! Pattern: none (tests).

use std::fs;
use std::path::{Path, PathBuf};

use fs_forms_exchange::{Format, read_layout};
use fs_forms_layout::{LayoutCode, Limits};
use fs_forms_run::Blocks;

/// The `.toml` files below `dir`; a directory that cannot be read is a problem, never "no files".
fn files_in(dir: &Path, out: &mut Vec<PathBuf>, bad: &mut Vec<String>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            bad.push(format!("{}: directory not readable: {e}", dir.display()));
            return;
        }
    };
    for entry in entries {
        let p = match entry {
            Ok(e) => e.path(),
            Err(e) => {
                bad.push(format!("{}: entry not readable: {e}", dir.display()));
                continue;
            }
        };
        if p.is_dir() {
            files_in(&p, out, bad);
        } else if p.extension().is_some_and(|x| x == "toml")
            && p.file_name().is_some_and(|n| n != "scaffold.toml")
        {
            out.push(p);
        }
    }
}

/// The directories to check and whether each file there MUST read as a description. The own directories of the repository hold
/// fixtures that are no frontend description (samples, wishes): those may not read and are not the subject. The directories named by
/// `FS_DESCRIPTION_DIRS` (not set = none) are descriptions of a program repository: every file there must read, or the test fails.
fn dirs() -> Vec<(PathBuf, bool)> {
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut dirs = vec![
        (here.join("tests/fixtures"), false),
        (
            here.join("../fs-forms-testdata/tests/fixtures/descriptions"),
            false,
        ),
        (here.join("../../forms"), false),
    ];
    if let Some(extra) = std::env::var_os("FS_DESCRIPTION_DIRS") {
        dirs.extend(std::env::split_paths(&extra).map(|d| (d, true)));
    }
    dirs
}

#[test]
fn every_action_of_every_description_fits_the_block_it_is_at() {
    let (mut checked, mut bad) = (0, Vec::new());
    for (dir, must_read) in dirs() {
        let mut files = Vec::new();
        files_in(&dir, &mut files, &mut bad);
        for f in &files {
            let mut file = match fs::File::open(f) {
                Ok(file) => file,
                Err(e) => {
                    bad.push(format!("{}: file not readable: {e}", f.display()));
                    continue;
                }
            };
            let doc = match read_layout(&mut file, f, Format::Toml) {
                Ok(doc) => doc,
                Err(e) => {
                    if must_read {
                        bad.push(format!(
                            "{}: not a readable description: {e:?}",
                            f.display()
                        ));
                    }
                    continue;
                }
            };
            if doc.actions.is_empty() {
                continue;
            }
            checked += 1;
            let problems = doc
                .check(&Blocks::rules(), &Limits::declared())
                .err()
                .unwrap_or_default();
            for c in problems.iter().filter(|c| {
                matches!(
                    c.code,
                    LayoutCode::ActionInvalid | LayoutCode::BindingInvalid
                )
            }) {
                let cause = c.param("cause").unwrap_or("");
                bad.push(format!(
                    "{}: {} {cause}",
                    f.display(),
                    c.param("elem").unwrap_or("")
                ));
            }
        }
    }
    assert!(bad.is_empty(), "{bad:#?}");
    assert!(
        checked >= 8,
        "the descriptions with actions are found: {checked}"
    );
}
