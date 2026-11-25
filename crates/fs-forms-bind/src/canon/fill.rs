//! The canonical bytes of a `Fill` (child of `canon`, split to keep the files short).
//!
//! Pattern: Visitor-like encoder (continued from `canon`).

use super::Enc;
use crate::plan::{Fill, InputState};

impl Enc {
    pub(super) fn fill(&mut self, label: u16, f: &Fill) {
        match f {
            Fill::Records { source } => {
                self.variant(label, "records", |e| e.num(1, u64::from(*source)));
            }
            Fill::Field(ff) => self.variant(label, "field", |e| {
                e.strs(1, &ff.path);
                e.text(2, ff.role.name());
                e.text(3, ff.basic.name());
                e.strs(4, &ff.field.path);
                e.text(5, ff.field.basic.name());
                e.opt(6, ff.field.target.as_ref(), |e, t| {
                    e.0.extend_from_slice(t.as_bytes());
                });
                e.flag(7, ff.field.read_only);
                e.text(8, &ff.block);
                e.opt(9, ff.schemes.as_ref(), |e, v| e.strs(0, v));
                match &ff.input {
                    InputState::NotInput => e.variant(10, "not-input", |_| {}),
                    InputState::Open => e.variant(10, "open", |_| {}),
                    InputState::Locked(l) => e.variant(10, "locked", |e| e.lock(1, l)),
                }
            }),
            Fill::Grouping { name, heads } => self.variant(label, "grouping", |e| {
                e.text(1, name);
                // Only when set: the digest of every plan without heads stays what it was.
                e.opt(2, heads.as_ref(), |e, h| {
                    e.0.extend_from_slice(&u64::from(*h).to_be_bytes());
                });
            }),
            Fill::Order(n) => self.variant(label, "order", |e| e.text(1, n)),
            Fill::Command(c) => self.variant(label, "command", |e| e.command(1, c)),
            Fill::Condition(c) => self.variant(label, "condition", |e| {
                e.strs(1, &c.path);
                e.text(2, c.basic.name());
                e.text(3, &c.chip);
            }),
        }
    }
}
