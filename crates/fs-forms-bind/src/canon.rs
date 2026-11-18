//! The canonical byte form of a plan and its SHA-256 (A-P2): a fixed encoding, independent of `Debug` and of the compiler.
//!
//! Encoding: every value is a tag byte or a length-prefixed field; integers are big-endian; `Option` is `0` or `1` + value; a
//! sequence is its count (u64) + items; text is its byte length (u64) + UTF-8 bytes. The `style` of the plan is NOT part of it.
//!
//! Pattern: Visitor-like encoder (one method per value kind).

use sha2::{Digest, Sha256};

use crate::coded::Coded;
use crate::codes::{Cause, Notice, Part, PartLock};
use crate::menu::{MenuEntry, MenuNode};
use crate::model::{Range, Target};
use crate::plan::{
    CommandPlan, Fill, InputState, PagePlan, PlaceStatus, Plan, SlotFill, SourcePlan,
};

struct Enc(Vec<u8>);

impl Enc {
    fn u(&mut self, n: u64) {
        self.0.extend_from_slice(&n.to_be_bytes());
    }
    fn tag(&mut self, t: u8) {
        self.0.push(t);
    }
    fn s(&mut self, t: &str) {
        self.u(t.len() as u64);
        self.0.extend_from_slice(t.as_bytes());
    }
    fn b(&mut self, v: bool) {
        self.tag(u8::from(v));
    }
    fn opt<T>(&mut self, v: Option<&T>, f: impl FnOnce(&mut Self, &T)) {
        match v {
            Some(x) => {
                self.tag(1);
                f(self, x);
            }
            None => self.tag(0),
        }
    }
    fn seq<T>(&mut self, v: &[T], mut f: impl FnMut(&mut Self, &T)) {
        self.u(v.len() as u64);
        for x in v {
            f(self, x);
        }
    }
    fn strs(&mut self, v: &[String]) {
        self.seq(v, |e, x| e.s(x));
    }

    fn coded(&mut self, c: &Coded) {
        self.s(&c.code);
        self.seq(&c.params, |e, (k, v)| {
            e.s(k);
            e.s(v);
        });
    }
    fn cause(&mut self, c: &Cause) {
        self.tag(match c {
            Cause::Accepts(_) => 0,
            Cause::Layout(_) => 1,
            Cause::Records(_) => 2,
            Cause::Bind(_) => 3,
        });
        self.coded(c.coded());
    }
    fn part(&mut self, p: &Part) {
        self.s(&p.text());
    }
    fn lock(&mut self, l: &PartLock) {
        self.part(&l.part);
        self.tag(l.severity as u8);
        self.cause(&l.cause);
    }
    fn locks(&mut self, v: &[PartLock]) {
        self.seq(v, Self::lock);
    }
    fn range(&mut self, r: &Range) {
        for n in [r.min.0, r.min.1, r.max.0, r.max.1] {
            self.u(u64::from(n));
        }
    }

    fn source(&mut self, s: &SourcePlan) {
        self.u(u64::from(s.idx));
        self.s(&s.elem);
        self.range(&s.range);
        self.s(&s.data_type);
        match &s.status {
            Ok(r) => {
                self.tag(1);
                self.s(&r.capability);
                self.u(u64::from(r.version.0));
                self.u(u64::from(r.version.1));
                self.tag(r.search as u8);
            }
            Err(l) => {
                self.tag(0);
                self.lock(l);
            }
        }
    }

    fn command(&mut self, c: &CommandPlan) {
        self.u(u64::from(c.source));
        self.s(&c.name);
        self.b(c.description.supported);
        self.b(c.description.writes);
        self.b(c.description.on_record);
        self.opt(c.description.type_may.as_ref(), |e, v| e.b(*v));
        self.b(c.on_record);
        self.opt(c.fixed.as_ref(), Self::lock);
        match &c.target {
            Target::Program => self.tag(0),
            Target::Workflow(w) => {
                self.tag(1);
                self.s(w);
            }
        }
    }

