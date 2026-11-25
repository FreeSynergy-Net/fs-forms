//! Puts the answer of a fixed source into the set, next to the generated ones.
//!
//! Pattern: Adapter step of the builder (the wish names a source; its slice becomes an ordinary `Slice`: same order, same `render()` line,
//! so `SampleModule` and the host cannot tell the two apart).

use fs_types::Zone;

use super::FixedRegistry;
use crate::builder::resolve;
use crate::error::{Failure, SampleCode, fail};
use crate::set::{SampleSet, Slice};
use crate::spec::{SetWish, Span};

/// A fixed set has no wish of the generator; one next to `fixed` is `fixed-conflict`.
fn alone(wish: &SetWish) -> Result<(), Failure> {
    let written = [
        ("count", wish.count != 0),
        ("distinct", !wish.distinct.is_empty()),
        ("empty", !wish.empty.is_empty()),
        ("cases", !wish.cases.is_empty()),
        ("invalid", wish.invalid != 0),
    ];
    match written.iter().find(|(_, w)| *w) {
        Some((what, _)) => Err(fail(SampleCode::FixedConflict, "what", *what)),
        None => Ok(()),
    }
}

/// Asks the source `name` for the records of the form of `wish` in `window` and appends them to `set`.
///
/// # Errors
/// `fixed-unknown`, `fixed-conflict`, `fixed-type`, `form-unknown`, and whatever the source says.
pub(crate) fn place(
    registry: &FixedRegistry,
    name: &str,
    wish: &SetWish,
    (window, zone): (&Span, &Zone),
    set: &mut SampleSet,
) -> Result<(), Failure> {
    let source = registry.get(name)?;
    alone(wish)?;
    if source.data_type() != wish.data_type {
        return Err(fail(
            SampleCode::FixedType,
            "what",
            format!(
                "{name} gives {}, the wish says {}",
                source.data_type(),
                wish.data_type
            ),
        ));
    }
    let form = resolve(&wish.form)?;
    let slice = source.fill(form, window, zone)?;
    let (first, len) = (set.objects.len(), slice.objects.len());
    set.objects.extend(slice.objects);
    set.declared.push(slice.declared);
    set.names.extend(slice.names);
    set.now = Some(source.now(zone)?);
    set.slices.push(Slice {
        form: form.key(),
        data_type: wish.data_type.clone(),
        first,
        len,
        ctx: slice.ctx,
    });
    Ok(())
}
