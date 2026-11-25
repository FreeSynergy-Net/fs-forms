//! Closed-schema access to one table of the tree: every key must be asked for, a key nobody asked for is an error (never ignored),
//! a wrong type or a missing key is an error with its line. All problems are collected, never only the first.
//!
//! Pattern: Builder of an error list (collecting parameter).

use fs_forms_layout::{Coded, ElemId, LayoutCode};

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
    /// Line of the first line at or after `start` that mentions `needle` (a table's own line when it is not found).
    fn line_from(&self, start: u32, needle: &str) -> u32 {
        let skip = (start.max(1) - 1) as usize;
        match self.src.lines().skip(skip).position(|l| l.contains(needle)) {
            Some(i) => u32::try_from(skip + i + 1).unwrap_or(u32::MAX),
            None => start,
        }
    }
    pub(crate) fn bad_at(&mut self, start: u32, cause: &str, needle: &str) {
        let line = self.line_from(start, needle);
        self.errs.push(
            Coded::new(LayoutCode::ImportInvalid)
                .with("file", self.file)
                .with("line", line)
                .with("cause", cause),
        );
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
    line: u32,
}

impl<'a> Fields<'a> {
    pub(crate) fn new(t: &'a Tree, what: &'a str, cx: &mut Ctx<'_>) -> Option<Self> {
        if let Tree::Obj(o, line) = t {
            Some(Self {
                items: o.iter().map(|(k, v)| (k.as_str(), v)).collect(),
                line: *line,
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
        if let Tree::Str(s) = self.take(key)? {
            Some(s.clone())
        } else {
            cx.bad_at(self.line, "type", key);
            None
        }
    }
    pub(crate) fn req_str(&mut self, cx: &mut Ctx<'_>, key: &str) -> String {
        if !self.has_list(key) {
            cx.bad_at(self.line, "key-missing", "");
        }
        self.str(cx, key).unwrap_or_default()
    }
    pub(crate) fn int(&mut self, cx: &mut Ctx<'_>, key: &str) -> Option<i64> {
        if let Tree::Int(i) = self.take(key)? {
            Some(*i)
        } else {
            cx.bad_at(self.line, "type", key);
            None
        }
    }
    pub(crate) fn has_list(&self, key: &str) -> bool {
        self.items.iter().any(|(k, _)| *k == key)
    }
    pub(crate) fn uint(&mut self, cx: &mut Ctx<'_>, key: &str) -> Option<u32> {
        let v = self.int(cx, key)?;
        let line = self.line;
        u32::try_from(v)
            .map_err(|_| cx.bad_at(line, "type", key))
            .ok()
    }
    pub(crate) fn bool(&mut self, cx: &mut Ctx<'_>, key: &str) -> Option<bool> {
        if let Tree::Bool(b) = self.take(key)? {
            Some(*b)
        } else {
            cx.bad_at(self.line, "type", key);
            None
        }
    }
    /// Line of the table (0 = unknown).
    pub(crate) fn line(&self) -> u32 {
        self.line
    }
    /// A table value (key = value pairs) and its line; a wrong type is reported.
    pub(crate) fn table(
        &mut self,
        cx: &mut Ctx<'_>,
        key: &str,
    ) -> Option<(&'a [(String, Tree)], u32)> {
        if let Tree::Obj(o, line) = self.take(key)? {
            Some((o, *line))
        } else {
            cx.bad_at(self.line, "type", key);
            None
        }
    }
    /// A table value as it stands (its keys are not read here), or `None`; a value that is no table is reported.
    pub(crate) fn raw_table(&mut self, cx: &mut Ctx<'_>, key: &str) -> Option<&'a Tree> {
        let t = self.take(key)?;
        if matches!(t, Tree::Obj(..)) {
            Some(t)
        } else {
            cx.bad_at(self.line, "type", key);
            None
        }
    }
    pub(crate) fn list(&mut self, cx: &mut Ctx<'_>, key: &str) -> &'a [Tree] {
        match self.take(key) {
            None => &[],
            Some(Tree::Arr(a)) => a,
            Some(_) => {
                cx.bad_at(self.line, "type", key);
                &[]
            }
        }
    }
    /// The `elem` key: parsed id and the line of it.
    pub(crate) fn elem(&mut self, cx: &mut Ctx<'_>) -> (Option<ElemId>, u32) {
        let Some(text) = self.str(cx, "elem") else {
            return (None, self.line); // a missing id is reported with the line of its table
        };
        let line = if self.line > 0 {
            self.line
        } else {
            cx.line_of(&format!("\"{text}\""))
        };
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
            cx.bad_at(self.line, "key-unknown", k);
        }
    }
}
