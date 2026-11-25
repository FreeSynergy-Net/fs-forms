//! What the placement declares about HOW its block is shown: `view` and `variant` of the `set` table of the placement.
//!
//! Pattern: Adapter (the host reads the declaration of the placement and hands it on in the type of fs-components-core; the
//! type itself is defined there ONCE). The host only READS the two keys; whether a word is allowed is the catalog's job (check
//! K20, the `sets` of the block), never repeated here. A placement that does not say: `None` (`PlaceInfo::default()`), the
//! component takes the standard of its contract.

use fs_forms_layout::{AttrDoc, Item};

pub use fs_components_core::form_view::PlaceInfo;

/// The key of the view in the `set` table of a placement.
const VIEW: &str = "view";
/// The key of the variant in the `set` table of a placement.
const VARIANT: &str = "variant";

/// Reads the two keys from the attributes of a placement (`set`); other keys are not the business of the host here.
#[must_use]
pub fn from_set<'a>(set: impl IntoIterator<Item = (&'a str, &'a str)>) -> PlaceInfo {
    let mut out = PlaceInfo::default();
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
pub fn from_doc(set: &[Item<AttrDoc>]) -> PlaceInfo {
    from_set(set.iter().map(|a| (a.v.name.as_str(), a.v.value.as_str())))
}

#[cfg(test)]
mod tests {
    use super::{PlaceInfo, from_set};

    #[test]
    fn reads_view_and_variant() {
        let p = from_set([("view", "month"), ("variant", "color-2"), ("x", "y")]);
        assert_eq!(p.view.as_deref(), Some("month"));
        assert_eq!(p.variant.as_deref(), Some("color-2"));
    }

    #[test]
    fn missing_is_none() {
        assert_eq!(from_set([("other", "1")]), PlaceInfo::default());
        assert_eq!(from_set([]), PlaceInfo::default());
    }

    #[test]
    fn only_one_declared() {
        let p = from_set([("view", "week")]);
        assert_eq!((p.view.as_deref(), p.variant), (Some("week"), None));
    }
}
