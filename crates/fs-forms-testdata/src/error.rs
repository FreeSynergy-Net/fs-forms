//! The codes of this crate: what went wrong while a sample set is made, never a sentence (ADR 0057).
//!
//! Pattern: Value object + catalog (`ErrorCode` of fs-types): one stable enum, the program words it under `message_key`.

use fs_types::coded::{Coded, ErrorCode};

/// The stable codes of the sample builder. Variants are never renamed or reused.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleCode {
    /// `{form}`: the form (name or version requirement) is not in the catalog.
    FormUnknown,
    /// `{what}`: a wish cannot be met by this form (an `AllDay` case without a flag channel, a required channel emptied ...).
    CaseImpossible,
    /// `{what}`: a relation names a channel the form does not have, or one of the wrong kind, or no set used it.
    RelationUnknown,
    /// `{needed, count}`: the cases alone need more records than `count`.
    CountTooSmall,
    /// `{zone}`: the zone of the anchor is unknown to the zone database.
    ZoneUnknown,
    /// `{what}`: the anchor names no valid month, week or day.
    AnchorBad,
    /// `{what}`: no strategy or no content mapping exists for this kind of channel or type.
    Unsupported,
}

impl ErrorCode for SampleCode {
    const PREFIX: &'static str = "sample-error-";
    const ALL: &'static [Self] = &[
        Self::FormUnknown,
        Self::CaseImpossible,
        Self::RelationUnknown,
        Self::CountTooSmall,
        Self::ZoneUnknown,
        Self::AnchorBad,
        Self::Unsupported,
    ];

    fn as_str(self) -> &'static str {
        match self {
            Self::FormUnknown => "form-unknown",
            Self::CaseImpossible => "case-impossible",
            Self::RelationUnknown => "relation-unknown",
            Self::CountTooSmall => "count-too-small",
            Self::ZoneUnknown => "zone-unknown",
            Self::AnchorBad => "anchor-bad",
            Self::Unsupported => "unsupported",
        }
    }
}

/// A failure of the builder: a code with typed parameters.
pub type Failure = Coded<SampleCode>;

/// The code `code` with one text parameter.
pub(crate) fn fail(code: SampleCode, key: &str, value: impl Into<String>) -> Failure {
    Coded::new(code).with_text(key, value)
}
