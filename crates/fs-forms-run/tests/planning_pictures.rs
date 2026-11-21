//! B4: the pictures of the window with REAL records, made through the planning service (see `support/service.rs`): the board
//! before and after each action (a card added, a card moved, a column added), and the host program given the socket of a service.
//! The pictures are kept under the folder named by `FS_PICTURE_OUT`, else in the target folder of the test (no fixed path). Where the machine has no graphics adapter a test says so and ends green.
//!
//! Tests: after each action the next drawing differs from the one before; the host program draws the card made in the service.
#![cfg(unix)]

mod support;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use fs_components_kanban::Answer;
use fs_forms_run::PlanningRecords;
use support::service::{connect, pictures_dir, start};

/// Wide enough that a fifth column is in the picture.
const WIDE: u32 = 1700;

type Step = Box<dyn Fn(&PlanningRecords)>;

fn actions() -> Vec<(&'static str, Step)> {
    vec![
        (
            "add-card",
            Box::new(|r| {
                r.add_card("todo", "Ship the first release");
            }),
        ),
        (
            "move-card",
            Box::new(|r| {
                let key = r.columns()[0].cards[0].key.clone();
                r.move_card(&key, "todo", "doing");
            }),
        ),
        (
            "add-column",
            Box::new(|r| {
                r.add_column("Review");
            }),
        ),
        (
            "rename",
            Box::new(|r| {
                r.rename_column("todo", "Backlog");
            }),
        ),
        (
            "limit",
            Box::new(|r| {
                // two cards in "To do": a limit of one shows the column over its limit
                r.set_limit("todo", Some(1));
            }),
        ),
    ]
}

use fs_forms_run::Records as _;

#[test]
#[cfg_attr(
    windows,
    ignore = "wgpu Validation Error on windows-2022 without a GPU adapter"
)]
fn the_pictures_show_real_records_of_the_service_before_and_after_each_action() {
    use fs_forms_run::LayoutLink as _;
    let dir = pictures_dir();
    let locale = fs_forms_run::session_locale();
    let texts = fs_forms_run::words(&locale);
    let view = fs_forms_run::FileLink::new(support::board(), "en")
        .resolve("", "")
        .unwrap();
    let comp = fs_forms_run::compose(&view, &fs_forms_run::Blocks);
    for (name, act) in actions() {
        let service = start();
        let records = Arc::new(connect(&service));
        // the data is made THROUGH the service: nothing of it is built into the host
        for (column, title) in [
            ("todo", "Write the spec"),
            ("todo", "Review the plan"),
            ("doing", "Build the board"),
            ("doing", "Read the description"),
            ("done", "Draw the first picture"),
        ] {
            assert_eq!(records.add_card(column, title), Answer::Accepted);
        }
        let widget = fs_forms_run::app_styled(
            &texts,
            &locale,
            &view,
            &comp,
            records.clone(),
            Arc::new(fs_forms_run::StyleCell::default()),
        )
        .unwrap();
        let (before, after) = (
            dir.join(format!("real-{name}-before.png")),
            dir.join(format!("real-{name}-after.png")),
        );
        if let Err(e) = widget.picture(WIDE, 760, &before) {
            let e = e.to_string().to_lowercase();
            assert!(e.contains("graphics") || e.contains("adapter"), "{e}");
            eprintln!("skipped: no graphics adapter ({e})");
            return;
        }
        act(&records);
        widget.picture(WIDE, 760, &after).unwrap();
        let read = |p: &Path| std::fs::read(p).unwrap();
        // `assert!` on a bool: a failed `assert_eq!` would print both pictures as bytes
        assert!(read(&before) != read(&after), "{name}: the action shows");
    }
}

#[test]
#[cfg_attr(
    windows,
    ignore = "wgpu Validation Error on windows-2022 without a GPU adapter"
)]
fn the_host_program_draws_the_board_of_the_service_it_is_given_by_socket() {
    let service = start();
    let records = connect(&service);
    records.add_card("todo", "Seen through the host program");
    let path = pictures_dir().join("real-host-program.png");
    let out = Command::new(env!("CARGO_BIN_EXE_fs-forms-run"))
        .arg("--description")
        .arg(support::board())
        .arg("--socket")
        .arg(&service.socket)
        .arg("--picture")
        .arg(&path)
        .arg("--size")
        .arg("1400x760")
        .env_clear()
        .env("HOME", service.home.path())
        .env("XDG_DATA_HOME", service.home.path().join("data"))
        .env("XDG_RUNTIME_DIR", service.home.path().join("run"))
        .env("LANG", "en_US.UTF-8")
        .env("FS_COLOR_SCHEME", "light")
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr).into_owned();
    if out.status.code() != Some(0) {
        assert!(
            err.to_lowercase().contains("graphics") || err.to_lowercase().contains("adapter"),
            "failed for another reason: {err}"
        );
        eprintln!("skipped: no graphics adapter ({err})");
        return;
    }
    assert!(path.is_file(), "no picture was written");
}
