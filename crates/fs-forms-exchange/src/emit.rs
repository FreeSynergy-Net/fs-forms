//! Canonical text of a [`Tree`]: TOML (UTF-8, LF, one trailing LF, no comments, keys in the order of the tree, scalars before
//! tables) and JSON (RFC 8785: keys sorted by UTF-16 code units, no white space, integers decimal).
//!
//! Pattern: Adapter (tree to file text).

use std::fmt::Write;

use crate::tree::Tree;

fn toml_str(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                let _ = write!(out, "\\u{:04X}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// A bare key when the TOML grammar allows it, else a quoted one.
fn key(k: &str, out: &mut String) {
    if !k.is_empty()
        && k.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        out.push_str(k);
    } else {
        toml_str(k, out);
    }
}

fn scalar(t: &Tree, out: &mut String) -> bool {
    match t {
        Tree::Str(s) => toml_str(s, out),
        Tree::Int(i) => {
            let _ = write!(out, "{i}");
        }
        Tree::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Tree::Arr(a) if a.iter().all(|x| !matches!(x, Tree::Obj(..) | Tree::Arr(_))) => {
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                scalar(x, out);
            }
            out.push(']');
        }
        Tree::Obj(o, _) => {
            out.push_str("{ ");
            for (i, (k, x)) in o.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                key(k, out);
                out.push_str(" = ");
                scalar(x, out);
            }
            out.push_str(" }");
        }
        Tree::Arr(_) => return false,
    }
    true
}

fn is_tables(t: &Tree) -> bool {
    matches!(t, Tree::Arr(a) if !a.is_empty() && a.iter().all(|x| matches!(x, Tree::Obj(..))))
}

/// A table that holds a list of tables (at any depth) cannot be inline: it gets its own header.
fn holds_tables(t: &Tree) -> bool {
    match t {
        Tree::Obj(o, _) => o.iter().any(|(_, v)| is_tables(v) || holds_tables(v)),
        _ => false,
    }
}

fn table(path: &str, items: &[(String, Tree)], out: &mut String) {
    for (k, v) in items {
        if is_tables(v) || holds_tables(v) || matches!(v, Tree::Arr(a) if a.is_empty()) {
            continue;
        }
        let _ = write!(out, "{k} = ");
        scalar(v, out);
        out.push('\n');
    }
    for (k, v) in items {
        if let (true, Tree::Obj(inner, _)) = (holds_tables(v), v) {
            let here = if path.is_empty() {
                k.clone()
            } else {
                format!("{path}.{k}")
            };
            let _ = write!(out, "\n[{here}]\n");
            table(&here, inner, out);
        }
        if let (true, Tree::Arr(list)) = (is_tables(v), v) {
            for o in list {
                if let Tree::Obj(inner, _) = o {
                    let here = if path.is_empty() {
                        k.clone()
                    } else {
                        format!("{path}.{k}")
                    };
                    let _ = write!(out, "\n[[{here}]]\n");
                    table(&here, inner, out);
                }
            }
        }
    }
}

/// Canonical TOML of an object tree.
pub(crate) fn to_toml(root: &Tree) -> String {
    let mut out = String::new();
    if let Tree::Obj(items, _) = root {
        table("", items, &mut out);
    }
    format!("{}\n", out.trim_end())
}

fn json_str(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Canonical JSON (RFC 8785) of a tree, one trailing LF.
pub(crate) fn to_json(t: &Tree) -> String {
    let mut out = String::new();
    json(t, &mut out);
    out.push('\n');
    out
}

fn json(t: &Tree, out: &mut String) {
    match t {
        Tree::Str(s) => json_str(s, out),
        Tree::Int(i) => {
            let _ = write!(out, "{i}");
        }
        Tree::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Tree::Arr(a) => {
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                json(x, out);
            }
            out.push(']');
        }
        Tree::Obj(o, _) => {
            let mut sorted: Vec<&(String, Tree)> = o.iter().collect();
            sorted.sort_by(|a, b| a.0.encode_utf16().cmp(b.0.encode_utf16()));
            out.push('{');
            for (i, (k, v)) in sorted.into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                json_str(k, out);
                out.push(':');
                json(v, out);
            }
            out.push('}');
        }
    }
}
