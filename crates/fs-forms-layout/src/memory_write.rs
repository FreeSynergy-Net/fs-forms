//! `LayoutWrite` and `PackageLayouts` of the memory store. Refusals of the description are a [`Report`] with `problems`;
//! refusals of the call (exists, package-owned, rev-changed, unknown) are errors. Either way nothing is changed.
//!
//! Pattern: Unit of work (see `memory.rs`).

use crate::apply::apply;
use crate::codes::LayoutCode;
use crate::doc::LayoutDoc;
use crate::ids::{FrontendKey, LayoutId, PackageRef, Rev, Who};
use crate::layout::Layout;
use crate::memory::{MemoryLayoutStore, Res, State, Stored, ready, rev_changed, unknown};
use crate::overlay::{Conflict, OverlayState, Value};
use crate::port::{LayoutWrite, PackageLayouts, Pending};
use crate::shim::{Coded, Utc};
use crate::types::{LogAction, Mode, Origin, Report};

fn exists(key: &FrontendKey) -> Coded<LayoutCode> {
    Coded::new(LayoutCode::Exists)
        .with("extends", &key.extends)
        .with("frontend", &key.frontend)
}
fn refused(problems: Vec<Coded<LayoutCode>>) -> Res<(Report, bool)> {
    Ok((
        Report {
            problems,
            ..Report::default()
        },
        false,
    ))
}

impl MemoryLayoutStore {
    fn checked(&self, doc: LayoutDoc) -> Result<Layout, Vec<Coded<LayoutCode>>> {
        doc.check(&*self.catalog, &self.limits)
    }

    fn put(
        &self,
        doc: LayoutDoc,
        origin: Origin,
        mode: Mode,
        who: &Who,
        action: LogAction,
    ) -> Res<Report> {
        let key = doc.key.clone();
        self.tx_for(who, mode == Mode::Apply, |st, now| {
            let layout = match self.checked(doc) {
                Ok(l) => l,
                Err(p) => return refused(p),
            };
            if st.live_key(&key).is_some() {
                return Err(exists(&key));
            }
            let id = st.new_layout_id();
            let row = Stored {
                id: id.clone(),
                key,
                origin,
                package: None,
                owner: Some(who.author.clone()),
                rev: Rev(1),
                updated: now,
                deleted: None,
                layout: Some(layout),
            };
            st.layouts.push(row);
            let e = st.log(now, who, action);
            e.layout = Some(id.clone());
            e.rev_after = Some(Rev(1));
            Ok((
                Report {
                    layout: Some(id),
                    rev: Some(Rev(1)),
                    ..Report::default()
                },
                true,
            ))
        })
    }

    /// The row of a layout the person may change: live, not a package original, `expected` equal to its `rev`.
    fn own_row(st: &State, id: &LayoutId, expected: Rev) -> Res<usize> {
        let i = st.live(id).ok_or_else(|| unknown("layout", &id.0.0))?;
        let s = &st.layouts[i];
        if s.origin == Origin::Package {
            return Err(Coded::new(LayoutCode::PackageOwned).with("id", &id.0.0));
        }
        if s.rev != expected {
            return Err(rev_changed(&id.0.0, expected, s.rev));
        }
        Ok(i)
    }

    /// Applies every overlay of a layout again after its original changed; writes conflicts, never drops one.
    fn rebase(st: &mut State, idx: usize, base: &PackageRef, now: Utc) -> Vec<Conflict> {
        let id = st.layouts[idx].id.clone();
        let Some(orig) = st.layouts[idx].layout.clone() else {
            return Vec::new();
        };
        let locks = st.locks_of(&id);
        let mut found = Vec::new();
        for o in st
            .overlays
            .iter_mut()
            .filter(|o| o.layout == id && o.state != OverlayState::Off)
        {
            o.state = OverlayState::Applied;
            o.base = base.clone();
            let applied = apply(&orig, std::slice::from_ref(&*o), &locks, &base.version);
            for e in applied.effects {
                if let Err(cause) = e.result {
                    found.push(Conflict {
                        overlay: e.overlay,
                        seq: e.seq,
                        base_version: base.version.clone(),
                        cause,
                        found: Value::None,
                        at: now,
                        open: true,
                    });
                    o.state = OverlayState::Conflict;
                }
            }
        }
        let fresh = |c: &&Conflict| {
            !st.conflicts.iter().any(|x| {
                x.overlay == c.overlay && x.seq == c.seq && x.base_version == c.base_version
            })
        };
        let add: Vec<Conflict> = found.iter().filter(fresh).cloned().collect();
        st.conflicts.extend(add.clone());
        add
    }
}

