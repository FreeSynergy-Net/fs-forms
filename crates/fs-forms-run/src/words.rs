//! The words of the host: `MessageFormat` 2 files in German and English (ADR 0057), picked by the language of the session.
//!
//! Pattern: Factory over embedded `.mf2` files; a language without a file comes in English. The sentences of the plan's own codes
//! (`bind-error-*`, `accepts-error-*`, `layout-error-*`) belong to their crates; a code without a sentence here is shown as its code.

use fs_components_mf2::{Mf2Texts, Source};
use fs_components_words::Locale;

/// The two language files; `fs_components_mf2::check_pairs(TEXTS)` must be empty (a test asserts it).
pub const TEXTS: &[Source] = &[
    Source {
        language: "en",
        text: include_str!("texts/en.mf2"),
    },
    Source {
        language: "de",
        text: include_str!("texts/de.mf2"),
    },
];

/// The words in `locale`.
#[must_use]
pub fn words(locale: &Locale) -> Mf2Texts {
    Mf2Texts::new(TEXTS, locale)
}

/// The language of this session: `LC_ALL`, `LC_MESSAGES`, `LANG` in this order, else English.
#[must_use]
pub fn session_locale() -> Locale {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|name| std::env::var(name).ok())
        .find_map(|value| Locale::from_posix(&value))
        .unwrap_or_else(Locale::english)
}
