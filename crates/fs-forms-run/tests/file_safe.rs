//! A5/N1/N2 (fs-forms v0.0.3): the host reads foreign descriptions with a byte limit, follows no link out of the folder, says an
//! unreadable blueprint and checks the one file against `--frontend` / `--extends`. Real files in a temp folder under the target.
//! Mutants: no byte limit on the scaffold; an unreadable scaffold skipped; `is_link` always false; the one file taken whatever
//! `--frontend` says.

use std::fs;
use std::path::PathBuf;

use fs_forms_run::{FileLink, HostError, LayoutLink};

const BOARD: &str = include_str!("fixtures/board.toml");

fn dir(name: &str) -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("k5-file-safe")
        .join(name);
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

fn resolve(root: &PathBuf, frontend: &str) -> Result<fs_forms_run::View, HostError> {
    FileLink::new(root, "en").resolve("", frontend)
}

fn problems(e: &HostError) -> String {
    match e {
        HostError::LayoutUnreadable { problems, .. } => problems.join(" | "),
        other => panic!("wrong error {other:?}"),
    }
}

#[test]
fn a_scaffold_that_is_there_but_unreadable_is_said_not_skipped() {
    let d = dir("scaffold-unreadable");
    fs::create_dir_all(d.join("forms")).unwrap();
    fs::write(d.join("forms/board.toml"), BOARD).unwrap();
    // it would read fine without the scaffold: the folder has exactly one description
    assert_eq!(resolve(&d, "").unwrap().frontend, "board");
    fs::write(d.join("scaffold.toml"), [0xff, 0xfe, 0x00]).unwrap();
    let e = resolve(&d, "").unwrap_err();
    assert_eq!(e.code(), "layout-unreadable");
    assert!(problems(&e).contains("layout-read-failed"), "{e}");
    // a folder in its place cannot be read either
    fs::remove_file(d.join("scaffold.toml")).unwrap();
    fs::create_dir(d.join("scaffold.toml")).unwrap();
    assert_eq!(resolve(&d, "").unwrap_err().code(), "layout-unreadable");
}

#[test]
fn a_description_over_the_byte_limit_is_a_visible_error() {
    let max = usize::try_from(fs_forms_exchange::LAYOUT_BYTES_MAX).unwrap();
    let d = dir("too-big");
    fs::write(d.join("scaffold.toml"), vec![b'#'; max + 1]).unwrap();
    let e = resolve(&d, "").unwrap_err();
    assert_eq!(e.code(), "layout-unreadable");
    assert!(problems(&e).contains("layout-too-big"), "{e}");
    fs::remove_file(d.join("scaffold.toml")).unwrap();
    fs::write(d.join("b.toml"), vec![b'#'; max + 1]).unwrap();
    let e = resolve(&d.join("b.toml"), "").unwrap_err();
    assert_eq!(e.code(), "layout-unreadable");
    assert!(problems(&e).contains("too-big"), "{e}");
}

#[test]
fn one_file_must_be_the_frontend_and_the_program_that_was_asked_for() {
    let d = dir("one-file");
    let f = d.join("a.toml");
    fs::write(&f, BOARD).unwrap();
    let link = FileLink::new(&f, "en");
    assert!(link.resolve("net.freesynergy.Tasks", "board").is_ok());
    assert!(link.resolve("", "").is_ok(), "no word asks for nothing");
    assert_eq!(
        link.resolve("", "other").unwrap_err().code(),
        "layout-missing"
    );
    assert_eq!(
        link.resolve("net.freesynergy.Other", "board")
            .unwrap_err()
            .code(),
        "layout-missing"
    );
}

/// Links lead out of the folder: refused with a code (Linux only; the Windows runner may not make links).
#[cfg(unix)]
mod links {
    use std::os::unix::fs::symlink;

    use super::*;

    #[test]
    fn a_scaffold_that_cannot_be_opened_is_said_not_skipped() {
        use std::os::unix::fs::PermissionsExt;
        let d = dir("scaffold-denied");
        fs::create_dir_all(d.join("forms")).unwrap();
        fs::write(d.join("forms/board.toml"), BOARD).unwrap();
        let scaffold = d.join("scaffold.toml");
        fs::write(&scaffold, "[program]\nname = \"x\"\n").unwrap();
        fs::set_permissions(&scaffold, fs::Permissions::from_mode(0o000)).unwrap();
        if fs::File::open(&scaffold).is_ok() {
            return; // a user that may read everything (root): nothing to test here
        }
        let e = resolve(&d, "").unwrap_err();
        assert_eq!(e.code(), "layout-unreadable", "{e}");
        assert!(problems(&e).contains("layout-read-failed"), "{e}");
    }

    fn refused(d: &PathBuf) {
        let e = resolve(d, "").unwrap_err();
        assert_eq!(e.code(), "layout-unreadable", "{e}");
        assert!(problems(&e).contains("layout-link-refused"), "{e}");
    }

    #[test]
    fn a_link_in_forms_that_leads_out_is_refused() {
        let d = dir("link-file");
        let outside = dir("link-file-outside");
        fs::write(outside.join("o.toml"), BOARD).unwrap();
        fs::create_dir_all(d.join("bp/forms")).unwrap();
        symlink(outside.join("o.toml"), d.join("bp/forms/board.toml")).unwrap();
        refused(&d.join("bp"));
    }

    #[test]
    fn a_link_for_forms_itself_is_refused() {
        let d = dir("link-forms");
        let outside = dir("link-forms-outside");
        fs::write(outside.join("o.toml"), BOARD).unwrap();
        symlink(&outside, d.join("forms")).unwrap();
        refused(&d);
    }

    #[test]
    fn a_link_named_by_the_blueprint_and_a_link_as_the_blueprint_are_refused() {
        let d = dir("link-scaffold");
        let outside = dir("link-scaffold-outside");
        fs::write(outside.join("o.toml"), BOARD).unwrap();
        fs::write(
            d.join("scaffold.toml"),
            "[frontend]\nlayout = \"forms/board.toml\"\n",
        )
        .unwrap();
        symlink(&outside, d.join("forms")).unwrap();
        refused(&d);
        fs::remove_file(d.join("forms")).unwrap();
        fs::create_dir(d.join("forms")).unwrap();
        symlink(outside.join("o.toml"), d.join("forms/board.toml")).unwrap();
        refused(&d);
        fs::remove_file(d.join("scaffold.toml")).unwrap();
        fs::write(outside.join("scaffold.toml"), "[program]\nname = \"x\"\n").unwrap();
        symlink(outside.join("scaffold.toml"), d.join("scaffold.toml")).unwrap();
        refused(&d);
    }
}