impl LayoutWrite for MemoryLayoutStore {
    fn create<'a>(&'a self, doc: LayoutDoc, who: &'a Who) -> Pending<'a, Report> {
        ready(self.put(doc, Origin::Person, Mode::Apply, who, LogAction::Create))
    }
    fn import<'a>(&'a self, doc: LayoutDoc, mode: Mode, who: &'a Who) -> Pending<'a, Report> {
        ready(self.put(doc, Origin::Import, mode, who, LogAction::Import))
    }
    fn save<'a>(
        &'a self,
        id: &'a LayoutId,
        expected: Rev,
        doc: LayoutDoc,
        who: &'a Who,
    ) -> Pending<'a, Report> {
        ready(self.tx_for(who, true, |st, now| {
            let i = Self::own_row(st, id, expected)?;
            let key = doc.key.clone();
            let layout = match self.checked(doc) {
                Ok(l) => l,
                Err(p) => return refused(p),
            };
            if st.live_key(&key).is_some_and(|j| j != i) {
                return Err(exists(&key));
            }
            let s = &mut st.layouts[i];
            let before = s.rev;
            (s.key, s.layout, s.rev, s.updated) = (key, Some(layout), Rev(before.0 + 1), now);
            let after = s.rev;
            let e = st.log(now, who, LogAction::Save);
            (e.layout, e.rev_before, e.rev_after) = (Some(id.clone()), Some(before), Some(after));
            Ok((
                Report {
                    layout: Some(id.clone()),
                    rev: Some(after),
                    ..Report::default()
                },
                true,
            ))
        }))
    }
    fn delete<'a>(&'a self, id: &'a LayoutId, expected: Rev, who: &'a Who) -> Pending<'a, ()> {
        ready(self.tx_for(who, true, |st, now| {
            let i = Self::own_row(st, id, expected)?;
            let s = &mut st.layouts[i];
            (s.layout, s.deleted, s.updated, s.rev) = (None, Some(now), now, Rev(expected.0 + 1));
            let e = st.log(now, who, LogAction::Delete);
            (e.layout, e.rev_before, e.rev_after) =
                (Some(id.clone()), Some(expected), Some(Rev(expected.0 + 1)));
            Ok(((), true))
        }))
    }
}

impl PackageLayouts for MemoryLayoutStore {
    fn install<'a>(
        &'a self,
        doc: LayoutDoc,
        package: &'a PackageRef,
        mode: Mode,
        who: &'a Who,
    ) -> Pending<'a, Report> {
        let key = doc.key.clone();
        ready(self.tx(mode == Mode::Apply, |st, now| {
            let layout = match self.checked(doc) {
                Ok(l) => l,
                Err(p) => return refused(p),
            };
            let same_pkg = |s: &Stored| {
                s.origin == Origin::Package
                    && s.package
                        .as_ref()
                        .is_some_and(|p| p.package == package.package)
            };
            let (idx, action) = match st.live_key(&key) {
                Some(i) if same_pkg(&st.layouts[i]) => (i, LogAction::Update),
                Some(_) => return Err(exists(&key)),
                None => match st
                    .layouts
                    .iter()
                    .position(|s| s.deleted.is_some() && s.key == key && same_pkg(s))
                {
                    Some(i) => (i, LogAction::Install), // revive the tombstone: same id, overlays attach again
                    None => {
                        let id = st.new_layout_id();
                        let row = Stored {
                            id,
                            key,
                            origin: Origin::Package,
                            package: None,
                            owner: None,
                            rev: Rev(0),
                            updated: now,
                            deleted: None,
                            layout: None,
                        };
                        st.layouts.push(row);
                        (st.layouts.len() - 1, LogAction::Install)
                    }
                },
            };
            let s = &mut st.layouts[idx];
            (s.layout, s.package, s.deleted, s.updated, s.rev) = (
                Some(layout),
                Some(package.clone()),
                None,
                now,
                Rev(s.rev.0 + 1),
            );
            let (id, rev) = (s.id.clone(), s.rev);
            let conflicts = Self::rebase(st, idx, package, now);
            let e = st.log(now, who, action);
            (e.layout, e.rev_after, e.package) =
                (Some(id.clone()), Some(rev), Some(package.clone()));
            Ok((
                Report {
                    layout: Some(id),
                    rev: Some(rev),
                    conflicts,
                    ..Report::default()
                },
                true,
            ))
        }))
    }

    fn uninstall<'a>(&'a self, package: &'a str, who: &'a Who) -> Pending<'a, Vec<Report>> {
        ready(self.tx(true, |st, now| {
            let hit = |s: &Stored| {
                s.deleted.is_none()
                    && s.origin == Origin::Package
                    && s.package.as_ref().is_some_and(|p| p.package == package)
            };
            let rows: Vec<usize> = st
                .layouts
                .iter()
                .enumerate()
                .filter(|(_, s)| hit(s))
                .map(|(i, _)| i)
                .collect();
            let mut reports = Vec::new();
            for i in rows {
                let s = &mut st.layouts[i];
                (s.layout, s.deleted, s.updated, s.rev) = (None, Some(now), now, Rev(s.rev.0 + 1));
                let (id, rev, pkg) = (s.id.clone(), s.rev, s.package.clone());
                st.overlays
                    .iter_mut()
                    .filter(|o| o.layout == id)
                    .for_each(|o| o.state = OverlayState::Orphaned);
                let e = st.log(now, who, LogAction::Uninstall);
                (e.layout, e.rev_after, e.package) = (Some(id.clone()), Some(rev), pkg);
                reports.push(Report {
                    layout: Some(id),
                    rev: Some(rev),
                    ..Report::default()
                });
            }
            let changed = !reports.is_empty();
            Ok((reports, changed))
        }))
    }
}
