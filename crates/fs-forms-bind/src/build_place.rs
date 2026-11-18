//! Steps 3 to 5 of `Bind::build` for one placement: block, accepts, field choice, schemes, commands.
//!
//! Pattern: Builder. Collects ALL problems of the placement, never only the first.

use crate::build::Pool;
use crate::catalog::{Catalog, check_bindings};
use crate::coded::Coded;
use crate::codes::{BindCode, Part, PartLock, Severity};
use crate::commands::command_plan;
use crate::model::{BindKind, BindingIn, PageIn, PlaceIn};
use crate::plan::{
    ConditionFill, FieldFill, Fill, InputState, PlacePlan, PlaceStatus, ReadyPlace, SlotFill,
    SourcePlan,
};
use crate::providers::HostCaps;
use crate::types::{Basic, FieldDesc, Role, schemes_rule};

pub(crate) struct Env<'a> {
    pub catalog: &'a dyn Catalog,
    pub host: HostCaps,
    pub sources: &'a [SourcePlan],
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
        match env.sources.get(i as usize).map(|s| &s.status) {
            Some(Err(l)) => locks.push(PartLock {
                part: part.clone(),
                severity: Severity::Locked,
                cause: l.cause.clone(),
            }),
            Some(Ok(_)) => {}
            None => locks.push(PartLock::locked(
                part.clone(),
                Coded::new("layout-error-source-unknown").with("index", i.to_string()),
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
        slots.push(slot(env, page, p, b, &part, &mut locks, pool));
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
        status,
    }
}

/// A locked placement drops its slots; their locks (layout problem, missing component, locked command) must stay visible.
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

fn slot(
    env: &Env<'_>,
    page: &PageIn,
    p: &PlaceIn,
    b: &BindingIn,
    place: &Part,
    locks: &mut Vec<PartLock>,
    pool: &mut Pool<'_>,
) -> SlotFill {
    let spart = Part::Slot {
        page: page.page.clone(),
        place: p.place.clone(),
        slot: b.slot.clone(),
        nth: b.nth,
    };
    let mut slot_lock = None;
    let mut layout_schemes = false;
    for c in pool.take(&b.elem) {
        if c.code.ends_with("-schemes-missing") || c.code.ends_with("-schemes-unexpected") {
            layout_schemes |= c.code.ends_with("-schemes-missing");
            locks.push(PartLock::locked(place.clone(), c));
        } else if slot_lock.is_none() {
            slot_lock = Some(PartLock::locked(spart.clone(), c));
        } else {
            locks.push(PartLock::locked(place.clone(), c));
        }
    }
    let fill = match &b.kind {
        BindKind::Records(s) => Fill::Records { source: *s },
        BindKind::Grouping(n) => Fill::Grouping(n.clone()),
        BindKind::Order(n) => Fill::Order(n.clone()),
        BindKind::Command(c) => {
            let src = env
                .sources
                .get(c.source as usize)
                .and_then(|s| s.status.as_ref().err());
            Fill::Command(command_plan(c, env.host, src))
        }
        BindKind::Condition { path, basic } => {
            let desc = FieldDesc {
                path: path.clone(),
                basic: *basic,
                target: None,
                read_only: false,
            };
            Fill::Condition(ConditionFill {
                path: path.clone(),
                basic: *basic,
                chip: env.catalog.choose(&desc, Role::Chip),
            })
        }
        BindKind::Field {
            field,
            role,
            schemes,
        } => {
            // ONE rule source (question 81): F1b's `schemes_rule`; mirrored in `types::schemes_rule` until its crate is pinned.
            // A code F1b already reported for this element is not repeated.
            if !layout_schemes
                && let Some(c) = schemes_rule(
                    field.basic,
                    schemes.as_deref(),
                    &b.elem,
                    &field.path.join("."),
                )
            {
                locks.push(PartLock::locked(place.clone(), c));
            }
            let (f, l) = field_fill(env, field, *role, schemes.clone(), &spart);
            if slot_lock.is_none() {
                slot_lock = l;
            }
            Fill::Field(f)
        }
    };
    SlotFill {
        slot: b.slot.clone(),
        nth: b.nth,
        elem: b.elem.clone(),
        fill,
        lock: slot_lock,
    }
}

fn field_fill(
    env: &Env<'_>,
    field: &FieldDesc,
    role: Role,
    schemes: Option<Vec<String>>,
    part: &Part,
) -> (FieldFill, Option<PartLock>) {
    let block = env.catalog.choose(field, role);
    let missing = field.basic == Basic::Unknown || !env.catalog.built(&block);
    let lock = missing.then(|| {
        let basic = field.basic.name();
        let c = BindCode::FieldComponentMissing
            .coded()
            .with("block", block.clone())
            .with("basic", basic)
            .with("role", role.name());
        PartLock::locked(part.clone(), c)
    });
    let input = match (role, &lock) {
        (Role::Input, _) if field.read_only => InputState::NotInput,
        (Role::Input, Some(l)) => InputState::Locked(l.clone()),
        (Role::Input, None) if !env.host.write => InputState::Locked(PartLock::locked(
            part.clone(),
            BindCode::WriteLater
                .coded()
                .with("command", field.path.join(".")),
        )),
        (Role::Input, None) => InputState::Open,
        _ => InputState::NotInput,
    };
    let ff = FieldFill {
        path: field.path.clone(),
        role,
        basic: field.basic,
        field: field.clone(),
        block,
        schemes,
        input,
    };
    (ff, lock)
}
