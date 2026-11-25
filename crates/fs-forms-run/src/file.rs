//! Reading a description from disk: a blueprint folder (`scaffold.toml` names the file), a folder of `forms/*.toml`, or one file.
//! The host knows no program: the path decides, nothing is built in. A description that is missing or unreadable is a
//! [`HostError`] that the window shows (`host-error-layout-missing`, `host-error-layout-unreadable`), never an empty window.
//!
//! Pattern: Adapter (files to the model of `fs-forms-layout`), behind the port `LayoutLink`.

use std::fs;

use std::path::{Path, PathBuf};

use fs_forms_exchange::scaffold_layout;
use fs_forms_layout::{Layout, Limits};

use super::file_io::{
    describe, inside, link_inside, missing, read_file, read_small, refused_link, unreadable,
};
use crate::error::HostError;
use crate::from_layout::view_of;
use crate::link::LayoutLink;
use crate::view::View;

/// The description at a path.
#[derive(Clone, Debug)]
pub struct FileLink {
    root: PathBuf,
    lang: String,
    limits: Limits,
}

impl FileLink {
    /// A link to the description at `root`; `lang` is the language of the words of the window (`en` when a text has no twin).
    #[must_use]
    pub fn new(root: impl Into<PathBuf>, lang: impl Into<String>) -> Self {
        Self {
            root: root.into(),
            lang: lang.into(),
            limits: Limits::declared(),
        }
    }

    /// The limits of the instance (its scheme policy included) that every file is checked against; the default is `Limits::declared()`.
    #[must_use]
    pub fn with_limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }
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
fn locate(
    root: &Path,
    limits: &Limits,
    extends: &str,
    frontend: &str,
) -> Result<(PathBuf, Layout), HostError> {
    if root.is_file() {
        let layout = read_file(root, limits)?;
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
            let layout = read_file(&file, limits)?;
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
        let layout = read_file(&file, limits)?;
        if asked_for(&layout, extends, frontend) {
            found.push((file, layout));
        }
    }
    match found.len() {
        1 => Ok(found.remove(0)),
        0 if !frontend.is_empty() => Err(HostError::FrontendUnknown {
            id: frontend.to_owned(),
            available: available_in(&dir, limits, extends)?,
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
fn available_in(dir: &Path, limits: &Limits, extends: &str) -> Result<Vec<String>, HostError> {
    let mut ids: Vec<String> = Vec::new();
    for file in descriptions_in(dir)? {
        let layout = read_file(&file, limits)?;
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
    /// The ids of all frontends the folder describes for `extends`, sorted.
    ///
    /// # Errors
    /// The folder or a description in it could not be read.
    pub fn available(&self, extends: &str) -> Result<Vec<String>, HostError> {
        let root = &self.root;
        if root.is_file() {
            let layout = read_file(root, &self.limits)?;
            return Ok(if asked_for(&layout, extends, "") {
                vec![layout.doc().key.frontend.clone()]
            } else {
                Vec::new()
            });
        }
        link_inside(root, Path::new("forms"))?;
        let forms = root.join("forms");
        available_in(
            if forms.is_dir() { &forms } else { root },
            &self.limits,
            extends,
        )
    }

    /// Like `resolve`, but says which file was loaded.
    ///
    /// # Errors
    /// The same as `resolve`; a `frontend` that no description carries is `frontend-unknown` with the ids that exist.
    pub fn load(&self, extends: &str, frontend: &str) -> Result<Loaded, HostError> {
        let (file, layout) = locate(&self.root, &self.limits, extends, frontend)?;
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
        let (_file, layout) = locate(&self.root, &self.limits, extends, frontend)?;
        Ok(view_of(&layout, &self.lang))
    }
}
