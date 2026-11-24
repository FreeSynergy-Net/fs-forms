//! The picture "frontend switch" (stage 6, C1): one window, one record; the person changes the frontend (`board` -> `board-compact`)
//! and the window shows the other frontend, in the same window.
//!
//! Reference: a FRESH window opened directly in `board-compact` on the same record (another world, so nothing of the switched window
//! is in it). The switched window after the change must equal it (no pixel beyond `TOL` per channel). Counter-test against the
//! tolerance trap: the window BEFORE the change (the failure case: the change did not show) must differ from the reference by at least
//! `MIN_DIFF` pixels, far over the tolerance, and so must the window before from the window after; a window that stays as it was cannot
//! pass as equal. The pictures go to `FS_PICTURE_OUT` when set (`frontend-wechsel-vorher.png`, `frontend-wechsel.png`); without a
//! graphics adapter the test is skipped (like the others).
//!
//! Mutant (run, red): `use_frontend` exchanges the key but not the page in the cell -> the window after equals the window before and
//! the test fails at the first comparison.

mod host_common;

use std::path::Path;

use fs_forms_run::{WindowOpened as Opened, WindowOwner};
use host_common::{req, world};

/// A channel may differ by this much between two draws of the same state (anti-aliasing of the renderer).
const TOL: u8 = 2;
/// The failure case must differ in at least this many pixels (a card moved: far more).
const MIN_DIFF: usize = 400;

fn read(path: &Path) -> Pixels {
    let d = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).unwrap()));
    let mut r = d.read_info().unwrap();
    let mut buf = vec![0; r.output_buffer_size().unwrap()];
    let i = r.next_frame(&mut buf).unwrap();
    let step = i.color_type.samples();
    let rgb = buf[..i.buffer_size()]
        .chunks(step)
        .flat_map(|p| p[..3].to_vec())
        .collect();
    (i.width, i.height, rgb)
}

/// How many pixels differ by more than `TOL` in any channel.
type Pixels = (u32, u32, Vec<u8>);

fn diff(first: &Pixels, second: &Pixels) -> usize {
    assert_eq!((first.0, first.1), (second.0, second.1), "same size");
    first
        .2
        .chunks(3)
        .zip(second.2.chunks(3))
        .filter(|(left, right)| left.iter().zip(*right).any(|(x, y)| x.abs_diff(*y) > TOL))
        .count()
}

fn draw(window: &fs_components_app::AppWidget, path: &Path) -> Option<Pixels> {
    match window.picture(900, 560, path) {
        Ok(()) => Some(read(path)),
        Err(e) => {
            let m = e.to_string().to_lowercase();
            assert!(
                m.contains("graphics") || m.contains("adapter"),
                "another reason: {m}"
            );
            eprintln!("skipped: no graphics adapter ({m})");
            None
        }
    }
}

#[test]
#[cfg_attr(
    windows,
    ignore = "wgpu Validation Error on windows-2022 without a GPU adapter"
)]
fn the_window_shows_the_other_frontend_after_the_change() {
    let dir = std::env::var_os("FS_PICTURE_OUT").map_or_else(
        || {
            let t = tempfile::tempdir().unwrap();
            (Some(t.path().to_path_buf()), Some(t))
        },
        |d| (Some(d.into()), None),
    );
    let (out, _keep) = (dir.0.unwrap(), dir.1);
    let host = world();
    let Opened::New(id) = host.owner.open(req("board", "urn:a")).unwrap() else {
        panic!("a new window")
    };
    let window = host.shown.widgets.lock().unwrap()[0].1.clone();
    let Some(before) = draw(&window, &out.join("frontend-wechsel-vorher.png")) else {
        return;
    };
    host.owner.use_frontend(&id, "board-compact").unwrap();
    let after = draw(&window, &out.join("frontend-wechsel.png")).unwrap();
    // the reference: a fresh window of the other frontend in a world of its own
    let other = world();
    let Opened::New(_) = other.owner.open(req("board-compact", "urn:a")).unwrap() else {
        panic!("a new window")
    };
    let fresh_window = other.shown.widgets.lock().unwrap()[0].1.clone();
    let fresh = draw(&fresh_window, &out.join("frontend-wechsel-referenz.png")).unwrap();
    assert!(
        diff(&before, &after) >= MIN_DIFF,
        "the window changed (else the test switches nothing)"
    );
    let stale = diff(&before, &fresh);
    assert!(
        stale >= MIN_DIFF,
        "the failure case (not switched) is {stale} pixels away from the reference, far over the tolerance"
    );
    assert_eq!(
        diff(&after, &fresh),
        0,
        "the switched window shows what a fresh window of that frontend shows"
    );
}
