//! The one fact the host needs from a blueprint (`scaffold.toml`): where the forms description of the frontend lies
//! (`[frontend] layout = "forms/board.toml"`). Nothing else of the blueprint is read here.
//!
//! Pattern: Adapter (file to one value).

use fs_forms_layout::{Coded, LayoutCode};

use crate::tree::line_at;

/// The relative path named by `[frontend] layout`, if the blueprint names one.
///
/// # Errors
/// `import-invalid{file=scaffold.toml, line, cause=toml}` when the text is not TOML; `cause=type` when `layout` is not a text.
pub fn scaffold_layout(text: &str) -> Result<Option<String>, Coded<LayoutCode>> {
    let bad = |line: u32, cause: &str| {
        Coded::new(LayoutCode::ImportInvalid)
            .with("file", "scaffold.toml")
            .with("line", line)
            .with("cause", cause)
    };
    let doc = toml_edit::ImDocument::parse(text)
        .map_err(|e| bad(e.span().map_or(0, |s| line_at(text, s.start)), "toml"))?;
    let Some(item) = doc.get("frontend").and_then(|f| f.get("layout")) else {
        return Ok(None);
    };
    item.as_str()
        .map(|s| Some(s.to_owned()))
        .ok_or_else(|| bad(0, "type"))
}
