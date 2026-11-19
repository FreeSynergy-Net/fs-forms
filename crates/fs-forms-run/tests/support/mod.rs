//! Shared by the picture tests: the window drawn through the host binary (`--picture`) and read back as pixels.

#![allow(dead_code)] // each test file uses a part

use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Picture {
    pub w: u32,
    pub h: u32,
    pub rgb: Vec<u8>,
}

pub fn out_dir() -> (Option<tempfile::TempDir>, PathBuf) {
    if let Some(d) = std::env::var_os("FS_PICTURE_OUT") {
        return (None, PathBuf::from(d));
    }
    let t = tempfile::tempdir().unwrap();
    let p = t.path().to_path_buf();
    (Some(t), p)
}

pub fn draw(args: &[&str], scheme: &str, path: &Path) -> Option<Picture> {
    let out = Command::new(env!("CARGO_BIN_EXE_fs-forms-run"))
        .args(args)
        .arg("--picture")
        .arg(path)
        .env("LANG", "en_US.UTF-8")
        .env("FS_COLOR_SCHEME", scheme)
        .env_remove("FS_RENDER_SCHEME")
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
    let step = info.color_type.samples();
    let rgb = buf[..info.buffer_size()]
        .chunks(step)
        .flat_map(|p| p[..3].to_vec())
        .collect();
    Some(Picture {
        w: info.width,
        h: info.height,
        rgb,
    })
}

#[allow(clippy::cast_precision_loss)] // a picture has far fewer than 2^52 samples
pub fn mean(p: &Picture) -> f64 {
    p.rgb.iter().map(|b| f64::from(*b)).sum::<f64>() / p.rgb.len() as f64
}

pub fn board() -> String {
    format!("{}/tests/fixtures/board.toml", env!("CARGO_MANIFEST_DIR"))
}

/// Whether every pixel of the box has the colour of its first one.
pub fn uniform(p: &Picture, x: std::ops::Range<u32>, y: std::ops::Range<u32>) -> bool {
    let at = |x: u32, y: u32| {
        let i = ((y * p.w + x) * 3) as usize;
        [p.rgb[i], p.rgb[i + 1], p.rgb[i + 2]]
    };
    let first = at(x.start, y.start);
    y.clone().all(|yy| x.clone().all(|xx| at(xx, yy) == first))
}
