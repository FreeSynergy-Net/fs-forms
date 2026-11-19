//! K5-5: the window of a real description, drawn through `fs-components` without a window (`--picture`), read back as pixels.
//! Where the machine has no graphics adapter a test says so and ends green (as the render tests of fs-components do).
//!
//! Tests: pixel tests. (1) the description of the frontend `board` of fs-tasks gives a window of the size asked, not one flat
//! colour, with something dark in it; (2) light differs from dark and the dark picture is darker on average; (3) a missing
//! description gives a window too (the visible error `host-error-layout-missing`) that differs from the board window.
//! Mutants: the host ignores `--description` (all pictures equal); the error window is blank; dark is drawn as light.
//! Set `FS_PICTURE_OUT=<dir>` to keep the pictures (they are then written there instead of a temp folder).

mod support;
use support::{Picture, board, draw, mean, out_dir, uniform};

#[test]
#[cfg_attr(
    windows,
    ignore = "wgpu Validation Error on windows-2022 without a GPU adapter"
)]
fn the_description_of_a_program_is_drawn_light_and_dark_and_a_missing_one_says_so() {
    let (_keep, dir) = out_dir();
    let board = board();
    let Some(light) = draw(
        &["--description", &board],
        "light",
        &dir.join("board-light.png"),
    ) else {
        return;
    };
    assert_eq!((light.w, light.h), (1000, 760));
    assert!(
        light.rgb.iter().any(|b| *b != light.rgb[0]),
        "not one flat colour"
    );
    let dark = draw(
        &["--description", &board],
        "dark",
        &dir.join("board-dark.png"),
    )
    .unwrap();
    assert_ne!(light.rgb, dark.rgb, "dark is not the light picture");
    assert!(
        mean(&dark) + 20.0 < mean(&light),
        "dark {} is darker than light {}",
        mean(&dark),
        mean(&light)
    );
    let gone = dir.join("no-such-description");
    let gone = gone.to_str().unwrap();
    let missing = draw(
        &["--description", gone],
        "light",
        &dir.join("layout-missing-light.png"),
    )
    .unwrap();
    assert!(
        missing.rgb.iter().any(|b| *b != missing.rgb[0]),
        "the error window is not blank"
    );
    assert_ne!(
        light.rgb, missing.rgb,
        "the error window is not the board window"
    );
    let missing_dark = draw(
        &["--description", gone],
        "dark",
        &dir.join("layout-missing-dark.png"),
    )
    .unwrap();
    assert_ne!(missing.rgb, missing_dark.rgb);
}

#[test]
#[cfg_attr(
    windows,
    ignore = "wgpu Validation Error on windows-2022 without a GPU adapter"
)]
fn with_sample_records_the_board_draws_cards_in_columns_light_and_dark() {
    let (_keep, dir) = out_dir();
    let board = board();
    let args = ["--description", board.as_str(), "--sample-data"];
    let Some(empty) = draw(
        &["--description", &board],
        "light",
        &dir.join("e-light.png"),
    ) else {
        return;
    };
    let light = draw(&args, "light", &dir.join("board-sample-light.png")).unwrap();
    let dark = draw(&args, "dark", &dir.join("board-sample-dark.png")).unwrap();
    assert_ne!(light.rgb, empty.rgb, "sample records change the window");
    assert_ne!(light.rgb, dark.rgb);
    assert!(mean(&dark) + 20.0 < mean(&light));
    // cards are drawn: the picture has far more distinct colours than the plain one (cards, labels, due marks)
    let colours = |p: &Picture| {
        p.rgb
            .chunks(3)
            .map(|c| (c[0], c[1], c[2]))
            .collect::<std::collections::HashSet<_>>()
            .len()
    };
    assert!(
        colours(&light) > colours(&empty) + 5,
        "{} vs {}",
        colours(&light),
        colours(&empty)
    );
}

#[test]
#[cfg_attr(
    windows,
    ignore = "wgpu Validation Error on windows-2022 without a GPU adapter"
)]
fn the_side_list_stands_beside_the_board_not_above_it_light_and_dark() {
    let (_keep, dir) = out_dir();
    let board = board();
    for scheme in ["light", "dark"] {
        let name = format!("board-sample-side-{scheme}.png");
        let Some(p) = draw(
            &["--description", &board, "--sample-data"],
            scheme,
            &dir.join(name),
        ) else {
            return;
        };
        // the strip (x 200..420 of a 1000 px window, the shell's own navigation takes the left 200): the list is in its top
        // part, nothing below it
        assert!(
            !uniform(&p, 240..300, 70..180),
            "{scheme}: the list is drawn in the strip, 24 px (not 96) from its edge"
        );
        assert!(
            uniform(&p, 230..410, 300..700),
            "{scheme}: the strip is empty below the list"
        );
        // the board starts right of the strip, at the top of the main area (a board below the list would leave this box blank)
        assert!(
            !uniform(&p, 440..680, 125..200),
            "{scheme}: the board begins beside the list"
        );
        // the board is not shifted: its first column is at the left edge of the main area, not far right of it
        assert!(
            uniform(&p, 421..440, 130..700),
            "{scheme}: nothing of the board left of its own padding"
        );
        assert!(
            !uniform(&p, 440..450, 130..700),
            "{scheme}: the first column starts 24 px (not 96) from the main area"
        );
    }
}
