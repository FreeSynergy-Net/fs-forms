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
    /// `{command}`: the declaration has no `writes` column for this command, so it cannot be run from the declaration.
    WritesUnknown,
    /// `{command}`: the command writes no channel (`open`, `remove` look the same in the declaration), so `SampleModule` has no effect to run.
    EffectUnknown,
    /// `{what}`: an argument is not written by the command, is doubled, is missing or its value does not fit the kind of its channel.
    ArgumentBad,
    /// `{command}`: no form of the module has a command channel of this name.
    CommandUnknown,
    /// `{record}`: the module has no such record.
    RecordUnknown,
    /// `{change}`: the module has no such change.
    ChangeUnknown,
    /// `{cursor}`: the cursor of a page is not one this module gave.
    CursorBad,
    /// `{field}`: the module has no binary field.
    BlobUnsupported,
    /// `{record}`: the record changed since the caller saw it (`expect`), or the change to take back is not the latest of its record.
    Stale,
    /// `{what}`: the section `[sample]` of the description cannot be read (not TOML, an unknown key, a value of the wrong kind or a bad anchor).
    SectionBad,
    /// `{source}`: a wish or a block names a source the description does not have.
    SourceUnknown,
    /// `{what}`: the `type` of a wish is not the type of its source.
    TypeMismatch,
    /// `{what}`: two wishes of `[sample]` for the same form of the same source.
    WishTwice,
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
        Self::WritesUnknown,
        Self::EffectUnknown,
        Self::ArgumentBad,
        Self::CommandUnknown,
        Self::RecordUnknown,
        Self::ChangeUnknown,
        Self::CursorBad,
        Self::BlobUnsupported,
        Self::Stale,
        Self::SectionBad,
        Self::SourceUnknown,
        Self::TypeMismatch,
        Self::WishTwice,
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
            Self::WritesUnknown => "writes-unknown",
            Self::EffectUnknown => "effect-unknown",
            Self::ArgumentBad => "argument-bad",
            Self::CommandUnknown => "command-unknown",
            Self::RecordUnknown => "record-unknown",
            Self::ChangeUnknown => "change-unknown",
            Self::CursorBad => "cursor-bad",
            Self::BlobUnsupported => "blob-unsupported",
            Self::Stale => "stale",
            Self::SectionBad => "section-bad",
            Self::SourceUnknown => "source-unknown",
            Self::TypeMismatch => "type-mismatch",
            Self::WishTwice => "wish-twice",
        }
    }
}

/// A failure of the builder: a code with typed parameters.
pub type Failure = Coded<SampleCode>;

/// The code `code` with one text parameter.
pub(crate) fn fail(code: SampleCode, key: &str, value: impl Into<String>) -> Failure {
    Coded::new(code).with_text(key, value)
}
