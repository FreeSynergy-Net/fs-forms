//! The picture "two windows, one record" (stage 6, O4a): the board and the compact board of ONE program show the same record. A moves
//! a card; B, which never wrote, shows the moved card after its own `watch` (the drawing reads the shared cache, read again once).
//!
//! Reference: there is no stored picture. The reference of "B shows the change" is a third window C opened AFTER the move on the same
//! record: it reads the new state fresh, by no path that B took. B after the move must equal C (tolerance 0 pixels beyond rounding of
//! `TOL` per channel). Counter-test against the tolerance trap: B BEFORE the move (the failure case: B did not learn of the change)
//! must differ from C by at least `MIN_DIFF` pixels, far over the tolerance, so a window that stays stale cannot pass as equal.
//!
//! Mutant (run, red): a write through the grip is refused -> the move never happens, the test fails at the move. NOT caught by this
//! test (run, stayed green): the cache is not read again on `observe`; the sample board is one live object, so the drawing shows the
//! state anyway. The mechanism (own `watch`, one reread per revision, the wake) is proven by `host_windows` H1 and `record_hub`
//! GA5, not by this picture; the picture proves the outcome, as the person sees it.
//! The pictures are written to `FS_PICTURE_OUT` when set; without a graphics adapter the test is skipped (like the others).

mod host_common;

use std::path::Path;

use fs_components_kanban::Answer;
use fs_forms_run::{WindowOpened as Opened, WindowOwner};
use host_common::{req, world};

/// A channel may differ by this much between two draws of the same state (anti-aliasing of the renderer).
const TOL: u8 = 2;
/// The failure case must differ in at least this many pixels (a card moved: far more).
const MIN_DIFF: usize = 400;

fn read(path: &Path) -> (u32, u32, Vec<u8>) {
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
fn diff(a: &(u32, u32, Vec<u8>), b: &(u32, u32, Vec<u8>)) -> usize {
    assert_eq!((a.0, a.1), (b.0, b.1), "same size");
    a.2.chunks(3)
        .zip(b.2.chunks(3))
        .filter(|(p, q)| p.iter().zip(*q).any(|(x, y)| x.abs_diff(*y) > TOL))
        .count()
}

fn draw(w: &fs_components_app::AppWidget, path: &Path) -> Option<(u32, u32, Vec<u8>)> {
    match w.picture(900, 560, path) {
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
fn two_windows_one_record_the_other_window_shows_the_move() {
    let dir = std::env::var_os("FS_PICTURE_OUT").map_or_else(
        || {
            let t = tempfile::tempdir().unwrap();
            (Some(t.path().to_path_buf()), Some(t))
        },
        |d| (Some(d.into()), None),
    );
    let (out, _keep) = (dir.0.unwrap(), dir.1);
    let w = world();
    let (Opened::New(a), Opened::New(b)) = (
        w.owner.open(req("board", "urn:a")).unwrap(),
        w.owner.open(req("board-compact", "urn:a")).unwrap(),
    ) else {
        panic!("two new windows")
    };
    let widget = |n: usize| w.shown.widgets.lock().unwrap()[n].1.clone();
    let Some(a_before) = draw(&widget(0), &out.join("two-windows-a-before.png")) else {
        return;
    };
    let b_before = draw(&widget(1), &out.join("two-windows-b-before.png")).unwrap();
    assert_eq!(
        w.owner
            .grip_of(&a)
            .unwrap()
            .records()
            .move_card("FS-1", "done"),
        Answer::Accepted
    );
    let b_after = draw(&widget(1), &out.join("two-windows-b-after.png")).unwrap();
    let a_after = draw(&widget(0), &out.join("two-windows-a-after.png")).unwrap();
    // C: a fresh window on the same record AFTER the move (B's place is free once B is closed): the reference no path of B went through
    w.owner.close(&b).unwrap();
    let Opened::New(_) = w.owner.open(req("board-compact", "urn:a")).unwrap() else {
        panic!("a new window")
    };
    let c = draw(&widget(1), &out.join("two-windows-c-fresh.png")).unwrap();
    assert!(
        diff(&a_before, &a_after) >= MIN_DIFF,
        "the move shows in A (else the test moves nothing)"
    );
    assert_eq!(
        diff(&b_after, &c),
        0,
        "B shows what a fresh window shows: the moved card"
    );
    let stale = diff(&b_before, &c);
    assert!(
        stale >= MIN_DIFF,
        "the failure case (B stale) is {stale} pixels away from the reference, far over the tolerance"
    );
}
