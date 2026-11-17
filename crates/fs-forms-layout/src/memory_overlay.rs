//! `OverlayWrite` of the memory store. Ops are checked against the CURRENT original; `expect` is filled here from the original,
//! never trusted from the caller (D13). Conflicts are written, never deleted automatically.
//!
//! Pattern: Unit of work (see `memory.rs`); Command (the stored ops).

use crate::apply::apply;
use crate::codes::LayoutCode;
use crate::elems::{find, get};
use crate::ids::{ElemId, LayoutId, OverlayId, PackageRef, Rev, Who};
use crate::memory::{MemoryLayoutStore, Res, State, ready, rev_changed, unknown};
use crate::overlay::{
    Conflict, Op, OpKind, Overlay, OverlayScope, OverlayState, Resolution, Value,
};
use crate::port::{OverlayWrite, Pending};
use crate::shim::{Coded, Utc};
use crate::types::{LogAction, Mode, Origin, OverlayDoc, Report};

impl MemoryLayoutStore {
    /// Creates an overlay on a live package original. `scope` is the person (default) or the instance (admin port).
    pub(crate) fn create_in(
        &self,
        st: &mut State,
        now: Utc,
        layout: &LayoutId,
        scope: OverlayScope,
        lang: Option<&str>,
        title: Option<&str>,
        who: &Who,
    ) -> Res<OverlayId> {
        let i = st
            .live(layout)
            .ok_or_else(|| unknown("layout", &layout.0.0))?;
        let s = &st.layouts[i];
        if s.origin != Origin::Package {
            return Err(Coded::new(LayoutCode::NotOverlayable).with("id", &layout.0.0));
        }
        let base = s.package.clone().unwrap_or(PackageRef {
            package: String::new(),
            version: String::new(),
        });
        let live = st.overlays.iter().filter(|o| &o.layout == layout).count();
        if live >= self.limits.overlays_max as usize {
            return Err(Coded::new(LayoutCode::TooMany)
                .with("what", "overlays")
                .with("max", self.limits.overlays_max)
                .with("by", self.limits.by.overlays.as_str()));
        }
        let id = st.new_overlay_id();
        st.overlays.push(Overlay {
            id: id.clone(),
            layout: layout.clone(),
            scope,
            title_key: title.map(str::to_owned),
            lang: lang.map(str::to_owned),
            who: who.clone(),
            base,
            state: OverlayState::Applied,
            rev: Rev(1),
            ops: Vec::new(),
        });
        let e = st.log(now, who, LogAction::OverlayCreate);
        (e.layout, e.overlay) = (Some(layout.clone()), Some(id.clone()));
        Ok(id)
    }

