//! A format-neutral tree: what a TOML or JSON file says, with keys in document or canonical order. Readers turn a file into
//! a [`Tree`], the model reader turns a tree into a description; writers do the opposite. Only this crate links toml/serde_json.
//!
//! Pattern: Adapter (file formats to one tree).

/// A value of the exchange tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Tree {
    Str(String),
    Int(i64),
    Bool(bool),
    Arr(Vec<Tree>),
    Obj(Vec<(String, Tree)>),
}

impl Tree {
    /// Builds a table with the given entries.
    pub(crate) fn obj(items: Vec<(&str, Tree)>) -> Tree {
        Tree::Obj(items.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
    }
    pub(crate) fn s(v: &str) -> Tree {
        Tree::Str(v.to_owned())
    }
}

/// Converts a parsed TOML value; floats and dates are not part of the format.
pub(crate) fn from_toml(v: toml::Value) -> Result<Tree, &'static str> {
    Ok(match v {
        toml::Value::String(s) => Tree::Str(s),
        toml::Value::Integer(i) => Tree::Int(i),
        toml::Value::Boolean(b) => Tree::Bool(b),
        toml::Value::Array(a) => Tree::Arr(a.into_iter().map(from_toml).collect::<Result<_, _>>()?),
        toml::Value::Table(t) => Tree::Obj(
            t.into_iter()
                .map(|(k, v)| from_toml(v).map(|v| (k, v)))
                .collect::<Result<_, _>>()?,
        ),
        toml::Value::Float(_) | toml::Value::Datetime(_) => return Err("type"),
    })
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
        ),
        J::Null => return Err("type"),
    })
}
