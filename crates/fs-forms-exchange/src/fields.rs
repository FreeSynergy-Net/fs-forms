//! Closed-schema access to one table of the tree: every key must be asked for, a key nobody asked for is an error (never ignored),
//! a wrong type or a missing key is an error with its line. All problems are collected, never only the first.
//!
//! Pattern: Builder of an error list (collecting parameter).

use fs_forms_layout::{Coded, ElemId, ErrorCode, LayoutCode};

use crate::tree::Tree;

/// File name, file text (for line numbers) and the problems found so far.
pub(crate) struct Ctx<'a> {
    pub src: &'a str,
    pub file: &'a str,
    pub errs: Vec<Coded<LayoutCode>>,
}

impl Ctx<'_> {
    /// First line (1-based) that mentions `needle`, 0 when none; a best effort, the file is the only place with lines.
    pub(crate) fn line_of(&self, needle: &str) -> u32 {
        let n = self
            .src
            .lines()
            .position(|l| l.contains(needle))
            .map_or(0, |i| i + 1);
        u32::try_from(n).unwrap_or(u32::MAX)
    }
    pub(crate) fn bad(&mut self, cause: &str, needle: &str) {
        let line = self.line_of(needle);
        self.errs.push(
            Coded::new(LayoutCode::ImportInvalid)
                .with("file", self.file)
                .with("line", line)
                .with("cause", cause),
        );
    }
}

/// One table with the keys not yet asked for.
pub(crate) struct Fields<'a> {
    items: Vec<(&'a str, &'a Tree)>,
    what: &'a str,
}

impl<'a> Fields<'a> {
    pub(crate) fn new(t: &'a Tree, what: &'a str, cx: &mut Ctx<'_>) -> Option<Self> {
        if let Tree::Obj(o) = t {
            Some(Self {
                items: o.iter().map(|(k, v)| (k.as_str(), v)).collect(),
                what,
            })
        } else {
            cx.bad("type", what);
            None
        }
    }
    fn take(&mut self, key: &str) -> Option<&'a Tree> {
        let i = self.items.iter().position(|(k, _)| *k == key)?;
        Some(self.items.remove(i).1)
    }
    pub(crate) fn str(&mut self, cx: &mut Ctx<'_>, key: &str) -> Option<String> {
        match self.take(key)? {
            Tree::Str(s) => Some(s.clone()),
            _ => {
                cx.bad("type", key);
                None
            }
        }
    }
    pub(crate) fn req_str(&mut self, cx: &mut Ctx<'_>, key: &str) -> String {
        if !self.has_list(key) {
            cx.bad("key-missing", self.what);
        }
        self.str(cx, key).unwrap_or_default()
    }
    pub(crate) fn int(&mut self, cx: &mut Ctx<'_>, key: &str) -> Option<i64> {
        match self.take(key)? {
            Tree::Int(i) => Some(*i),
            _ => {
                cx.bad("type", key);
                None
            }
        }
    }
    pub(crate) fn has_list(&self, key: &str) -> bool {
        self.items.iter().any(|(k, _)| *k == key)
    }
    pub(crate) fn uint(&mut self, cx: &mut Ctx<'_>, key: &str) -> Option<u32> {
        let v = self.int(cx, key)?;
        u32::try_from(v).map_err(|_| cx.bad("type", key)).ok()
    }
    pub(crate) fn bool(&mut self, cx: &mut Ctx<'_>, key: &str) -> Option<bool> {
        match self.take(key)? {
            Tree::Bool(b) => Some(*b),
            _ => {
                cx.bad("type", key);
                None
            }
        }
    }
    pub(crate) fn list(&mut self, cx: &mut Ctx<'_>, key: &str) -> &'a [Tree] {
        match self.take(key) {
            None => &[],
            Some(Tree::Arr(a)) => a,
            Some(_) => {
                cx.bad("type", key);
                &[]
            }
        }
    }
    /// The `elem` key: parsed id and the line of it.
    pub(crate) fn elem(&mut self, cx: &mut Ctx<'_>) -> (Option<ElemId>, u32) {
        let Some(text) = self.str(cx, "elem") else {
            return (None, 0);
        };
        let line = cx.line_of(&format!("\"{text}\""));
        match ElemId::parse(&text) {
            Ok(e) => (Some(e), line),
            Err(c) => {
                cx.errs.push(c.with("line", line));
                (None, line)
            }
        }
    }
    /// Reports every key nobody asked for.
    pub(crate) fn done(self, cx: &mut Ctx<'_>) {
        for (k, _) in self.items {
            cx.bad("key-unknown", k);
        }
    }
}

/// The message key of a code, for callers that print it.
#[must_use]
pub fn message_key(c: LayoutCode) -> String {
    c.message_key()
}
