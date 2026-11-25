//! A format-neutral tree: what a TOML or JSON file says, with keys in document or canonical order. Readers turn a file into
//! a [`Tree`], the model reader turns a tree into a description; writers do the opposite. Only this crate links `toml_edit` and `serde_json`.
//!
//! Pattern: Adapter (file formats to one tree).

/// A value of the exchange tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Tree {
    Str(String),
    Int(i64),
    Bool(bool),
    Arr(Vec<Tree>),
    Obj(Vec<(String, Tree)>, u32),
}

impl Tree {
    /// Builds a table with the given entries.
    pub(crate) fn obj(items: Vec<(&str, Tree)>) -> Tree {
        Tree::Obj(
            items.into_iter().map(|(k, v)| (k.to_owned(), v)).collect(),
            0,
        )
    }
    pub(crate) fn s(v: &str) -> Tree {
        Tree::Str(v.to_owned())
    }
}

/// 1-based line of a byte offset.
pub(crate) fn line_at(text: &str, byte: usize) -> u32 {
    let n = text[..byte.min(text.len())]
        .bytes()
        .filter(|b| *b == b'\n')
        .count()
        + 1;
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn span_line(span: Option<std::ops::Range<usize>>, text: &str) -> u32 {
    span.map_or(0, |s| line_at(text, s.start))
}

fn from_value(v: &toml_edit::Value, text: &str) -> Result<Tree, &'static str> {
    use toml_edit::Value as V;
    Ok(match v {
        V::String(s) => Tree::Str(s.value().clone()),
        V::Integer(i) => Tree::Int(*i.value()),
        V::Boolean(b) => Tree::Bool(*b.value()),
        V::Array(a) => Tree::Arr(
            a.iter()
                .map(|x| from_value(x, text))
                .collect::<Result<_, _>>()?,
        ),
        V::InlineTable(t) => {
            let items = t
                .iter()
                .map(|(k, x)| from_value(x, text).map(|x| (k.to_owned(), x)))
                .collect::<Result<_, _>>()?;
            Tree::Obj(items, span_line(t.span(), text))
        }
        V::Float(_) | V::Datetime(_) => return Err("type"),
    })
}

fn from_table(t: &toml_edit::Table, text: &str) -> Result<Tree, &'static str> {
    let mut items = Vec::new();
    for (k, item) in t {
        items.push((k.to_owned(), from_item(item, text)?));
    }
    Ok(Tree::Obj(items, span_line(t.span(), text)))
}

/// Converts a parsed TOML item, keeping the line of every table (spans of `toml_edit` 0.22).
pub(crate) fn from_item(item: &toml_edit::Item, text: &str) -> Result<Tree, &'static str> {
    match item {
        toml_edit::Item::Value(v) => from_value(v, text),
        toml_edit::Item::Table(t) => from_table(t, text),
        toml_edit::Item::ArrayOfTables(a) => Ok(Tree::Arr(
            a.iter()
                .map(|t| from_table(t, text))
                .collect::<Result<_, _>>()?,
        )),
        toml_edit::Item::None => Err("type"),
    }
}

/// The tree of a TOML text (the text a reader gave back for a section it keeps whole).
pub(crate) fn from_toml(text: &str) -> Result<Tree, &'static str> {
    let doc = toml_edit::ImDocument::parse(text).map_err(|_| "toml")?;
    from_item(doc.as_item(), text)
}

/// Converts a parsed JSON value; floats, nulls are not part of the format.
#[cfg(feature = "json")]
pub(crate) fn from_json(v: serde_json::Value) -> Result<Tree, &'static str> {
    use serde_json::Value as J;
    Ok(match v {
        J::String(s) => Tree::Str(s),
        J::Number(n) => Tree::Int(n.as_i64().ok_or("type")?),
        J::Bool(b) => Tree::Bool(b),
        J::Array(a) => Tree::Arr(a.into_iter().map(from_json).collect::<Result<_, _>>()?),
        J::Object(o) => Tree::Obj(
            o.into_iter()
                .map(|(k, v)| from_json(v).map(|v| (k, v)))
                .collect::<Result<_, _>>()?,
            0,
        ),
        J::Null => return Err("type"),
    })
}
