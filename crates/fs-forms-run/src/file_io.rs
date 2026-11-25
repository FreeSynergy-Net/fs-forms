//! The small file steps under `FileLink`: errors, links that are never followed, bounded reads, one checked description.
//!
//! Pattern: Adapter (files to the model of `fs-forms-layout`), the plain I/O part of `file`.

use std::fmt::Write as _;
use std::fs;
use std::io::{ErrorKind, Read};
use std::path::{Component, Path};

use fs_forms_exchange::{Format, LAYOUT_BYTES_MAX, read_layout};
use fs_forms_layout::{Layout, Limits, OpenCatalog};

use crate::error::HostError;

pub(super) fn missing(path: &Path) -> HostError {
    HostError::DescriptionMissing {
        path: path.display().to_string(),
    }
}

pub(super) fn unreadable(file: &Path, problems: Vec<String>) -> HostError {
    HostError::LayoutUnreadable {
        file: file.display().to_string(),
        problems,
    }
}

pub(super) fn describe<C: fs_forms_layout::ErrorCode>(c: &fs_forms_layout::Coded<C>) -> String {
    let mut out = c.code.message_key();
    for (k, v) in &c.params {
        let _ = write!(out, " {k}={v}");
    }
    out
}

/// A link (symbolic link) is never followed below the path the person named: it could lead out of the folder.
pub(super) fn refused_link(path: &Path) -> HostError {
    unreadable(
        path,
        vec![format!("layout-link-refused path={}", path.display())],
    )
}

pub(super) fn is_link(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink())
}

/// Every step of `rel` below `root` (the folder itself is the person's word, what is inside it is not).
pub(super) fn link_inside(root: &Path, rel: &Path) -> Result<(), HostError> {
    let mut at = root.to_path_buf();
    for part in rel.components() {
        at.push(part);
        if is_link(&at) {
            return Err(refused_link(&at));
        }
    }
    Ok(())
}

/// The text of a small file; `None` when there is no such file. Larger than [`LAYOUT_BYTES_MAX`] or not UTF-8 is an error.
pub(super) fn read_small(path: &Path) -> Result<Option<String>, HostError> {
    let failed = || unreadable(path, vec!["layout-read-failed".to_owned()]);
    let mut f = match fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(failed()),
    };
    let mut bytes = Vec::new();
    (&mut f)
        .take(LAYOUT_BYTES_MAX + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| failed())?;
    if bytes.len() as u64 > LAYOUT_BYTES_MAX {
        return Err(unreadable(
            path,
            vec![format!("layout-too-big max={LAYOUT_BYTES_MAX}")],
        ));
    }
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| unreadable(path, vec!["layout-read-failed cause=utf8".to_owned()]))
}

/// Read and check one file.
pub(super) fn read_file(path: &Path, limits: &Limits) -> Result<Layout, HostError> {
    let mut f = fs::File::open(path).map_err(|_| missing(path))?;
    let doc = read_layout(&mut f, path, Format::Toml)
        .map_err(|e| unreadable(path, e.iter().map(describe).collect()))?;
    doc.check(&OpenCatalog, limits)
        .map_err(|e| unreadable(path, e.iter().map(describe).collect()))
}

/// A path named inside a blueprint may not leave the folder.
pub(super) fn inside(rel: &str) -> bool {
    let p = Path::new(rel);
    !rel.is_empty()
        && p.components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}
