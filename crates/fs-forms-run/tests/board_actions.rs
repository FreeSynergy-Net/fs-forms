//! K3: the board of the description drawn before and after each action on the sample board (a card added, a card moved, a column
//! added), pixel for pixel. The sample data is in memory and says so in the status line ("Sample data (not your records)").
//! Where the machine has no graphics adapter a test says so and ends green. The pictures stay under the target of the test
//! (`k3-pictures/k3-<action>-before.png`, `-after.png`).
//!
//! Tests: after each action the next drawing differs from the one before; two drawings without an action are equal (the
//! difference is the action, not noise). The data path of the gestures is tested in `src/write/tests.rs` and `fs-components-kanban`.

mod support;
use std::sync::Arc;

use fs_forms_run::{Records, SampleBoard};
use support::board;

/// Wide enough that a fourth column is in the picture.
const WIDE: u32 = 1700;

type Step = Box<dyn Fn(&SampleBoard)>;

fn actions() -> Vec<(&'static str, Step)> {
    vec![
        (
            "add-card",
            Box::new(|b| {
                b.add_card("todo", "Ship the first release");
            }),
        ),
        (
            "move-card",
            Box::new(|b| {
                b.move_card("FS-1", "doing");
            }),
        ),
        (
            "add-column",
            Box::new(|b| {
                b.add_column("Review");
            }),
        ),
    ]
}

#[test]
#[cfg_attr(
    windows,
    ignore = "wgpu Validation Error on windows-2022 without a GPU adapter"
)]
fn each_action_on_the_sample_board_shows_in_the_next_drawing_of_the_same_window() {
    use fs_forms_run::LayoutLink as _;
    // the pictures are kept under the target of the test (never a user folder): `k3-<action>-before.png` and `-after.png`
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("k3-pictures");
    std::fs::create_dir_all(&dir).unwrap();
    let locale = fs_forms_run::session_locale();
    let texts = fs_forms_run::words(&locale);
    let view = fs_forms_run::FileLink::new(board(), "en")
        .resolve("", "")
        .unwrap();
    let comp = fs_forms_run::compose(&view, &fs_forms_run::Blocks);
    for (name, act) in actions() {
        let records = Arc::new(SampleBoard::default());
        let widget = fs_forms_run::app_styled(
            &texts,
            &locale,
            &view,
            &comp,
            records.clone(),
            Arc::new(fs_forms_run::StyleCell::default()),
        )
        .unwrap();
        let (before, again, after) = (
            dir.join(format!("k3-{name}-before.png")),
            dir.join(format!("k3-{name}-again.png")),
            dir.join(format!("k3-{name}-after.png")),
        );
        if let Err(e) = widget.picture(WIDE, 760, &before) {
            let e = e.to_string().to_lowercase();
            assert!(e.contains("graphics") || e.contains("adapter"), "{e}");
            eprintln!("skipped: no graphics adapter ({e})");
            return;
        }
        widget.picture(WIDE, 760, &again).unwrap();
        act(&records);
        widget.picture(WIDE, 760, &after).unwrap();
        let read = |p: &std::path::Path| std::fs::read(p).unwrap();
        // `assert!` on a bool: a failed `assert_eq!` would print both pictures as bytes
        assert!(
            read(&before) == read(&again),
            "{name}: no action, no difference"
        );
        assert!(read(&before) != read(&after), "{name}: the action shows");
        std::fs::remove_file(&again).unwrap();
    }
}
