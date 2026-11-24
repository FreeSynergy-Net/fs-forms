//! The canonical byte form of a plan and its SHA-256 (A-P2): a fixed encoding, independent of `Debug`, of the compiler and of
//! the declaration order of enums.
//!
//! Encoding: every field is `label (u16 BE) + length (u32 BE) + payload`. The label is the position of the field in its struct
//! (declaration order); numbers are u64 BE; text is UTF-8; `bool` is a number 0/1; `Option` is an absent field or the value
//! field; a list is its items in plan order (never sorted), each item a field. A variant is a field whose payload starts with the
//! variant NAME (text field, label 0) followed by its fields, so a reordered enum does not change the digest. The `style` of the
//! plan is NOT part of it.
//!
//! Pattern: Visitor-like encoder (one method per value kind).

use sha2::{Digest, Sha256};

use crate::coded::Coded;
use crate::codes::{Cause, Notice, PartLock, Severity};
use crate::menu::{MenuEntry, MenuNode};
use crate::model::{Range, Target};
use crate::plan::{
    CommandPlan, Fill, InputState, PagePlan, PlacePlan, PlaceStatus, Plan, PlanState, SlotFill,
    SourcePlan,
};
use crate::plan_tree::{AttrPlan, NodePlan};
use crate::types::SearchKind;

struct Enc(Vec<u8>);

impl Enc {
    fn field(&mut self, label: u16, body: impl FnOnce(&mut Enc)) {
        let mut inner = Enc(Vec::new());
        body(&mut inner);
        self.0.extend_from_slice(&label.to_be_bytes());
        // A field above 4 GiB saturates its length: unreachable, the host limits (`Limits`) are far below it.
        let len = u32::try_from(inner.0.len()).unwrap_or(u32::MAX);
        self.0.extend_from_slice(&len.to_be_bytes());
        self.0.extend_from_slice(&inner.0);
    }
    fn text(&mut self, label: u16, t: &str) {
        self.field(label, |e| e.0.extend_from_slice(t.as_bytes()));
    }
    fn num(&mut self, label: u16, n: u64) {
        self.field(label, |e| e.0.extend_from_slice(&n.to_be_bytes()));
    }
    fn flag(&mut self, label: u16, v: bool) {
        self.num(label, u64::from(v));
    }
    fn opt<T>(&mut self, label: u16, v: Option<&T>, f: impl FnOnce(&mut Enc, &T)) {
        if let Some(x) = v {
            self.field(label, |e| f(e, x));
        }
    }
    fn list<T>(&mut self, label: u16, v: &[T], mut f: impl FnMut(&mut Enc, &T)) {
        self.field(label, |e| {
            for x in v {
                e.field(0, |e| f(e, x));
            }
        });
    }
    fn strs(&mut self, label: u16, v: &[String]) {
        self.list(label, v, |e, x| e.0.extend_from_slice(x.as_bytes()));
    }
    fn variant(&mut self, label: u16, name: &str, body: impl FnOnce(&mut Enc)) {
        self.field(label, |e| {
            e.text(0, name);
            body(e);
        });
    }

    fn coded(&mut self, label: u16, c: &Coded) {
        self.field(label, |e| {
            e.text(0, &c.code);
            e.list(1, &c.params, |e, (k, v)| {
                e.text(0, k);
                e.text(1, v);
            });
        });
    }
    fn cause(&mut self, label: u16, c: &Cause) {
        let name = match c {
            Cause::Accepts(_) => "accepts",
            Cause::Layout(_) => "layout",
            Cause::Records(_) => "records",
            Cause::Bind(_) => "bind",
            Cause::Unknown(_) => "unknown",
        };
        self.variant(label, name, |e| e.coded(1, c.coded()));
    }
    fn lock(&mut self, label: u16, l: &PartLock) {
        self.field(label, |e| {
            e.text(0, &l.part.text());
            e.text(
                1,
                match l.severity {
                    Severity::Locked => "locked",
                    Severity::Coming => "coming",
                },
            );
            e.cause(2, &l.cause);
        });
    }
    fn locks(&mut self, label: u16, v: &[PartLock]) {
        self.list(label, v, |e, l| e.lock(0, l));
    }
    fn range(&mut self, label: u16, r: &Range) {
        self.field(label, |e| {
            for (i, n) in [r.min.0, r.min.1, r.min.2, r.max.0, r.max.1, r.max.2]
                .into_iter()
                .enumerate()
            {
                e.num(u16::try_from(i).unwrap_or(0), u64::from(n));
            }
        });
    }

    fn source(&mut self, s: &SourcePlan) {
        self.num(0, u64::from(s.idx));
        self.text(1, &s.elem);
        self.range(2, &s.range);
        self.text(3, &s.data_type);
        match &s.status {
            Ok(r) => self.variant(4, "ready", |e| {
                e.text(1, &r.capability);
                e.num(2, u64::from(r.version.0));
                e.num(3, u64::from(r.version.1));
                e.num(4, u64::from(r.version.2));
                e.text(
                    5,
                    match r.search {
                        SearchKind::Prefix => "prefix",
                        SearchKind::Words => "words",
                    },
                );
            }),
            Err(l) => self.variant(4, "locked", |e| e.lock(1, l)),
        }
    }

