//! The public functions: read a description from a file, write a layout canonically.
//!
//! Pattern: Adapter (file to model and back).

use std::io::{Read, Write};
use std::path::Path;

use fs_forms_layout::{Coded, Layout, LayoutCode, LayoutDoc};

use crate::fields::Ctx;
use crate::tree::{self, Tree};
use crate::{emit, read, write};

/// The file format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// TOML.
    Toml,
    /// JSON (RFC 8785 canonical on output); needs feature `json`.
    Json,
}

fn problem(file: &Path, line: u32, cause: &str) -> Coded<LayoutCode> {
    Coded::new(LayoutCode::ImportInvalid)
        .with("file", file.display())
        .with("line", line)
        .with("cause", cause)
}

fn parse(text: &str, file: &Path, format: Format) -> Result<Tree, Coded<LayoutCode>> {
    match format {
        Format::Toml => {
            let doc = toml_edit::ImDocument::parse(text).map_err(|e| {
                problem(
                    file,
                    e.span().map_or(0, |s| tree::line_at(text, s.start)),
                    "toml",
                )
            })?;
            tree::from_item(doc.as_item(), text).map_err(|c| problem(file, 0, c))
        }
        #[cfg(feature = "json")]
        Format::Json => {
            let v: serde_json::Value = serde_json::from_str(text)
                .map_err(|e| problem(file, u32::try_from(e.line()).unwrap_or(0), "json"))?;
            tree::from_json(v).map_err(|c| problem(file, 0, c))
        }
        #[cfg(not(feature = "json"))]
        Format::Json => Err(problem(file, 0, "json-not-built")),
    }
}

/// Reads a description. Problems: `import-invalid{file,line,cause}` with cause `toml`, `json`, `type`, `key-unknown`,
/// `key-missing`, `utf8`; ALL problems of the file, never only the first. Unknown keys are errors. CRLF is accepted, a BOM is not.
///
/// # Errors
/// Every problem found.
pub fn read_layout(
    src: &mut dyn Read,
    file: &Path,
    format: Format,
) -> Result<LayoutDoc, Vec<Coded<LayoutCode>>> {
    let mut bytes = Vec::new();
    src.read_to_end(&mut bytes)
        .map_err(|_| vec![problem(file, 0, "read")])?;
    let text = String::from_utf8(bytes).map_err(|_| vec![problem(file, 0, "utf8")])?;
    if text.starts_with('\u{feff}') {
        return Err(vec![problem(file, 1, "utf8")]);
    }
    let tree = parse(&text, file, format).map_err(|e| vec![e])?;
    let mut cx = Ctx {
        src: &text,
        file: &file.display().to_string(),
        errs: Vec::new(),
    };
    let doc = read::layout_doc(&tree, &mut cx);
    match (doc, cx.errs.is_empty()) {
        (Some(d), true) => Ok(d),
        _ => Err(cx.errs),
    }
}

/// Writes a layout, always format 2 and canonical. `own_views`: include the views of a person (default off: they are the
/// person's data, not the package's).
///
/// # Errors
/// `export-failed` when the layout holds something this cut cannot write, or the output fails.
pub fn write_layout(
    layout: &Layout,
    format: Format,
    own_views: bool,
    out: &mut dyn Write,
) -> Result<(), Coded<LayoutCode>> {
    let tree = write::layout_tree(layout, own_views)?;
    let text = match format {
        Format::Toml => emit::to_toml(&tree),
        Format::Json => emit::to_json(&tree),
    };
    out.write_all(text.as_bytes()).map_err(|_| {
        Coded::new(LayoutCode::ExportFailed)
            .with("file", "")
            .with("kind", "io")
    })
}
