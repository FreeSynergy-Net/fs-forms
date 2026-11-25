//! The error codes of the layouts interface (prefix `layout-error-`; MF2 keys de + en live in the program).
//! Append only, never renamed (new codes go to the END of the list). Parameters are named in the comment of each code.
//!
//! Pattern: Value Object: a closed enum with a 1:1 text form.

use crate::shim::ErrorCode;

macro_rules! codes {
    ($($(#[$m:meta])* $v:ident => $s:literal),* $(,)?) => {
        /// A code of the layouts interface.
        #[non_exhaustive]
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum LayoutCode { $($(#[$m])* $v),* }
        impl LayoutCode {
            /// Every code once.
            pub const ALL: &'static [LayoutCode] = &[$(LayoutCode::$v),*];
            /// Reads the kebab-case name (without prefix).
            #[must_use]
            pub fn parse(name: &str) -> Option<Self> {
                match name { $($s => Some(Self::$v),)* _ => None }
            }
        }
        impl ErrorCode for LayoutCode {
            const PREFIX: &'static str = "layout-error-";
            fn as_str(self) -> &'static str { match self { $(Self::$v => $s),* } }
        }
    };
}

codes! {
    /// `{file, cause}` emergency mode, file byte-identical.
    StoreBroken => "store-broken",
    /// `{file, found, known}`.
    StoreNewer => "store-newer",
    /// `{file}`.
    StoreBusy => "store-busy",
    /// `{which}`.
    ReadOnly => "read-only",
    /// `{socket}`, client only.
    Unreachable => "unreachable",
    /// `{what}`, e.g. admin on the person service.
    Unsupported => "unsupported",
    /// `{what, id}`; also for "not yours" (no existence leak).
    Unknown => "unknown",
    /// `{id, expected, found}` compare-and-set lost.
    RevChanged => "rev-changed",
    /// `{size, max}`.
    PageSize => "page-size",
    /// `{}`.
    CursorInvalid => "cursor-invalid",
    /// `{id}` write to a package original outside install/uninstall.
    PackageOwned => "package-owned",
    /// `{extends, frontend}` key taken by a live layout.
    Exists => "exists",
    /// `{id}` overlay on a layout the person owns.
    NotOverlayable => "not-overlayable",
    /// `{elem, reason, by}`.
    Locked => "locked",
    /// `{what}`.
    ReasonMissing => "reason-missing",
    /// `{file, line, cause}`.
    ImportInvalid => "import-invalid",
    /// `{found, known}`.
    FormatNewer => "format-newer",
    /// `{found}`.
    FormatInvalid => "format-invalid",
    /// `{elem, line}`.
    ElemInvalid => "elem-invalid",
    /// `{line}` format 2.
    ElemMissing => "elem-missing",
    /// `{elem, kind}`.
    ElemTwice => "elem-twice",
    /// `{elem, kind}` overlay target or anchor not in the original.
    ElemUnknown => "elem-unknown",
    /// `{kind, key}` natural key twice.
    KeyTwice => "key-twice",
    /// `{text}`.
    RangeInvalid => "range-invalid",
    /// `{elem, block}`.
    BlockUnknown => "block-unknown",
    /// `{elem, cause}` cause = the catalog code without prefix.
    BindingInvalid => "binding-invalid",
    /// `{key}` MF2 key.
    KeyInvalid => "key-invalid",
    /// `{lang}` BCP 47.
    LangInvalid => "lang-invalid",
    /// `{key, lang, cause}`.
    MessageInvalid => "message-invalid",
    /// `{what, max, by}`.
    TooMany => "too-many",
    /// `{elem, bytes, max, by}`.
    TextTooLong => "text-too-long",
    /// `{elem, path}`.
    SchemesMissing => "schemes-missing",
    /// `{elem}`.
    SchemesEmpty => "schemes-empty",
    /// `{elem, scheme}` a valid scheme name that is not on the allowed (positive) list.
    SchemeNotAllowed => "scheme-not-allowed",
    /// `{elem, scheme}` not an RFC 3986 scheme name.
    SchemeBad => "scheme-bad",
    /// `{elem, scheme}`.
    SchemeTwice => "scheme-twice",
    /// `{elem, path}` list on a type without schemes.
    SchemesUnexpected => "schemes-unexpected",
    /// `{capability}` provider away, live checks pending.
    ProviderUnchecked => "provider-unchecked",
    /// `{overlay, seq, cause}`.
    OverlayConflict => "overlay-conflict",
    /// `{overlay, seq, cause}` op shape wrong.
    OpInvalid => "op-invalid",
    /// `{overlay}` writing ops to an orphaned overlay.
    Orphaned => "orphaned",
    /// `{file, kind}`.
    ExportFailed => "export-failed",
    /// `{name, bytes, max, by}` exchange content over the size limit.
    ContentTooLarge => "content-too-large",
    /// `{elem, cause}` the pair rules of a menu entry (command, view, separator).
    MenuInvalid => "menu-invalid",
    /// `{elem, source}` a source number that the description does not have.
    SourceUnknown => "source-unknown",
    /// `{elem, frame, place}` K4: the frame declares no such place.
    PlaceUnknown => "place-unknown",
    /// `{elem, place, name}` K6: the place does not take this block or frame.
    PlaceTakes => "place-takes",
    /// `{elem, place, count, min, max}` K6: too few or too many children on the place; `elem` is the frame or page.
    PlaceCount => "place-count",
    /// `{elem, name, owner}` K20, warning only: the key is not in the `sets` of the block or frame `owner`.
    SetUnknown => "set-unknown",
    /// `{found, known}` a file in a format before 3 (`[[page.block]]`); it is not read, write it again in format 3.
    FormatOld => "format-old",
    /// `{elem, cause}` a frontend node (S6-3): cause = extends-empty, frontend-empty, user-unknown (`flag`), scope-from-unknown (`scope-from`).
    FrontendInvalid => "frontend-invalid",
}

/// The error texts of the layouts interface, German (ADR 0057, `MessageFormat` 2); one line per code: `layout-error-<code> = text`.
pub const TEXTS_DE: &str = include_str!("texts/de.mf2");
/// The same texts in English.
pub const TEXTS_EN: &str = include_str!("texts/en.mf2");