    fn fill(&mut self, f: &Fill) {
        match f {
            Fill::Records { source } => {
                self.tag(0);
                self.u(u64::from(*source));
            }
            Fill::Field(ff) => {
                self.tag(1);
                self.strs(&ff.path);
                self.tag(ff.role as u8);
                self.tag(ff.basic as u8);
                self.strs(&ff.field.path);
                self.tag(ff.field.basic as u8);
                self.opt(ff.field.target.as_ref(), |e, t| e.s(t));
                self.b(ff.field.read_only);
                self.s(&ff.block);
                self.opt(ff.schemes.as_ref(), |e, v| e.strs(v));
                match &ff.input {
                    InputState::NotInput => self.tag(0),
                    InputState::Open => self.tag(1),
                    InputState::Locked(l) => {
                        self.tag(2);
                        self.lock(l);
                    }
                }
            }
            Fill::Grouping(n) => {
                self.tag(2);
                self.s(n);
            }
            Fill::Order(n) => {
                self.tag(3);
                self.s(n);
            }
            Fill::Command(c) => {
                self.tag(4);
                self.command(c);
            }
            Fill::Condition(c) => {
                self.tag(5);
                self.strs(&c.path);
                self.tag(c.basic as u8);
                self.s(&c.chip);
            }
        }
    }

    fn slot(&mut self, s: &SlotFill) {
        self.s(&s.slot);
        self.u(u64::from(s.nth));
        self.s(&s.elem);
        self.fill(&s.fill);
        self.opt(s.lock.as_ref(), Self::lock);
    }

    fn page(&mut self, p: &PagePlan) {
        self.s(&p.page);
        self.s(&p.elem);
        self.u(u64::from(p.seq));
        self.s(&p.frame);
        self.locks(&p.locks);
        self.seq(&p.places, |e, pl| {
            e.s(&pl.place);
            e.s(&pl.elem);
            e.u(u64::from(pl.seq));
            e.s(&pl.block);
            e.opt(pl.source.as_ref(), |e, s| e.u(u64::from(*s)));
            match &pl.status {
                PlaceStatus::Ready(r) => {
                    e.tag(1);
                    e.seq(&r.slots, Self::slot);
                }
                PlaceStatus::Locked(v) => {
                    e.tag(0);
                    e.locks(v);
                }
            }
        });
    }

    fn node(&mut self, n: &MenuNode) {
        self.s(&n.elem);
        match &n.entry {
            MenuEntry::Menu { label_key } => {
                self.tag(0);
                self.s(label_key);
            }
            MenuEntry::Standard { which } => {
                self.tag(1);
                self.s(which);
            }
            MenuEntry::Command {
                command,
                on_selected,
                shortcut,
            } => {
                self.tag(2);
                self.command(command);
                self.b(*on_selected);
                self.opt(shortcut.as_ref(), |e, s| e.s(&s.aria()));
            }
            MenuEntry::View { view } => {
                self.tag(3);
                self.s(view);
            }
            MenuEntry::Separator => self.tag(4),
        }
        self.seq(&n.children, Self::node);
    }

    fn notice(&mut self, n: &Notice) {
        self.cause(&n.cause);
    }
}

impl Plan {
    /// The canonical bytes of the plan WITHOUT `style` (see the module doc).
    #[must_use]
    pub fn canonical_bytes_without_style(&self) -> Vec<u8> {
        let mut e = Enc(Vec::new());
        e.s(&self.key);
        e.u(self.stamp);
        e.u(self.rev);
        e.seq(&self.sources, Enc::source);
        e.seq(&self.pages, Enc::page);
        e.seq(&self.menu.roots, Enc::node);
        e.locks(&self.menu.locks);
        e.strs(&self.keys.reserved);
        e.seq(&self.notices, Enc::notice);
        e.tag(self.state as u8);
        e.0
    }

    /// SHA-256 over [`Plan::canonical_bytes_without_style`]: equal for every style (rule `plan.style-blind`, M8).
    #[must_use]
    pub fn digest_without_style(&self) -> [u8; 32] {
        Sha256::digest(self.canonical_bytes_without_style()).into()
    }
}
