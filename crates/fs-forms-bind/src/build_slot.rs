//! Step 4 of `Bind::build` for one placement: the fill of ONE bound slot occurrence (records, field, grouping, command, condition).
//!
//! Pattern: Builder (continued from `build_place`). One lock per slot; every further problem goes to the placement.

use crate::build::Pool;
use crate::build_heads::heads_of_slot;
use crate::build_place::Env;
use crate::coded::Reason;
use crate::codes::{BindCode, Part, PartLock};
use crate::commands::command_plan;
use crate::model::{BindKind, BindingIn, PageIn, PlaceIn};
use crate::plan::{ConditionFill, FieldFill, Fill, InputState, SlotFill};
use crate::types::{Basic, FieldDesc, Role, schemes_rule};

/// A slot shows ONE lock; every further one becomes a placement lock (nothing silent).
fn keep_second(
    slot_lock: &mut Option<PartLock>,
    l: PartLock,
    locks: &mut Vec<PartLock>,
    place: &Part,
) {
    if slot_lock.is_none() {
        *slot_lock = Some(l);
    } else {
        locks.push(PartLock::locked(place.clone(), l.cause.coded().clone()));
    }
}

pub(crate) fn slot(
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
        if c.reason().is_some() {
            layout_schemes |= c.reason() == Some(Reason::SchemesMissing);
            locks.push(PartLock::locked(place.clone(), c));
        } else if slot_lock.is_none() {
            slot_lock = Some(PartLock::locked(spart.clone(), c));
        } else {
            locks.push(PartLock::locked(place.clone(), c));
        }
    }
    let fill = match &b.kind {
        BindKind::Records(s) => Fill::Records { source: *s },
        BindKind::Grouping { name, heads } => {
            if let Some(l) = heads.and_then(|h| heads_of_slot(env, p, b, h, &spart)) {
                keep_second(&mut slot_lock, l, locks, place);
            }
            Fill::Grouping {
                name: name.clone(),
                heads: *heads,
            }
        }
        BindKind::Order(n) => Fill::Order(n.clone()),
        BindKind::Command(c) => {
            let src = crate::plan_query::source_by_number(env.sources, c.source)
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
            let chip = env.catalog.choose(&desc, Role::Chip);
            // The chip component must be built, like a field component (B5, nothing silent).
            if !env.catalog.built(&chip) {
                let code = BindCode::FieldComponentMissing
                    .coded()
                    .with("block", chip.clone())
                    .with("basic", basic.name())
                    .with("role", Role::Chip.name());
                keep_second(
                    &mut slot_lock,
                    PartLock::locked(spart.clone(), code),
                    locks,
                    place,
                );
            }
            Fill::Condition(ConditionFill {
                path: path.clone(),
                basic: *basic,
                chip,
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
            // ONE lock per slot; a second one (a layout problem AND a missing component) stays visible as a placement
            // lock, like the other further problems above (nothing silent).
            if let Some(l) = l {
                keep_second(&mut slot_lock, l, locks, place);
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
