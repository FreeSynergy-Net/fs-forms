//! Re-applying the overlays of a layout after its original changed (install of a new version, revival).
//!
//! Pattern: Unit of work (see `memory.rs`); Command replay.

use crate::apply::apply;
use crate::ids::PackageRef;
use crate::memory::{MemoryLayoutStore, State};
use crate::overlay::{Conflict, OverlayState, Value};
use crate::shim::Utc;

impl MemoryLayoutStore {
    /// Applies every overlay of a layout again after its original changed; writes conflicts, never drops one.
    pub(crate) fn rebase(
        &self,
        st: &mut State,
        idx: usize,
        base: &PackageRef,
        now: Utc,
    ) -> Vec<Conflict> {
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
            let applied = apply(
                &orig,
                std::slice::from_ref(&*o),
                &locks,
                &base.version,
                &*self.catalog,
                &self.limits,
            );
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
