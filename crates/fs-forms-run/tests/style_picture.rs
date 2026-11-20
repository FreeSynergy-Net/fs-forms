//! The style of the window: a description that says `default` gets the style set for the program (`--style`), asked at every
//! drawing; two styles draw the same description differently.
//!
//! Tests: pixel tests (skipped without a graphics adapter). Mutants: the style is read once at start; `default` is always admin;
//! `--style` is ignored.

mod support;
use std::path::Path;
use support::{board, draw, out_dir};

#[test]
#[cfg_attr(
    windows,
    ignore = "wgpu Validation Error on windows-2022 without a GPU adapter"
)]
fn the_same_description_looks_different_in_two_styles_and_the_style_is_asked_at_every_drawing() {
    let (_keep, dir) = out_dir();
    let board = board();
    let admin = dir.join("board-sample-style-admin-light.png");
    let apple = dir.join("board-sample-style-apple-light.png");
    let sample = |style: &str, path: &Path| {
        draw(
            &["--description", &board, "--sample-data", "--style", style],
            "light",
            path,
        )
    };
    let Some(a) = sample("admin", &admin) else {
        return;
    };
    let p = sample("apple", &apple).unwrap();
    assert_ne!(
        a.rgb, p.rgb,
        "admin and apple differ for the same description"
    );
    let plain = draw(
        &["--description", &board, "--sample-data"],
        "light",
        &dir.join("board-sample-style-preset-light.png"),
    )
    .unwrap();
    assert_eq!(plain.rgb, a.rgb, "no --style: the preset is admin");
    // dark too
    let da = draw(
        &["--description", &board, "--sample-data", "--style", "admin"],
        "dark",
        &dir.join("board-sample-style-admin-dark.png"),
    )
    .unwrap();
    let dp = draw(
        &["--description", &board, "--sample-data", "--style", "apple"],
        "dark",
        &dir.join("board-sample-style-apple-dark.png"),
    )
    .unwrap();
    assert_ne!(da.rgb, dp.rgb);
}

#[test]
#[cfg_attr(
    windows,
    ignore = "wgpu Validation Error on windows-2022 without a GPU adapter"
)]
fn a_change_of_the_style_set_for_the_program_shows_in_the_next_drawing_of_the_same_window() {
    use fs_forms_run::LayoutLink as _;
    use std::sync::Arc;
    let (_keep, dir) = out_dir();
    let locale = fs_forms_run::session_locale();
    let texts = fs_forms_run::words(&locale);
    let view = fs_forms_run::FileLink::new(board(), "en")
        .resolve("", "")
        .unwrap();
    let comp = fs_forms_run::compose(&view, &fs_forms_run::Blocks);
    let cell = Arc::new(fs_forms_run::StyleCell::default());
    let widget = fs_forms_run::app_styled(
        &texts,
        &locale,
        &view,
        &comp,
        Arc::new(fs_forms_run::SampleBoard::default()),
        cell.clone(),
    )
    .unwrap();
    let (first, second) = (dir.join("live-1.png"), dir.join("live-2.png"));
    if let Err(e) = widget.picture(1000, 760, &first) {
        let e = e.to_string().to_lowercase();
        assert!(e.contains("graphics") || e.contains("adapter"), "{e}");
        eprintln!("skipped: no graphics adapter ({e})");
        return;
    }
    cell.set(fs_components_styles::StyleId::Apple);
    widget.picture(1000, 760, &second).unwrap();
    assert_ne!(
        std::fs::read(&first).unwrap(),
        std::fs::read(&second).unwrap(),
        "the second drawing of the same window shows the new style"
    );
}
