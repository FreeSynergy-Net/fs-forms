//! The rights of the sample module: everything is allowed except what the spec forbids.
//!
//! Pattern: Strategy (`RightsRules`) built from a Specification (`Forbid`, a wish of the spec); nothing is hidden.

use fs_interfaces::module::{Ask, Context, HiddenReason, RightsRules, Verdict};

use crate::spec::Forbid;

/// The rules of `SampleModule`: a command is denied when a `Forbid` of the spec names it (for that record, or everywhere).
#[derive(Clone, Debug, Default)]
pub struct SampleRules {
    forbid: Vec<Forbid>,
}

impl SampleRules {
    /// Rules that deny what `forbid` names.
    #[must_use]
    pub fn new(forbid: Vec<Forbid>) -> Self {
        Self { forbid }
    }
}

impl RightsRules for SampleRules {
    fn may(&self, ask: &Ask<'_>) -> Verdict {
        let denied = self.forbid.iter().any(|f| {
            f.command == ask.command && f.record.as_deref().is_none_or(|r| ask.record == Some(r))
        });
        if denied {
            Verdict::deny("sample-forbidden")
        } else {
            Verdict::Allow
        }
    }

    fn hidden(&self, _who: &Context, _record: &str, _field: &str) -> Option<HiddenReason> {
        None
    }
}
