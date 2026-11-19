//! The style of the window. A description names a style (`style = "default"`) or says nothing; `default` means "the style set
//! for the program" (ADR 0097 E2): the host resolves it to a [`StyleId`] by asking a [`StyleSource`], the setting of the program
//! (today the argument `--style`, later the setting service), with `admin` as the preset. The style is asked at EVERY drawing, not
//! once at start: a change of the setting shows in the next drawing, no restart.
//!
//! Pattern: Port (`StyleSource`) with a cell as the adapter; `resolve` is a pure function (the rule), so it is testable without a window.

use std::sync::atomic::{AtomicUsize, Ordering};

use fs_components_styles::StyleId;

/// The word of a description that means "the style set for the program".
pub const DEFAULT_STYLE: &str = "default";

/// Where the style set for the program comes from.
pub trait StyleSource: Send + Sync {
    /// The style set for the program now.
    fn current(&self) -> StyleId;
}

/// A style that can be changed while the window runs (the adapter of the setting; also used by tests).
#[derive(Debug)]
pub struct StyleCell(AtomicUsize);

impl StyleCell {
    /// A cell holding `id`.
    #[must_use]
    pub fn new(id: StyleId) -> Self {
        Self(AtomicUsize::new(index(id)))
    }

    /// Change the style set for the program; the next drawing shows it.
    pub fn set(&self, id: StyleId) {
        self.0.store(index(id), Ordering::Relaxed);
    }
}

impl Default for StyleCell {
    /// The preset: `admin`.
    fn default() -> Self {
        Self::new(StyleId::Admin)
    }
}

fn index(id: StyleId) -> usize {
    StyleId::ALL.iter().position(|s| *s == id).unwrap_or(0)
}

impl StyleSource for StyleCell {
    fn current(&self) -> StyleId {
        StyleId::ALL
            .get(self.0.load(Ordering::Relaxed))
            .copied()
            .unwrap_or(StyleId::Admin)
    }
}

/// The style a description gets: `default` (or none) is the style set for the program; another word is the name of a style; a name
/// that is no style is the style of the program too (the host says so: [`unknown`]).
#[must_use]
pub fn resolve(description: Option<&str>, program: StyleId) -> StyleId {
    match description.map(str::trim) {
        None | Some("" | DEFAULT_STYLE) => program,
        Some(name) => StyleId::from_name(name).unwrap_or(program),
    }
}

/// The name in a description that is neither `default` nor a style of this build.
#[must_use]
pub fn unknown(description: Option<&str>) -> Option<&str> {
    let name = description.map(str::trim)?;
    (!name.is_empty() && name != DEFAULT_STYLE && StyleId::from_name(name).is_none())
        .then_some(name)
}

/// The names of the styles, for the help text and the error of a wrong `--style`.
#[must_use]
pub fn names() -> String {
    StyleId::ALL
        .iter()
        .map(|s| s.name())
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::{StyleCell, StyleSource, names, resolve, unknown};
    use fs_components_styles::StyleId;

    #[test]
    fn default_is_the_style_set_for_the_program_and_a_name_is_that_style() {
        for word in [None, Some(""), Some("default"), Some(" default ")] {
            assert_eq!(resolve(word, StyleId::Apple), StyleId::Apple, "{word:?}");
        }
        assert_eq!(resolve(Some("windows"), StyleId::Apple), StyleId::Windows);
        assert_eq!(resolve(Some("no-such"), StyleId::Simple), StyleId::Simple);
    }

    #[test]
    fn only_a_word_that_is_no_style_is_unknown() {
        assert_eq!(unknown(Some("no-such")), Some("no-such"));
        for word in [None, Some(""), Some("default"), Some("apple")] {
            assert_eq!(unknown(word), None, "{word:?}");
        }
    }

    #[test]
    fn the_cell_holds_every_style_and_the_preset_is_admin() {
        assert_eq!(StyleCell::default().current(), StyleId::Admin);
        let cell = StyleCell::default();
        for id in StyleId::ALL {
            cell.set(id);
            assert_eq!(cell.current(), id);
        }
        assert!(names().contains("apple") && names().contains("admin"));
    }
}
