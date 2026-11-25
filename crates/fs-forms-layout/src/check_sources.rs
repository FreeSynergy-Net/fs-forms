//! The source reference of a binding (format 4): only `grouping` may name a source, and the source must exist.
//!
//! Pattern: Template method (like `check.rs`): a small function that only appends problems.

use crate::codes::LayoutCode;
use crate::doc::{BindFrom, BindingDoc, Item};
use crate::ids::ElemId;
use crate::shim::Coded;

/// `binding-invalid` with `cause = bind-source-kind{kind}` when a binding other than `grouping` names a source, and
/// `bind-source-unknown{index}` when the number is not one of the `sources` sources (numbers count from 1).
pub(crate) fn check_binding_sources(
    list: &[Item<BindingDoc>],
    sources: usize,
    out: &mut Vec<Coded<LayoutCode>>,
) {
    for b in list {
        let Some(n) = b.v.source else { continue };
        let elem = b.elem.as_ref().map_or("", ElemId::as_str);
        let bad = |cause: &str| {
            Coded::new(LayoutCode::BindingInvalid)
                .with("elem", elem)
                .with("cause", cause)
        };
        if !matches!(b.v.from, BindFrom::Grouping(_)) {
            out.push(bad("bind-source-kind").with("kind", kind_word(&b.v.from)));
        } else if n == 0 || usize::try_from(n).map_or(true, |n| n > sources) {
            out.push(bad("bind-source-unknown").with("index", n));
        }
    }
}

fn kind_word(from: &BindFrom) -> &'static str {
    match from {
        BindFrom::Records => "records",
        BindFrom::Field { .. } => "field",
        BindFrom::Grouping(_) => "grouping",
        BindFrom::Order(_) => "order",
        BindFrom::Command(_) => "command",
        BindFrom::Lock => "lock",
        BindFrom::Condition => "condition",
    }
}
