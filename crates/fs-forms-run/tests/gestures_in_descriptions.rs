//! Every description of the repository (the fixtures of the host and of the test data, the calendar frontends) passes the check of its
//! gestures against the REAL block catalog (`Blocks::rules()`): each `[[action]]` with `at` names a command slot of the block at that
//! place, within its `repeat`, and no gesture is left without a place (`action-invalid`). More directories (e.g. the forms of a program
//! repository) are checked when `FS_DESCRIPTION_DIRS` names them (separated like `PATH`).
//!
//! Mutant: removing a `slot` rename in a description (or the `at` of an action) makes this test fail with the file and the cause.

use std::fs;
use std::path::{Path, PathBuf};

use fs_forms_exchange::{Format, read_layout};
use fs_forms_layout::{LayoutCode, Limits};
use fs_forms_run::Blocks;

fn files_in(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = entry.path();
        if p.is_dir() {
            files_in(&p, out);
        } else if p.extension().is_some_and(|x| x == "toml")
            && p.file_name().is_some_and(|n| n != "scaffold.toml")
        {
            out.push(p);
        }
    }
}

fn dirs() -> Vec<PathBuf> {
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut dirs = vec![
        here.join("tests/fixtures"),
        here.join("../fs-forms-testdata/tests/fixtures/descriptions"),
        here.join("../../forms"),
    ];
    if let Some(extra) = std::env::var_os("FS_DESCRIPTION_DIRS") {
        dirs.extend(std::env::split_paths(&extra));
    }
    dirs
}

/// Open until the block `quick-add` (programs.toml, `accepts = []`) has a command slot `quick-add` (card for C4): the action of the
/// entry row of `day.toml` is then correctly red. The test says so itself, and fails when it is fixed (the entry must then go).
const OPEN_UNTIL_C4: [(&str, &str); 1] = [("day.toml", "slot-unknown")];

#[test]
fn every_action_of_every_description_fits_the_block_it_is_at() {
    let mut files = Vec::new();
    for d in dirs() {
        files_in(&d, &mut files);
    }
    let (mut checked, mut open, mut bad) = (0, Vec::new(), Vec::new());
    for f in &files {
        let Ok(mut file) = fs::File::open(f) else {
            continue;
        };
        // fixtures that are no frontend description (samples, wishes) do not read as one: they are not the subject here
        let Ok(doc) = read_layout(&mut file, f, Format::Toml) else {
            continue;
        };
        if doc.actions.is_empty() {
            continue;
        }
        checked += 1;
        let name = f.file_name().and_then(|n| n.to_str()).unwrap_or("");
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
            if OPEN_UNTIL_C4.contains(&(name, cause)) {
                open.push((name.to_owned(), cause.to_owned()));
            } else {
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
    for (name, cause) in OPEN_UNTIL_C4 {
        assert!(
            open.iter().any(|(n, c)| n == name && c == cause),
            "{name} is fixed now: take it out of OPEN_UNTIL_C4"
        );
    }
}
