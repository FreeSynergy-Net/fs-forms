//! A picture of the window, drawn without a window (`--picture`): the same shell and surfaces that the window uses. Where the
//! machine has no graphics adapter the test says so and ends green, as the render tests of fs-components do.
//!
//! Tests: Pixel tests; the picture goes to a temp folder and is read back.

use std::process::Command;

fn picture(screen: &str, path: &std::path::Path) -> Option<(u32, u32, Vec<u8>)> {
    let out = Command::new(env!("CARGO_BIN_EXE_fs-forms-simple"))
        .args(["--socket", "/nonexistent/forms.sock", "--screen", screen])
        .arg("--picture")
        .arg(path)
        .env("LANG", "en_US.UTF-8")
        .env_remove("LC_ALL")
        .env_remove("LC_MESSAGES")
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr).into_owned();
    if out.status.code() != Some(0) {
        assert!(
            err.to_lowercase().contains("graphics") || err.to_lowercase().contains("adapter"),
            "failed for another reason: {err}"
        );
        eprintln!("skipped: no graphics adapter ({err})");
        return None;
    }
    let decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).unwrap()));
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    Some((info.width, info.height, buf))
}

#[test]
fn both_pages_are_drawn_at_the_size_asked_and_differ() {
    let dir = tempfile::tempdir().unwrap();
    let Some((w, h, start)) = picture("start", &dir.path().join("start.png")) else {
        return;
    };
    assert_eq!((w, h), (1000, 760));
    assert!(start.iter().any(|b| *b != start[0]), "not one flat colour");
    let (_, _, comes) = picture("comes", &dir.path().join("comes.png")).unwrap();
    assert_ne!(start, comes, "the two pages are not the same picture");
}
