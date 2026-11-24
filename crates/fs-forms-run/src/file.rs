//! Reading a description from disk: a blueprint folder (`scaffold.toml` names the file), a folder of `forms/*.toml`, or one file.
//! The host knows no program: the path decides, nothing is built in. A description that is missing or unreadable is a
//! [`HostError`] that the window shows (`host-error-layout-missing`, `host-error-layout-unreadable`), never an empty window.
//!
//! Pattern: Adapter (files to the model of `fs-forms-layout`), behind the port `LayoutLink`.

use std::fmt::Write as _;
use std::fs;
use std::io::{ErrorKind, Read};
use std::path::{Component, Path, PathBuf};

use fs_forms_exchange::{Format, LAYOUT_BYTES_MAX, read_layout, scaffold_layout};
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

/// A link (symbolic link) is never followed below the path the person named: it could lead out of the folder.
fn refused_link(path: &Path) -> HostError {
    unreadable(
        path,
        vec![format!("layout-link-refused path={}", path.display())],
    )
}

fn is_link(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink())
}

/// Every step of `rel` below `root` (the folder itself is the person's word, what is inside it is not).
fn link_inside(root: &Path, rel: &Path) -> Result<(), HostError> {
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
fn read_small(path: &Path) -> Result<Option<String>, HostError> {
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

fn descriptions_in(dir: &Path) -> Result<Vec<PathBuf>, HostError> {
    let mut files: Vec<PathBuf> = Vec::new();
    for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        // the blueprint is not a description
        if path.file_name().is_some_and(|n| n == "scaffold.toml") {
            continue;
        }
        if path.extension().is_none_or(|x| x != "toml") {
            continue;
        }
        match entry.file_type() {
            Ok(t) if t.is_symlink() => return Err(refused_link(&path)),
            Ok(t) if t.is_file() => files.push(path),
            _ => {}
        }
    }
    files.sort();
    Ok(files)
}

/// Whether the layout is the one asked for (an empty word asks for nothing).
fn asked_for(layout: &Layout, extends: &str, frontend: &str) -> bool {
    let key = &layout.doc().key;
    (extends.is_empty() || key.extends == extends)
        && (frontend.is_empty() || key.frontend == frontend)
}

/// The layout a path leads to; `extends` and `frontend` (may be empty) pick one of several files or refuse the one file.
fn locate(root: &Path, extends: &str, frontend: &str) -> Result<(PathBuf, Layout), HostError> {
    if root.is_file() {
        let layout = read_file(root)?;
        return if asked_for(&layout, extends, frontend) {
            Ok((root.to_path_buf(), layout))
        } else {
            Err(missing(root))
        };
    }
    if !root.is_dir() {
        return Err(missing(root));
    }
    let scaffold = root.join("scaffold.toml");
    link_inside(root, Path::new("scaffold.toml"))?;
    if let Some(text) = read_small(&scaffold)? {
        let named =
            scaffold_layout(&text).map_err(|c| unreadable(&scaffold, vec![describe(&c)]))?;
        if let Some(rel) = named {
            if !inside(&rel) {
                return Err(unreadable(
                    &scaffold,
                    vec![format!("layout-path-outside path={rel}")],
                ));
            }
            link_inside(root, Path::new(&rel))?;
            let file = root.join(rel);
            let layout = read_file(&file)?;
            // the blueprint names the standard: it wins only when it is what was asked for (empty asks for the standard)
            if asked_for(&layout, extends, frontend) {
                return Ok((file, layout));
            }
        }
    }
    link_inside(root, Path::new("forms"))?;
    let forms = root.join("forms");
    let dir = if forms.is_dir() {
        forms
    } else {
        root.to_path_buf()
    };
    let mut found: Vec<(PathBuf, Layout)> = Vec::new();
    for file in descriptions_in(&dir)? {
        let layout = read_file(&file)?;
        if asked_for(&layout, extends, frontend) {
            found.push((file, layout));
        }
    }
    match found.len() {
        1 => Ok(found.remove(0)),
        0 if !frontend.is_empty() => Err(HostError::FrontendUnknown {
            id: frontend.to_owned(),
            available: available_in(&dir, extends)?,
        }),
        _ if !frontend.is_empty() => Err(HostError::FrontendAmbiguous {
            id: frontend.to_owned(),
            files: found
                .iter()
                .map(|(f, _)| {
                    let rel = f.strip_prefix(root).unwrap_or(f);
                    rel.to_string_lossy().replace('\\', "/")
                })
                .collect(),
        }),
        _ => Err(missing(&dir)),
    }
}

/// The ids of all frontends the folder describes, sorted (what `frontend-unknown` offers).
fn available_in(dir: &Path, extends: &str) -> Result<Vec<String>, HostError> {
    let mut ids: Vec<String> = Vec::new();
    for file in descriptions_in(dir)? {
        let layout = read_file(&file)?;
        if asked_for(&layout, extends, "") {
            ids.push(layout.doc().key.frontend.clone());
        }
    }
    ids.sort();
    ids.dedup();
    Ok(ids)
}

/// A description that was really loaded: the view, the file it came from and the id it carries.
///
/// Pattern: Value object: the answer of [`FileLink::load`], so a host or a check can say which file it shows.
#[derive(Clone, Debug)]
pub struct Loaded {
    /// The resolved description.
    pub view: View,
    /// The file that was read, relative to the folder the link was made with (for a single file: its file name).
    pub file: PathBuf,
}

impl Loaded {
    /// `frontend <id> from <file>`: the line a host or check logs, from what was loaded and not from what was asked.
    #[must_use]
    pub fn line(&self) -> String {
        let file = self.file.to_string_lossy().replace('\\', "/");
        format!("frontend {} from {file}", self.view.frontend)
    }
}

impl FileLink {
    /// Like `resolve`, but says which file was loaded.
    ///
    /// # Errors
    /// The same as `resolve`; a `frontend` that no description carries is `frontend-unknown` with the ids that exist.
    pub fn load(&self, extends: &str, frontend: &str) -> Result<Loaded, HostError> {
        let (file, layout) = locate(&self.root, extends, frontend)?;
        // a single file is its own root: strip_prefix then leaves nothing, so the name of the file is taken
        let file = match file.strip_prefix(&self.root) {
            Ok(rel) if !rel.as_os_str().is_empty() => rel.to_path_buf(),
            _ => file.file_name().map_or_else(|| file.clone(), PathBuf::from),
        };
        Ok(Loaded {
            view: view_of(&layout, &self.lang),
            file,
        })
    }
}

impl LayoutLink for FileLink {
    fn resolve(&self, extends: &str, frontend: &str) -> Result<View, HostError> {
        let (_file, layout) = locate(&self.root, extends, frontend)?;
        Ok(view_of(&layout, &self.lang))
    }
}