    /// Replaces the ops of an overlay after checking each against the original. Problems: nothing changes (`ok = false`).
    pub(crate) fn ops_in(
        &self,
        st: &mut State,
        now: Utc,
        id: &OverlayId,
        expected: Rev,
        ops: Vec<OpKind>,
        who: &Who,
    ) -> Res<(Report, bool)> {
        let oi = st
            .overlays
            .iter()
            .position(|o| &o.id == id && self.visible(o))
            .ok_or_else(|| unknown("overlay", &id.0.0))?;
        let o = st.overlays[oi].clone();
        if o.state == OverlayState::Orphaned {
            return Err(Coded::new(LayoutCode::Orphaned).with("overlay", &id.0.0));
        }
        if o.rev != expected {
            return Err(rev_changed(&id.0.0, expected, o.rev));
        }
        if ops.len() > self.limits.ops_max as usize {
            let c = Coded::new(LayoutCode::TooMany)
                .with("what", "ops")
                .with("max", self.limits.ops_max)
                .with("by", self.limits.by.ops.as_str());
            return Ok((
                Report {
                    problems: vec![c],
                    ..Report::default()
                },
                false,
            ));
        }
        let li = st
            .live(&o.layout)
            .ok_or_else(|| unknown("layout", &o.layout.0.0))?;
        let mut original = st.layouts[li]
            .layout
            .clone()
            .ok_or_else(|| unknown("layout", &o.layout.0.0))?;
        let locks = st.locks_of(&o.layout);
        let person = matches!(o.scope, OverlayScope::Person(_));
        let (mut problems, mut stored) = (Vec::new(), Vec::new());
        for (n, kind) in ops.into_iter().enumerate() {
            let seq = u32::try_from(n).unwrap_or(u32::MAX);
            match check_op(original.doc_mut(), kind, person, &locks, id, seq) {
                Ok(k) => stored.push(Op {
                    seq,
                    kind: k,
                    who: who.clone(),
                    at: now,
                }),
                Err(p) => problems.push(p),
            }
        }
        if !problems.is_empty() {
            return Ok((
                Report {
                    problems,
                    ..Report::default()
                },
                false,
            ));
        }
        let rev = Rev(o.rev.0 + 1);
        let version = st.overlays[oi].base.version.clone();
        let mut updated = Overlay {
            ops: stored,
            rev,
            ..o
        };
        let applied = apply(&original, std::slice::from_ref(&updated), &locks, &version);
        let fresh: Vec<Conflict> = applied
            .effects
            .into_iter()
            .filter_map(|e| {
                e.result.err().map(|cause| Conflict {
                    overlay: id.clone(),
                    seq: e.seq,
                    base_version: version.clone(),
                    cause,
                    found: Value::None,
                    at: now,
                    open: true,
                })
            })
            .collect();
        updated.state = if fresh.is_empty() {
            OverlayState::Applied
        } else {
            OverlayState::Conflict
        };
        let layout_id = updated.layout.clone();
        st.overlays[oi] = updated;
        st.conflicts.retain(|c| &c.overlay != id);
        st.conflicts.extend(fresh.clone());
        let e = st.log(now, who, LogAction::OverlayOps);
        (e.overlay, e.rev_after, e.count) = (Some(id.clone()), Some(rev), None);
        Ok((
            Report {
                layout: Some(layout_id),
                rev: Some(rev),
                conflicts: fresh,
                ..Report::default()
            },
            true,
        ))
    }
}

/// Checks one op against the working copy of the original; fills `expect` from the original (D13).
pub(crate) fn check_op(
    doc: &mut crate::doc::LayoutDoc,
    kind: OpKind,
    person: bool,
    locks: &[crate::overlay::ElemLock],
    id: &OverlayId,
    seq: u32,
) -> Result<OpKind, Coded<LayoutCode>> {
    let bad = |cause: &str| {
        Coded::new(LayoutCode::OpInvalid)
            .with("overlay", &id.0.0)
            .with("seq", seq)
            .with("cause", cause)
    };
    let target = match &kind {
        OpKind::Set { target, .. } | OpKind::Remove { target } | OpKind::Move { target, .. } => {
            target.clone()
        }
        OpKind::Insert { .. } => return Ok(kind), // not checked or applied yet in this cut
    };
    let node = find(doc, &target).ok_or_else(|| {
        Coded::new(LayoutCode::ElemUnknown)
            .with("elem", target.elem.as_str())
            .with("kind", target.kind.as_str())
    })?;
    if let Some(l) = locks
        .iter()
        .find(|l| person && l.target.as_ref().is_none_or(|t| *t == target))
    {
        return Err(Coded::new(LayoutCode::Locked)
            .with("elem", target.elem.as_str())
            .with("reason", &l.reason)
            .with("by", &l.locked_by.0));
    }
    match kind {
        OpKind::Set {
            target,
            field,
            value,
            expect,
        } => {
            if !field.applies_to(target.kind) {
                return Err(bad("field-unknown"));
            }
            let expect = get(&node, field).unwrap_or(expect);
            Ok(OpKind::Set {
                target,
                field,
                value,
                expect,
            })
        }
        other => Ok(other),
    }
}
