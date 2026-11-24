//! `--steps`: the file is read and refused before anything is built (no graphics needed); with a graphics adapter the input of a
//! file moves a card of the fake module through the written way (`card.move` reaches the writer of the fixture) and the picture is
//! written after it. Where the machine has no adapter the test says so and ends green.
//!
//! Mutants (by hand, red): the picture written although the plan was refused; `read` lets `pointerCancel` through; the plan
//! played after the picture; a missing element exits 1 instead of 2.
//!
//! Pattern: none (tests).

use std::sync::Arc;

use fs_components_app::replay::parse;

use super::{play, read};
use crate::file::FileLink;
use crate::link::LayoutLink;
use crate::write::fixture::{FakeWriter, data, records};

const ELEMENT: &str = "element-6066-11e4-a52e-4f735466cecf";

fn body(actions: &str) -> String {
    format!(
        r#"{{"actions":[{{"type":"pointer","id":"mouse","parameters":{{"pointerType":"mouse"}},"actions":[{actions}]}}]}}"#
    )
}

fn click(id: &str) -> String {
    format!(
        r#"{{"type":"pointerMove","origin":{{"{ELEMENT}":"{id}"}},"x":0,"y":0}},{{"type":"pointerDown","button":0}},{{"type":"pointerUp","button":0}},{{"type":"pause","duration":100}}"#
    )
}

fn folder() -> tempfile::TempDir {
    // under the target folder of the workspace, never a system temp folder
    let target = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target");
    std::fs::create_dir_all(&target).unwrap();
    tempfile::tempdir_in(target).unwrap()
}

#[test]
fn a_file_that_is_not_there_or_not_playable_is_a_message_naming_the_file() {
    let keep = folder();
    let dir = keep.path();
    let gone = dir.join("not-there.json");
    assert!(read(&gone).unwrap_err().contains("cannot read"));
    for (name, text, says) in [
        ("broken.json", "{", "steps-invalid"),
        (
            "touch.json",
            r#"{"actions":[{"type":"pointer","id":"p","parameters":{"pointerType":"touch"},"actions":[]}]}"#,
            "steps-unsupported",
        ),
    ] {
        let file = dir.join(name);
        std::fs::write(&file, text).unwrap();
        let message = read(&file).unwrap_err();
        assert!(
            message.contains(says) && message.contains(name),
            "{message}"
        );
    }
}

#[test]
fn an_empty_input_is_valid_and_plays_nothing() {
    let keep = folder();
    let file = keep.path().join("empty.json");
    std::fs::write(&file, r#"{"actions":[]}"#).unwrap();
    assert!(read(&file).is_ok());
}

/// The board of the fixture folder with the actions that call the commands of the fake module by their keys (`drop` calls
/// `card.move`, `quick-add` calls `task.create`), as a module that binds its commands is described.
fn widget(writer: &Arc<FakeWriter>, dir: &std::path::Path) -> fs_components_app::AppWidget {
    let text = include_str!("../../tests/fixtures/board.toml")
        .replace(
            "trigger = \"drop\"\nsource = 1\ncommand = \"move\"",
            "trigger = \"drop\"\nsource = 1\ncommand = \"card.move\"",
        )
        .replace(
            "trigger = \"quick-add\"\nsource = 1\ncommand = \"create\"",
            "trigger = \"quick-add\"\nsource = 1\ncommand = \"task.create\"",
        );
    let board = dir.join("board.toml");
    std::fs::write(&board, text).unwrap();
    let locale = crate::words::session_locale();
    let texts = crate::words::words(&locale);
    let view = FileLink::new(board, "en").resolve("", "").unwrap();
    let comp = crate::compose::compose(&view, &crate::blocks::Blocks);
    crate::window::app_styled(
        &texts,
        &locale,
        &view,
        &comp,
        Arc::new(records(writer, data())),
        Arc::new(crate::style::StyleCell::default()),
    )
    .unwrap()
}

fn no_graphics(message: &str) -> bool {
    let m = message.to_lowercase();
    let none = m.contains("graphics") || m.contains("adapter");
    if none {
        eprintln!("skipped: no graphics adapter ({message})");
    }
    none
}

#[test]
#[cfg_attr(
    windows,
    ignore = "wgpu Validation Error on windows-2022 without a GPU adapter"
)]
fn the_input_of_a_file_moves_a_card_and_then_the_picture_is_written() {
    let keep = folder();
    let out = keep.path().join("moved.png");
    let _ = std::fs::remove_file(&out);
    let writer = Arc::new(FakeWriter::default());
    let plan = parse(&body(&format!(
        "{},{},{}",
        click("kb-card-a"),
        click("kb-grab-a"),
        click("kb-col-doing")
    )))
    .unwrap();
    if let Err((message, code)) = play(
        &widget(&writer, keep.path()),
        &plan,
        (1200, 800),
        &out,
        2,
        1,
    ) {
        assert!(no_graphics(&message) && code == 1, "{message} ({code})");
        return;
    }
    let calls = writer.calls.lock().unwrap();
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert_eq!(calls[0].command, "card.move");
    assert_eq!(calls[0].record.as_deref(), Some("a"));
    assert!(out.metadata().unwrap().len() > 1000, "the picture is there");
}

#[test]
#[cfg_attr(
    windows,
    ignore = "wgpu Validation Error on windows-2022 without a GPU adapter"
)]
fn an_element_that_is_not_there_is_exit_2_nothing_is_sent_and_no_picture_is_written() {
    let keep = folder();
    let out = keep.path().join("never.png");
    let _ = std::fs::remove_file(&out);
    let writer = Arc::new(FakeWriter::default());
    let plan = parse(&body(&click("kb-card-nobody"))).unwrap();
    let (message, code) = play(
        &widget(&writer, keep.path()),
        &plan,
        (1200, 800),
        &out,
        2,
        1,
    )
    .unwrap_err();
    if no_graphics(&message) {
        return;
    }
    assert_eq!(code, 2, "{message}");
    assert!(message.contains("steps-element-missing"), "{message}");
    assert!(writer.calls.lock().unwrap().is_empty());
    assert!(!out.exists(), "no picture after a refused input");
}
