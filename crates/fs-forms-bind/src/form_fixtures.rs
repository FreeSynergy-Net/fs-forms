//! Test doubles of the form match (feature `testing`): a reader of the DRAFT `forms.txt` and a small cards form by hand.
//!
//! The real reader will be `fs_interfaces::forms` (order SQ-1b); until it is pinned, this reads the same lines. Channel lines
//! are `#   name | Art: x | [Grundtyp: y |] Pflicht: ja (`!`) | Anzahl: z`. A `Grundtyp` of the seven words wins; the draft
//! still has older lines with `Art` only, which map: grouping -> text, order -> number, any -> text (facts show as text).
//!
//! Pattern: Test double + parser of a line format (no I/O: the text comes from the caller).

use crate::coded::Coded;
use crate::forms::{Channel, ComponentForms, Count, FormDef, FormRef, Grundtyp, ModuleForm};
use crate::types::Basic;

/// Reads the form lines and their channel comment lines of a `forms.txt`.
///
/// # Errors
/// `form-catalog-invalid{line}` for a line that cannot be read (never skipped silently).
pub fn parse_forms_txt(text: &str) -> Result<Vec<FormDef>, Coded> {
    let mut out: Vec<FormDef> = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let bad = || Coded::new("form-catalog-invalid").with("line", (n + 1).to_string());
        if let Some(rest) = line.strip_prefix('#') {
            let cols: Vec<&str> = rest.split('|').map(str::trim).collect();
            let channel = rest.starts_with("   ") && cols.iter().any(|c| c.starts_with("Pflicht:"));
            if channel {
                let def = out.last_mut().ok_or_else(bad)?;
                def.channels.push(read_channel(&cols).ok_or_else(bad)?);
            }
        } else if line.contains('|') {
            let key = line.split('|').next().unwrap_or("").trim();
            let (name, version) = key.split_once('@').ok_or_else(bad)?;
            let (major, minor) = version.split_once('.').ok_or_else(bad)?;
            out.push(FormDef {
                name: name.to_owned(),
                major: major.parse().map_err(|_| bad())?,
                minor: minor.parse().map_err(|_| bad())?,
                channels: Vec::new(),
            });
        }
    }
    Ok(out)
}

fn read_channel(cols: &[&str]) -> Option<Channel> {
    let value = |key: &str| cols.iter().find_map(|c| c.strip_prefix(key)).map(str::trim);
    let art = value("Art:")?;
    let grundtyp = value("Grundtyp:")
        .and_then(Grundtyp::from_name)
        .or_else(|| Grundtyp::from_name(art))
        .or(match art {
            "grouping" | "any" => Some(Grundtyp::Text),
            "order" => Some(Grundtyp::Number),
            _ => None,
        })?;
    let required = value("Pflicht:")?.starts_with("ja");
    let count = match value("Anzahl:").unwrap_or("offen") {
        s if s.starts_with("1 ") || s == "1" => Count::One,
        s if s.starts_with("0 oder 1") => Count::ZeroOrOne,
        s if s.starts_with("0 bis 4") => Count::ZeroToN(Some(4)),
        s if s.starts_with("0 bis n") => Count::ZeroToN(None),
        _ => Count::ZeroToN(None),
    };
    Some(Channel {
        name: (*cols.first()?).to_owned(),
        grundtyp,
        required,
        count,
    })
}

/// A form reference `name@H.N`.
#[must_use]
pub fn form_ref(name: &str, major: u32, minor: u32) -> FormRef {
    FormRef {
        name: name.to_owned(),
        major,
        minor,
    }
}

/// A module that maps `tree@1.0` completely (nodes, parent, label required; edge-kind optional left out).
#[must_use]
pub fn tree_module() -> ModuleForm {
    let map = |c: &str, f: &str| (c.to_owned(), vec![f.to_owned()]);
    ModuleForm {
        form: form_ref("tree", 1, 0),
        map: vec![
            map("nodes", "Group"),
            map("parent", "parent"),
            map("label", "name"),
        ],
        fields: vec![
            ("parent".into(), Basic::Reference),
            ("name".into(), Basic::Text),
        ],
    }
}

/// The component of the block `tree`, drawing `tree@1.0`.
#[must_use]
pub fn tree_component() -> ComponentForms {
    ComponentForms {
        block: "tree".into(),
        forms: vec![form_ref("tree", 1, 0)],
    }
}
