//! Steps 3 to 5 of `Bind::build` for one placement: block, accepts, field choice, schemes, commands.
//!
//! Pattern: Builder. Collects ALL problems of the placement, never only the first.

use crate::build::Pool;
use crate::catalog::{Catalog, check_bindings};
use crate::coded::{Coded, Family};
use crate::codes::{BindCode, Part, PartLock, Severity};
use crate::model::{PageIn, PlaceIn};
use crate::plan::{Fill, PlacePlan, PlaceStatus, ReadyPlace, SlotFill, SourcePlan};
use crate::providers::{HostCaps, ProviderFacts};

pub(crate) struct Env<'a> {
    pub catalog: &'a dyn Catalog,
    pub host: HostCaps,
    pub sources: &'a [SourcePlan],
    /// What the modules declare (forms with their data type): the heads of a grouping are checked against it.
    pub facts: &'a dyn ProviderFacts,
}

pub(crate) fn build_place(
    env: &Env<'_>,
    page: &PageIn,
    p: &PlaceIn,
    seq_twice: bool,
    pool: &mut Pool<'_>,
) -> PlacePlan {
    let part = Part::Place {
        page: page.page.clone(),
        place: p.place.clone(),
    };
    let mut locks: Vec<PartLock> = pool
        .take(&p.elem)
        .into_iter()
        .map(|c| PartLock::locked(part.clone(), c))
        .collect();
    if seq_twice {
        let c = BindCode::SeqTwice
            .coded()
            .with("page", page.page.clone())
            .with("seq", p.seq.to_string());
        locks.push(PartLock::locked(part.clone(), c));
    }
    if let Some(i) = p.source {
        match crate::plan_query::source_by_number(env.sources, i).map(|s| &s.status) {
            Some(Err(l)) => locks.push(PartLock {
                part: part.clone(),
                severity: Severity::Locked,
                cause: l.cause.clone(),
            }),
            Some(Ok(_)) => {}
            None => locks.push(PartLock::locked(
                part.clone(),
                Coded::new("layout-error-source-unknown")
                    .of(Family::Layout)
                    .with("index", i.to_string()),
            )),
        }
    }
    match env.catalog.need(&p.block) {
        Err(c) => locks.push(PartLock::locked(part.clone(), c)),
        Ok(spec) => {
            if !env.catalog.built(&p.block) {
                locks.push(PartLock::locked(
                    part.clone(),
                    BindCode::BlockNotBuilt
                        .coded()
                        .with("block", p.block.clone()),
                ));
            }
            locks.extend(
                check_bindings(spec, &p.bindings)
                    .into_iter()
                    .map(|c| PartLock::locked(part.clone(), c)),
            );
        }
    }
    let mut slots = Vec::new();
    for b in &p.bindings {
        slots.push(crate::build_slot::slot(
            env, page, p, b, &part, &mut locks, pool,
        ));
    }
    if !locks.is_empty() {
        keep_slot_locks(&slots, &mut locks);
    }
    let status = if locks.is_empty() {
        PlaceStatus::Ready(ReadyPlace { slots })
    } else {
        PlaceStatus::Locked(locks)
    };
    PlacePlan {
        place: p.place.clone(),
        elem: p.elem.clone(),
        seq: p.seq,
        block: p.block.clone(),
        source: p.source,
        set: crate::build_tree::attrs(&p.set),
        status,
    }
}

/// A locked placement drops its slots; their locks (layout problem, missing component, locked command) must stay visible.
/// Duplicates are removed by CAUSE (code and parameters); the part of a duplicate is not kept, the first lock of that cause is.
fn keep_slot_locks(slots: &[SlotFill], locks: &mut Vec<PartLock>) {
    let mut add = |l: &PartLock| {
        if !locks.iter().any(|k| k.cause == l.cause) {
            locks.push(l.clone());
        }
    };
    let found: Vec<&PartLock> = slots
        .iter()
        .flat_map(|s| {
            let fixed = match &s.fill {
                Fill::Command(c) => c.fixed.as_ref().filter(|l| l.severity == Severity::Locked),
                _ => None,
            };
            s.lock.iter().chain(fixed)
        })
        .collect();
    for l in found {
        add(l);
    }
}
