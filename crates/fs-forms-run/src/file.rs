//! Reading a description from disk: a blueprint folder (`scaffold.toml` names the file), a folder of `forms/*.toml`, or one file.
//! The host knows no program: the path decides, nothing is built in. A description that is missing or unreadable is a
//! [`HostError`] that the window shows (`host-error-layout-missing`, `host-error-layout-unreadable`), never an empty window.
//!
//! Pattern: Adapter (files to the model of `fs-forms-layout`), behind the port `LayoutLink`.

use std::fmt::Write as _;
use std::fs;
use std::path::{Component, Path, PathBuf};

use fs_forms_exchange::{Format, read_layout, scaffold_layout};
use fs_forms_layout::{Layout, Limits, OpenCatalog};

use crate::error::HostError;
use crate::from_layout::view_of;
use crate::link::LayoutLink;
use crate::view::View;

/// The description at a path.
#[derive(Clone, Debug)]
pub struct FileLink {
    root: PathBuf,
    lang: String,
}

impl FileLink {
    /// A link to the description at `root`; `lang` is the language of the words of the window (`en` when a text has no twin).
    #[must_use]
    pub fn new(root: impl Into<PathBuf>, lang: impl Into<String>) -> Self {
        Self {
            root: root.into(),
            lang: lang.into(),
        }
    }
}

fn missing(path: &Path) -> HostError {
    HostError::DescriptionMissing {
        path: path.display().to_string(),
    }
}

fn unreadable(file: &Path, problems: Vec<String>) -> HostError {
    HostError::LayoutUnreadable {
        file: file.display().to_string(),
        problems,
    }
}

fn describe<C: fs_forms_layout::ErrorCode>(c: &fs_forms_layout::Coded<C>) -> String {
    let mut out = c.code.message_key();
    for (k, v) in &c.params {
        let _ = write!(out, " {k}={v}");
    }
    out
}

/// Read and check one file.
fn read_file(path: &Path) -> Result<Layout, HostError> {
    let mut f = fs::File::open(path).map_err(|_| missing(path))?;
    let doc = read_layout(&mut f, path, Format::Toml)
        .map_err(|e| unreadable(path, e.iter().map(describe).collect()))?;
    doc.check(&OpenCatalog, &Limits::declared())
        .map_err(|e| unreadable(path, e.iter().map(describe).collect()))
}

/// A path named inside a blueprint may not leave the folder.
fn inside(rel: &str) -> bool {
    let p = Path::new(rel);
    !rel.is_empty()
        && p.components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}

fn descriptions_in(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "toml"))
        .collect();
    files.sort();
    files
}

/// The layout a path leads to; `frontend` (may be empty) picks one of several files.
fn locate(root: &Path, frontend: &str) -> Result<(PathBuf, Layout), HostError> {
    if root.is_file() {
        return Ok((root.to_path_buf(), read_file(root)?));
    }
    if !root.is_dir() {
        return Err(missing(root));
    }
    let scaffold = root.join("scaffold.toml");
    if let Ok(text) = fs::read_to_string(&scaffold) {
        let named =
            scaffold_layout(&text).map_err(|c| unreadable(&scaffold, vec![describe(&c)]))?;
        if let Some(rel) = named {
            if !inside(&rel) {
                return Err(unreadable(
                    &scaffold,
                    vec![format!("layout-path-outside path={rel}")],
                ));
            }
            let file = root.join(rel);
            let layout = read_file(&file)?;
            return Ok((file, layout));
        }
    }
    let forms = root.join("forms");
    let dir = if forms.is_dir() {
        forms
    } else {
        root.to_path_buf()
    };
    let files = descriptions_in(&dir);
    let mut found: Vec<(PathBuf, Layout)> = Vec::new();
    for file in &files {
        let layout = read_file(file)?;
        if frontend.is_empty() || layout.doc().key.frontend == frontend {
            found.push((file.clone(), layout));
        }
    }
    match found.len() {
        1 => Ok(found.remove(0)),
        _ => Err(missing(&dir)),
    }
}

impl LayoutLink for FileLink {
    fn resolve(&self, _extends: &str, frontend: &str) -> Result<View, HostError> {
        let (_file, layout) = locate(&self.root, frontend)?;
        Ok(view_of(&layout, &self.lang))
    }
}
