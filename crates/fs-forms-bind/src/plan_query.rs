//! Queries on a built `Plan`: all locks, placement lookup.
//!
//! Pattern: Query methods kept apart from the value objects.

use crate::codes::PartLock;
use crate::plan::{FieldFill, Fill, InputState, PlacePlan, PlaceStatus, Plan};

impl Plan {
    /// Every lock of the plan with its part.
    #[must_use]
    pub fn locks(&self) -> Vec<&PartLock> {
        let mut out: Vec<&PartLock> = self
            .sources
            .iter()
            .filter_map(|s| s.status.as_ref().err())
            .collect();
        out.extend(&self.menu.locks);
        for page in &self.pages {
            out.extend(&page.locks);
            for f in page.frame_locks() {
                out.push(f);
            }
            for pl in page.leaves() {
                match &pl.status {
                    PlaceStatus::Locked(l) => out.extend(l),
                    PlaceStatus::Ready(r) => {
                        out.extend(r.slot_locks());
                        for s in &r.slots {
                            collect_fill_locks(&s.fill, &mut out);
                        }
                    }
                }
            }
        }
        for c in self.menu.commands() {
            out.extend(&c.fixed);
        }
        out
    }

    /// The placement by (page, place).
    #[must_use]
    pub fn place(&self, page: &str, place: &str) -> Option<&PlacePlan> {
        self.pages
            .iter()
            .find(|p| p.page == page)?
            .leaves()
            .into_iter()
            .find(|p| p.place == place)
    }
}

fn collect_fill_locks<'a>(fill: &'a Fill, out: &mut Vec<&'a PartLock>) {
    match fill {
        Fill::Command(c) => out.extend(&c.fixed),
        Fill::Field(FieldFill {
            input: InputState::Locked(l),
            ..
        }) => out.push(l),
        _ => {}
    }
}

impl FieldFill {
    /// May an address with this scheme be accepted in this field? FAIL-CLOSED (K2, c0): no list, an empty list and a field without
    /// schemes (not a link or image) allow NOTHING; only a listed scheme (any case) is allowed. A person who sets the path of a
    /// binding to a link field therefore never gets a field that takes everything: without a per-use list the placement is locked
    /// (`layout-error-schemes-missing`) and this answers `false` for every scheme.
    #[must_use]
    pub fn allows_scheme(&self, scheme: &str) -> bool {
        self.basic.has_schemes()
            && self
                .schemes
                .as_deref()
                .is_some_and(|l| l.iter().any(|s| s.eq_ignore_ascii_case(scheme)))
    }
}
