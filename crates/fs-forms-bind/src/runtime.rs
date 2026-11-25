//! Pure decisions at run time, per record and per page: tested without a window.
//!
//! Pattern: Specification (one predicate per rule, case tables in tests); field reading is a Projection of the record's
//! `(path, value)` entries (a repeated path is a list, never reduced to its first entry).

use crate::coded::Coded;
use crate::codes::{BindCode, Part, PartLock};
use crate::plan::FieldFill;
use crate::types::{HiddenReason, Limits, RecordPage, RecordView};

/// What a field slot shows for one record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldShown {
    /// The value text (exactly one value).
    Value(String),
    /// A repeated value of one path (`assignee` with two people): ALL of them in the order the module gave, at least two, none empty.
    Values(Vec<String>),
    /// No value; distinct from hidden.
    Empty,
    /// Hidden for this person: shows "verborgen" + reason, NEVER a value.
    Hidden {
        /// Why.
        reason: HiddenReason,
    },
}

impl FieldShown {
    /// The coded note for a hidden field (`bind-error-field-hidden{path, reason}`).
    #[must_use]
    pub fn note(&self, path: &str) -> Option<Coded> {
        let FieldShown::Hidden { reason } = self else {
            return None;
        };
        let r = match reason {
            HiddenReason::NoRight => "no-right",
            HiddenReason::Sensitive => "sensitive",
        };
        Some(
            BindCode::FieldHidden
                .coded()
                .with("path", path)
                .with("reason", r),
        )
    }
}

/// What the field shows for the record: hidden wins over any value; a path the module repeats shows every one of its values.
#[must_use]
pub fn field_shown(fill: &FieldFill, record: &RecordView) -> FieldShown {
    let path = fill.path.join(".");
    if let Some((_, reason)) = record.hidden.iter().find(|(p, _)| *p == path) {
        return FieldShown::Hidden { reason: *reason };
    }
    let mut all: Vec<String> = record
        .values
        .iter()
        .filter(|(p, v)| *p == path && !v.is_empty())
        .map(|(_, v)| v.clone())
        .collect();
    match all.len() {
        0 => FieldShown::Empty,
        1 => FieldShown::Value(all.remove(0)),
        _ => FieldShown::Values(all),
    }
}

/// A page admitted or refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Admitted {
    /// Clean page, unchanged.
    Page(RecordPage),
    /// Refused as a whole: never drawn "a bit", no record of it leaves.
    Refused(PartLock),
}

/// A page from the records service is admitted only if it keeps the limits (`page-too-long`, `value-too-large`,
/// `hidden-has-value`, `estimate-mixed`). Refused: the lock sits on the placement (`part`).
#[must_use]
pub fn admit_page(limits: &Limits, size: usize, page: RecordPage, part: &Part) -> Admitted {
    // The cause of the placement lock is `bind-error-page-refused{kind}` with the records code as `kind` (draft section 13).
    let refuse = |c: Coded| {
        let mut r = BindCode::PageRefused.coded().with("kind", c.code.clone());
        for (k, v) in &c.params {
            r = r.with(k, v.clone());
        }
        Admitted::Refused(PartLock::locked(part.clone(), r))
    };
    if page.records.len() > size {
        let c = Coded::new("records-error-page-too-long")
            .with("size", size.to_string())
            .with("got", page.records.len().to_string());
        return refuse(c);
    }
    if page.total.is_some() && page.total_estimated.is_some() {
        return refuse(Coded::new("records-error-estimate-mixed"));
    }
    for r in &page.records {
        for (p, v) in &r.values {
            if r.hidden.iter().any(|(h, _)| h == p) {
                return refuse(
                    Coded::new("records-error-hidden-has-value").with("path", p.clone()),
                );
            }
            if v.len() > limits.value_max {
                let c = Coded::new("records-error-value-too-large")
                    .with("path", p.clone())
                    .with("max", limits.value_max.to_string());
                return refuse(c);
            }
        }
    }
    Admitted::Page(page)
}
