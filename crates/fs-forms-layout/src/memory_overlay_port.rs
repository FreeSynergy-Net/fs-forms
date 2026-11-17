//! `OverlayWrite` of the memory store: the port methods on top of the helpers of `memory_overlay`.
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

impl OverlayWrite for MemoryLayoutStore {
    fn create_overlay<'a>(
        &'a self,
        layout: &'a LayoutId,
        lang: Option<&'a str>,
        title_key: Option<&'a str>,
        who: &'a Who,
    ) -> Pending<'a, OverlayId> {
        let scope = OverlayScope::Person(who.author.clone());
        ready(self.tx(true, |st, now| {
            Ok((
                self.create_in(st, now, layout, scope, lang, title_key, who)?,
                true,
            ))
        }))
    }
    fn replace_ops<'a>(
        &'a self,
        id: &'a OverlayId,
        expected: Rev,
        ops: Vec<OpKind>,
        who: &'a Who,
    ) -> Pending<'a, Report> {
        let mut report = None;
        let r = self.tx(true, |st, now| {
            let (r, ok) = self.ops_in(st, now, id, expected, ops, who)?;
            report = Some(r);
            Ok(((), ok))
        });
        ready(r.map(|()| report.unwrap_or_default()))
    }
    fn set_state<'a>(
        &'a self,
        id: &'a OverlayId,
        expected: Rev,
        on: bool,
        who: &'a Who,
    ) -> Pending<'a, Rev> {
        ready(self.tx(true, |st, now| {
            let open = st.conflicts.iter().any(|c| &c.overlay == id && c.open);
            let o = st
                .overlays
                .iter_mut()
                .find(|o| &o.id == id)
                .ok_or_else(|| unknown("overlay", &id.0.0))?;
            if o.state == OverlayState::Orphaned {
                return Err(Coded::new(LayoutCode::Orphaned).with("overlay", &id.0.0));
            }
            if o.rev != expected {
                return Err(rev_changed(&id.0.0, expected, o.rev));
            }
            o.state = match (on, open) {
                (false, _) => OverlayState::Off,
                (true, true) => OverlayState::Conflict,
                (true, false) => OverlayState::Applied,
            };
            o.rev = Rev(o.rev.0 + 1);
            let rev = o.rev;
            let e = st.log(now, who, LogAction::OverlayState);
            (e.overlay, e.rev_after) = (Some(id.clone()), Some(rev));
            Ok((rev, true))
        }))
    }
    fn delete_overlay<'a>(
        &'a self,
        id: &'a OverlayId,
        expected: Rev,
        who: &'a Who,
    ) -> Pending<'a, ()> {
        ready(self.tx(true, |st, now| {
            let i = st
                .overlays
                .iter()
                .position(|o| &o.id == id)
                .ok_or_else(|| unknown("overlay", &id.0.0))?;
            if st.overlays[i].rev != expected {
                return Err(rev_changed(&id.0.0, expected, st.overlays[i].rev));
            }
            let gone = st.overlays.remove(i);
            st.conflicts.retain(|c| &c.overlay != id);
            let e = st.log(now, who, LogAction::OverlayDelete);
            (e.layout, e.overlay) = (Some(gone.layout), Some(id.clone()));
            Ok(((), true))
        }))
    }
    fn import_overlay<'a>(
        &'a self,
        doc: OverlayDoc,
        mode: Mode,
        who: &'a Who,
    ) -> Pending<'a, Report> {
        let mut report = None;
        let r = self.tx(mode == Mode::Apply, |st, now| {
            let key = crate::ids::FrontendKey {
                extends: doc.extends.clone(),
                frontend: doc.frontend.clone(),
            };
            let i = st
                .live_key(&key)
                .ok_or_else(|| unknown("layout", &doc.frontend))?;
            if st.layouts[i]
                .package
                .as_ref()
                .is_none_or(|p| p.package != doc.package)
            {
                return Err(unknown("layout", &doc.frontend));
            }
            let layout = st.layouts[i].id.clone();
            let scope = OverlayScope::Person(who.author.clone());
            let id = self.create_in(
                st,
                now,
                &layout,
                scope,
                doc.lang.as_deref(),
                doc.title_key.as_deref(),
                who,
            )?;
            let ops = doc.ops.into_iter().map(|i| i.v).collect();
            let (r, ok) = self.ops_in(st, now, &id, Rev(1), ops, who)?;
            report = Some(r);
            Ok(((), ok))
        });
        ready(r.map(|()| report.unwrap_or_default()))
    }
    fn resolve_conflict<'a>(
        &'a self,
        id: &'a OverlayId,
        seq: u32,
        base_version: &'a str,
        how: Resolution,
        who: &'a Who,
    ) -> Pending<'a, Rev> {
        ready(self.tx(true, |st, now| {
            let ci = st.conflicts.iter().position(|c| {
                &c.overlay == id && c.seq == seq && c.base_version == base_version && c.open
            });
            let ci = ci.ok_or_else(|| unknown("conflict", &id.0.0))?;
            let oi = st
                .overlays
                .iter()
                .position(|o| &o.id == id)
                .ok_or_else(|| unknown("overlay", &id.0.0))?;
            let li = st
                .live(&st.overlays[oi].layout)
                .ok_or_else(|| unknown("layout", &st.overlays[oi].layout.0.0))?;
            let mut original = st.layouts[li]
                .layout
                .clone()
                .ok_or_else(|| unknown("layout", &id.0.0))?;
            let pos = st.overlays[oi]
                .ops
                .iter()
                .position(|o| o.seq == seq)
                .ok_or_else(|| unknown("conflict", &id.0.0))?;
            let op = &mut st.overlays[oi].ops[pos];
            match how {
                Resolution::Drop => {
                    st.overlays[oi].ops.remove(pos);
                }
                Resolution::Keep => {
                    if let OpKind::Set {
                        target,
                        field,
                        expect,
                        ..
                    } = &mut op.kind
                    {
                        if let Some(n) = find(original.doc_mut(), target) {
                            *expect = get(&n, *field).unwrap_or(Value::None);
                        }
                    }
                }
                Resolution::Retarget(elem) => {
                    retarget(original.doc_mut(), &mut op.kind, elem, id, seq)?
                }
            }
            st.conflicts[ci].open = false;
            let o = &mut st.overlays[oi];
            o.rev = Rev(o.rev.0 + 1);
            let (rev, layout) = (o.rev, o.layout.clone());
            let e = st.log(now, who, LogAction::ConflictResolve);
            (e.layout, e.overlay, e.rev_after) = (Some(layout), Some(id.clone()), Some(rev));
            Ok((rev, true))
        }))
    }
}

/// Aims an op at another element of the same kind; it must exist.
fn retarget(
    doc: &mut crate::doc::LayoutDoc,
    kind: &mut OpKind,
    elem: ElemId,
    id: &OverlayId,
    seq: u32,
) -> Res<()> {
    let (OpKind::Set { target, .. } | OpKind::Remove { target } | OpKind::Move { target, .. }) =
        kind
    else {
        return Err(Coded::new(LayoutCode::OpInvalid)
            .with("overlay", &id.0.0)
            .with("seq", seq)
            .with("cause", "no-target"));
    };
    let wanted = crate::ids::ElemRef {
        kind: target.kind,
        elem,
    };
    if find(doc, &wanted).is_none() {
        return Err(Coded::new(LayoutCode::OpInvalid)
            .with("overlay", &id.0.0)
            .with("seq", seq)
            .with("cause", "retarget"));
    }
    *target = wanted;
    Ok(())
}
