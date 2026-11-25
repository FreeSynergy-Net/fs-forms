//! S6-7 picture: the SAME description (the board of Tasks with the node `settings-embed`) once with `mode = panel` and once with
//! `mode = window`. In panel mode the settings frontend is drawn inside the board (one window); in window mode the board shows
//! nothing of it and the settings frontend is a second window. The two pictures must differ in far more pixels than the tolerance
//! (the lesson: equal bytes prove nothing, and a picture without the panel must NOT pass as the picture with it).
//!
//! Counter-test (the failure case): the picture of the window mode board differs from the panel mode board by at least `MIN_DIFF`
//! pixels; a host that drew nothing for the panel would give equal pictures and fail here.
//! The pictures are written to `FS_PICTURE_OUT` when set (`stufe6-panel.png`, `stufe6-fenster.png`, `stufe6-fenster-settings.png`);
//! without a graphics adapter the test is skipped (like the others).

mod embed_common;

use std::path::Path;

use embed_common::{open_board, world};

/// A channel may differ by this much between two draws of the same state (anti-aliasing of the renderer).
const TOL: u8 = 2;
/// The panel is a whole form (a tree and a table): far more pixels than any rounding.
const MIN_DIFF: usize = 5000;
/// The size of the pictures (the board and the panel under it).
const SIZE: (u32, u32) = (1100, 1000);

type Pixels = (u32, u32, Vec<u8>);

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

fn diff(a: &Pixels, b: &Pixels) -> usize {
    assert_eq!((a.0, a.1), (b.0, b.1), "same size");
    a.2.chunks(3)
        .zip(b.2.chunks(3))
        .filter(|(x, y)| x.iter().zip(*y).any(|(p, q)| p.abs_diff(*q) > TOL))
        .count()
}

fn draw(window: &fs_components_app::AppWidget, path: &Path) -> Option<Pixels> {
    match window.picture(SIZE.0, SIZE.1, path) {
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
fn the_same_description_draws_the_panel_inside_or_a_second_window() {
    let temp = tempfile::tempdir().unwrap();
    let out =
        std::env::var_os("FS_PICTURE_OUT").map_or_else(|| temp.path().to_path_buf(), Into::into);
    let panel = world("panel");
    open_board(&panel);
    let window = world("window");
    open_board(&window);
    let first = |w: &embed_common::World, n: usize| w.shown.0.lock().unwrap()[n].1.clone();
    assert_eq!(
        panel.shown.0.lock().unwrap().len(),
        1,
        "panel mode: one window"
    );
    assert_eq!(
        window.shown.0.lock().unwrap().len(),
        2,
        "window mode: two windows"
    );
    let Some(in_board) = draw(&first(&panel, 0), &out.join("stufe6-panel.png")) else {
        return;
    };
    let alone = draw(&first(&window, 0), &out.join("stufe6-fenster.png")).unwrap();
    let settings = draw(&first(&window, 1), &out.join("stufe6-fenster-settings.png")).unwrap();
    let (a, b) = (diff(&in_board, &alone), diff(&in_board, &settings));
    eprintln!("panel vs window board: {a} pixels, panel vs settings window: {b} pixels");
    assert!(
        a >= MIN_DIFF,
        "the panel shows in the board: only {a} pixels differ"
    );
    assert!(
        diff(&alone, &settings) >= MIN_DIFF,
        "board and settings window are two pictures"
    );
}