    fn command(&mut self, label: u16, c: &CommandPlan) {
        self.field(label, |e| {
            e.num(0, u64::from(c.source));
            e.text(1, &c.name);
            e.flag(2, c.description.supported);
            e.flag(3, c.description.writes);
            e.flag(4, c.description.on_record);
            e.flag(5, c.description.type_may);
            e.flag(6, c.on_record);
            e.opt(7, c.fixed.as_ref(), |e, l| e.lock(0, l));
            match &c.target {
                Target::Program => e.variant(8, "program", |_| {}),
                Target::Workflow(w) => e.variant(8, "workflow", |e| e.text(1, w)),
            }
        });
    }

    fn fill(&mut self, label: u16, f: &Fill) {
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
            Fill::Grouping(n) => self.variant(label, "grouping", |e| e.text(1, n)),
            Fill::Order(n) => self.variant(label, "order", |e| e.text(1, n)),
            Fill::Command(c) => self.variant(label, "command", |e| e.command(1, c)),
            Fill::Condition(c) => self.variant(label, "condition", |e| {
                e.strs(1, &c.path);
                e.text(2, c.basic.name());
                e.text(3, &c.chip);
            }),
        }
    }

    fn slot(&mut self, s: &SlotFill) {
        self.text(0, &s.slot);
        self.num(1, u64::from(s.nth));
        self.text(2, &s.elem);
        self.fill(3, &s.fill);
        self.opt(4, s.lock.as_ref(), |e, l| e.lock(0, l));
    }

    fn page(&mut self, p: &PagePlan) {
        self.text(0, &p.page);
        self.text(1, &p.elem);
        self.num(2, u64::from(p.seq));
        self.text(3, &p.frame);
        self.locks(4, &p.locks);
        self.list(5, &p.places, Self::node_plan);
    }

    fn attr(&mut self, a: &AttrPlan) {
        self.text(0, &a.name);
        self.text(1, &a.value);
    }

    fn place(&mut self, pl: &PlacePlan) {
        self.text(0, &pl.place);
        self.text(1, &pl.elem);
        self.num(2, u64::from(pl.seq));
        self.text(3, &pl.block);
        self.opt(4, pl.source.as_ref(), |e, s| {
            e.0.extend_from_slice(&u64::from(*s).to_be_bytes());
        });
        match &pl.status {
            PlaceStatus::Ready(r) => self.variant(5, "ready", |e| e.list(1, &r.slots, Self::slot)),
            PlaceStatus::Locked(v) => self.variant(5, "locked", |e| e.locks(1, v)),
        }
        self.list(6, &pl.set, Self::attr);
    }

    fn node_plan(&mut self, n: &NodePlan) {
        match n {
            NodePlan::Block(pl) => self.variant(0, "block", |e| e.place(pl)),
            NodePlan::Frame(f) => self.variant(0, "frame", |e| {
                e.text(1, &f.place);
                e.text(2, &f.elem);
                e.num(3, u64::from(f.seq));
                e.text(4, &f.frame);
                e.list(5, &f.set, Self::attr);
                e.locks(6, &f.locks);
                e.list(7, &f.children, Self::node_plan);
            }),
        }
    }

    fn node(&mut self, n: &MenuNode) {
        self.text(0, &n.elem);
        match &n.entry {
            MenuEntry::Menu { label_key } => self.variant(1, "menu", |e| e.text(1, label_key)),
            MenuEntry::Standard { which } => self.variant(1, "standard", |e| e.text(1, which)),
            MenuEntry::Command {
                command,
                on_selected,
                shortcut,
            } => self.variant(1, "command", |e| {
                e.command(1, command);
                e.flag(2, *on_selected);
                e.opt(3, shortcut.as_ref(), |e, s| {
                    e.0.extend_from_slice(s.aria().as_bytes());
                });
            }),
            MenuEntry::View { view } => self.variant(1, "view", |e| e.text(1, view)),
            MenuEntry::Separator => self.variant(1, "separator", |_| {}),
        }
        self.list(2, &n.children, Self::node);
    }

    fn notice(&mut self, n: &Notice) {
        self.cause(0, &n.cause);
    }
}

impl Plan {
    /// The canonical bytes of the plan WITHOUT `style` (see the module doc).
    #[must_use]
    pub fn canonical_bytes_without_style(&self) -> Vec<u8> {
        let mut e = Enc(Vec::new());
        e.text(0, &self.key);
        e.num(1, self.stamp);
        e.num(2, self.rev);
        e.list(3, &self.sources, Enc::source);
        e.list(4, &self.pages, Enc::page);
        e.list(5, &self.menu.roots, Enc::node);
        e.locks(6, &self.menu.locks);
        e.strs(7, &self.keys.reserved);
        e.list(8, &self.notices, Enc::notice);
        e.text(
            9,
            match self.state {
                PlanState::Ready => "ready",
                PlanState::Partial => "partial",
                PlanState::Refused => "refused",
            },
        );
        e.0
    }

    /// SHA-256 over [`Plan::canonical_bytes_without_style`]: equal for every style (rule `plan.style-blind`, M8).
    #[must_use]
    pub fn digest_without_style(&self) -> [u8; 32] {
        Sha256::digest(self.canonical_bytes_without_style()).into()
    }
}
