//! Layer 2 to layer 3: applies overlays to an original. Pure: no clock, no store.
//!
//! Pattern: Command replay: stored ops run in a fixed order over a copy of the original; a failing op has no effect and is reported.

use crate::check::CatalogRules;
use crate::doc::LayoutDoc;
use crate::elems::{Node, find, get, remove, set};
use crate::ids::{ElemKind, ElemRef};
use crate::layout::Layout;
use crate::limits::Limits;
use crate::overlay::{
    Cause, ElemLock, Field, Op, OpKind, Overlay, OverlayScope, OverlayState, Value,
};
use crate::resolved::{Applied, Effect, Layer};

/// Applies the overlays. Order: instance overlays before person overlays, each by `seq` (never by time). Overlays that are
/// `Off` or `Orphaned` do nothing. Per op: target present, lock (person ops only), field allowed and typed, `expect` equal,
/// apply, then the result must pass the SAME check as any description (`check`: local rules, catalog, limits; D9), else revert with
/// `result-invalid`. Two more rules guard what an overlay may not do: a person's overlay never widens a scheme allowlist
/// (`allowlist-widened`; only the instance or the package itself may), and a source can only be removed when it is the last one
/// (a removal in the middle would renumber the sources and point blocks at another source).
/// A conflicting op has no effect and is reported; the others act. Never silent.
#[must_use]
pub fn apply(
    original: &Layout,
    overlays: &[Overlay],
    locks: &[ElemLock],
    _base_version: &str,
    catalog: &dyn CatalogRules,
    limits: &Limits,
) -> Applied {
    let mut layout = original.clone();
    let mut effects = Vec::new();
    let mut provenance: Vec<(ElemRef, Layer)> = Vec::new();
    let mut ordered: Vec<&Overlay> = overlays
        .iter()
        .filter(|o| matches!(o.state, OverlayState::Applied | OverlayState::Conflict))
        .collect();
    ordered.sort_by_key(|o| matches!(o.scope, OverlayScope::Person(_))); // stable: instance first
    for o in ordered {
        let person = matches!(o.scope, OverlayScope::Person(_));
        let mut ops: Vec<&Op> = o.ops.iter().collect();
        ops.sort_by_key(|op| op.seq);
        for op in ops {
            let result = apply_op(layout.doc_mut(), op, person, locks, (catalog, limits));
            if let (Ok(()), Some(t)) = (&result, target(&op.kind)) {
                let layer = if person {
                    Layer::Person(o.id.clone())
                } else {
                    Layer::Instance(o.id.clone())
                };
                provenance.retain(|(r, _)| r != t);
                provenance.push((t.clone(), layer));
            }
            effects.push(Effect {
                overlay: o.id.clone(),
                seq: op.seq,
                result,
            });
        }
    }
    Applied {
        layout,
        effects,
        provenance,
    }
}

fn target(k: &OpKind) -> Option<&ElemRef> {
    match k {
        OpKind::Set { target, .. } | OpKind::Remove { target } | OpKind::Move { target, .. } => {
            Some(target)
        }
        OpKind::Insert { .. } => None,
    }
}

fn locked(locks: &[ElemLock], t: &ElemRef) -> bool {
    locks
        .iter()
        .any(|l| l.target.as_ref().is_none_or(|x| x == t))
}

fn type_ok(f: Field, v: &Value) -> bool {
    match f {
        Field::Descending => matches!(v, Value::Bool(_)),
        Field::Source => matches!(v, Value::Int(_)),
        _ => matches!(v, Value::Text(_) | Value::None),
    }
}

fn apply_op(
    doc: &mut LayoutDoc,
    op: &Op,
    person: bool,
    locks: &[ElemLock],
    rules: (&dyn CatalogRules, &Limits),
) -> Result<(), Cause> {
    let Some(t) = target(&op.kind) else {
        return Err(Cause::FieldUnknown);
    };
    if find(doc, t).is_none() {
        return Err(Cause::TargetRemoved);
    }
    if person && locked(locks, t) {
        return Err(Cause::Locked);
    }
    let before = doc.clone();
    match &op.kind {
        OpKind::Set {
            target,
            field,
            value,
            expect,
        } => {
            if !field.applies_to(target.kind) {
                return Err(Cause::FieldUnknown);
            }
            if !type_ok(*field, value) {
                return Err(Cause::TypeMismatch);
            }
            if person && *field == Field::Scheme {
                return Err(Cause::AllowlistWidened); // any new value in the list would widen the allowlist of the use
            }
            let mut node = find(doc, target).ok_or(Cause::TargetRemoved)?;
            if matches!(node, Node::Other) {
                return Err(Cause::FieldUnknown); // not settable yet in this cut
            }
            if get(&node, *field).as_ref() != Some(expect) {
                return Err(Cause::TargetChanged);
            }
            if !set(&mut node, *field, value) {
                return Err(Cause::TypeMismatch);
            }
        }
        OpKind::Remove { target } => {
            let last_source = doc.sources.last().and_then(|s| s.elem.as_ref());
            if target.kind == ElemKind::Source && last_source != Some(&target.elem) {
                return Err(Cause::ResultInvalid); // would renumber the sources
            }
            if !remove(doc, target) {
                return Err(Cause::FieldUnknown);
            }
        }
        OpKind::Insert { .. } | OpKind::Move { .. } => return Err(Cause::FieldUnknown), // not applied yet in this cut
    }
    if let Some(checked) = result_checked(doc, rules) {
        *doc = checked;
        Ok(())
    } else {
        *doc = before;
        Err(Cause::ResultInvalid)
    }
}

/// D9: the result of every op goes through `check` again (local rules, catalog, limits); it comes back in canonical order.
fn result_checked(
    doc: &LayoutDoc,
    (catalog, limits): (&dyn CatalogRules, &Limits),
) -> Option<LayoutDoc> {
    doc.clone()
        .check(catalog, limits)
        .ok()
        .map(|l| l.doc().clone())
}
