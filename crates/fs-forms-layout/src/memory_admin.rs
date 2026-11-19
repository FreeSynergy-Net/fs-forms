//! `LayoutAdmin` of the memory store: instance locks and instance overlays. Only on the instance service; the person service
//! answers `unsupported{what=admin}`. A reason is mandatory for lock and unlock (after `trim`).
//!
//! Pattern: Unit of work (see `memory.rs`).

use crate::codes::LayoutCode;
use crate::elems::find;
use crate::ids::{ElemRef, LayoutId, OverlayId, Who};
use crate::memory::{MemoryLayoutStore, Res, ready, unknown};
use crate::overlay::{ElemLock, OverlayScope};
use crate::port::{LayoutAdmin, Pending};
use crate::shim::Coded;
use crate::types::LogAction;

impl MemoryLayoutStore {
    fn admin_gate(&self) -> Res<()> {
        if self.instance {
            Ok(())
        } else {
            Err(Coded::new(LayoutCode::Unsupported).with("what", "admin"))
        }
    }

    fn set_lock(
        &self,
        layout: &LayoutId,
        elem: Option<&ElemRef>,
        reason: &str,
        who: &Who,
        lock: bool,
    ) -> Res<bool> {
        self.admin_gate()?;
        let (what, action) = if lock {
            ("lock", LogAction::Lock)
        } else {
            ("unlock", LogAction::Unlock)
        };
        let reason = reason.trim();
        if reason.is_empty() {
            return Err(Coded::new(LayoutCode::ReasonMissing).with("what", what));
        }
        self.tx_for(who, true, |st, now| {
            let i = st
                .live(layout)
                .ok_or_else(|| unknown("layout", &layout.0.0))?;
            if let (Some(t), Some(mut l)) = (elem, st.layouts[i].layout.clone()) {
                find(l.doc_mut(), t).ok_or_else(|| {
                    Coded::new(LayoutCode::ElemUnknown)
                        .with("elem", t.elem.as_str())
                        .with("kind", t.kind.as_str())
                })?;
            }
            let same = |(l, k): &(LayoutId, ElemLock)| l == layout && k.target.as_ref() == elem;
            let changed = if lock {
                let new = !st.locks.iter().any(same);
                if new {
                    st.locks.push((
                        layout.clone(),
                        ElemLock {
                            target: elem.cloned(),
                            reason: reason.to_owned(),
                            locked_by: who.author.clone(),
                            locked_at: now,
                        },
                    ));
                }
                new
            } else {
                let before = st.locks.len();
                st.locks.retain(|x| !same(x));
                st.locks.len() != before
            };
            let e = st.log(now, who, action);
            (e.layout, e.elem, e.reason) = (
                Some(layout.clone()),
                elem.map(|t| t.elem.clone()),
                Some(reason.to_owned()),
            );
            Ok((changed, changed))
        })
    }
}

impl LayoutAdmin for MemoryLayoutStore {
    fn lock<'a>(
        &'a self,
        layout: &'a LayoutId,
        elem: Option<&'a ElemRef>,
        reason: &'a str,
        who: &'a Who,
    ) -> Pending<'a, bool> {
        ready(self.set_lock(layout, elem, reason, who, true))
    }
    fn unlock<'a>(
        &'a self,
        layout: &'a LayoutId,
        elem: Option<&'a ElemRef>,
        reason: &'a str,
        who: &'a Who,
    ) -> Pending<'a, bool> {
        ready(self.set_lock(layout, elem, reason, who, false))
    }
    fn create_instance_overlay<'a>(
        &'a self,
        layout: &'a LayoutId,
        lang: Option<&'a str>,
        title_key: Option<&'a str>,
        who: &'a Who,
    ) -> Pending<'a, OverlayId> {
        ready(self.admin_gate().and_then(|()| {
            self.tx_for(who, true, |st, now| {
                Ok((
                    self.create_in(
                        st,
                        now,
                        layout,
                        OverlayScope::Instance,
                        lang,
                        title_key,
                        who,
                    )?,
                    true,
                ))
            })
        }))
    }
}
