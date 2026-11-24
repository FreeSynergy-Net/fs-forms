//! A slot that stands for "empty" while records ARE there is never drawn as empty: its content is not bound or built yet, and the
//! slot says so as a tooltip ("not finished yet"), the one rule of `fs-components` (`Disabled::wrap` with `Why::Unsupported`).
//!
//! Pattern: Decorator (`Disabled` wraps the placeholder) over a Specification (`slot`: one predicate "records are there"); the host
//! paints nothing of its own, the placeholder is the `skeleton` block.

use fs_components::blinc::layout::div::{Div, div};
use fs_components_widgets::disabled::{Disabled, Labels, Why};
use fs_components_widgets::states::Skeleton;
use fs_components_words::{Locale, Localized as _};

use crate::records::Records;

/// What an "empty" slot may show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Slot {
    /// Nothing is there: the empty sentence is true.
    Empty,
    /// Records are there, the slot is not bound or built: the tooltip "not finished yet", never "empty".
    Unfinished,
}

/// Whether any form of `records` has a row.
pub(crate) fn has_data(records: &dyn Records) -> bool {
    records.forms().iter().any(|f| !f.rows.is_empty())
}

/// What an empty slot shows when `data` says whether records are there.
pub(crate) fn slot(data: bool) -> Slot {
    if data { Slot::Unfinished } else { Slot::Empty }
}

/// Rows of bars a placeholder shows (a height, so the tooltip has a surface to be reached).
const BARS: usize = 3;

/// The grey bars of the slot `name`, with an id of their own per slot `id`.
fn placeholder(id: &str, name: &str) -> Skeleton {
    Skeleton::new(name)
        .id(&format!("unfinished-{id}-skel"))
        .lines(BARS)
}

/// The placeholder of the slot `name` with the reason "not finished yet" as tooltip (hover, focus, press); it has bars, so it is
/// a surface of a height, not an empty box.
pub(crate) fn unfinished(locale: &Locale, id: &str, name: &str) -> Div {
    let labels = Labels::for_locale(locale);
    Disabled::new(&format!("unfinished-{id}"), Why::Unsupported, &labels)
        .wrap(div().child(placeholder(id, name).build()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::records::NoRecords;
    use crate::sample::SampleBoard;

    #[test]
    fn records_decide_between_empty_and_unfinished() {
        assert_eq!(slot(false), Slot::Empty);
        assert_eq!(slot(true), Slot::Unfinished);
        assert!(!has_data(&NoRecords), "nothing connected: nothing is there");
    }

    #[test]
    fn the_placeholder_has_bars_and_an_id_per_slot() {
        assert!(placeholder("a", "a").bars() >= 1, "no bars means no height");
        let _ = unfinished(&Locale::from_posix("en").expect("a locale"), "a", "a");
    }

    #[test]
    fn sample_records_with_cards_are_data() {
        assert!(
            has_data(&SampleBoard::default()),
            "the sample board has cards"
        );
    }
}
