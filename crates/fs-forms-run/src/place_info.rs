//! What the placement declares about HOW its block is shown: `view` and `variant` of the `set` table of the placement.
//!
//! Pattern: Strategy (the placement chooses the presentation; the component has no view in its head). The host only READS the two
//! keys and passes them on; whether a word is allowed is the catalog's job (check K20, the `sets` of the block), never repeated here.
//! A placement that does not say: `None`, the component takes the standard of its contract (no special case in the host).

use fs_forms_layout::{AttrDoc, Item};

/// The key of the view in the `set` table of a placement.
const VIEW: &str = "view";
/// The key of the variant in the `set` table of a placement.
const VARIANT: &str = "variant";

/// The declaration of a placement, as the host hands it to the component (not validated: the words are the contract's).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlaceInfo {
    /// `view=` (for a calendar: month, week, day, agenda).
    pub view: Option<String>,
    /// `variant=` (for a calendar: compact, color-N).
    pub variant: Option<String>,
}

impl PlaceInfo {
    /// Reads the two keys from the attributes of a placement (`set`); other keys are not the business of the host here.
    #[must_use]
    pub fn from_set<'a>(set: impl IntoIterator<Item = (&'a str, &'a str)>) -> Self {
        let mut out = Self::default();
        for (name, value) in set {
            match name {
                VIEW => out.view = Some(value.to_owned()),
                VARIANT => out.variant = Some(value.to_owned()),
                _ => {}
            }
        }
        out
    }

    /// The same, from the attributes of the layout document.
    #[must_use]
    pub fn from_doc(set: &[Item<AttrDoc>]) -> Self {
        Self::from_set(set.iter().map(|a| (a.v.name.as_str(), a.v.value.as_str())))
    }
}

#[cfg(test)]
mod tests {
    use super::PlaceInfo;

    #[test]
    fn reads_view_and_variant() {
        let p = PlaceInfo::from_set([("view", "month"), ("variant", "color-2"), ("x", "y")]);
        assert_eq!(p.view.as_deref(), Some("month"));
        assert_eq!(p.variant.as_deref(), Some("color-2"));
    }

    #[test]
    fn missing_is_none() {
        assert_eq!(PlaceInfo::from_set([("other", "1")]), PlaceInfo::default());
        assert_eq!(PlaceInfo::from_set([]), PlaceInfo::default());
    }

    #[test]
    fn only_one_declared() {
        let p = PlaceInfo::from_set([("view", "week")]);
        assert_eq!((p.view.as_deref(), p.variant), (Some("week"), None));
    }
}
