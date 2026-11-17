//! Queries on a built `Plan`: canonical form, all locks, placement lookup.
//!
//! Pattern: Query methods kept apart from the value objects.

use crate::codes::PartLock;
use crate::plan::{FieldFill, Fill, InputState, PlacePlan, PlaceStatus, Plan};

impl Plan {
    /// Canonical form of the plan WITHOUT `style` (rule `plan.style-blind`). Equal for every style.
    /// A sha256 over it comes when fs-types is a dependency (this crate has none on purpose).
    #[must_use]
    pub fn canonical_without_style(&self) -> String {
        let mut p = self.clone();
        p.style = None;
        format!("{p:?}")
    }

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
            for pl in &page.places {
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
            .places
            .iter()
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
