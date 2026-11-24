//! Layer 2: the checked original. Same data as [`LayoutDoc`], but every element has its id, the natural keys are unique,
//! ranges parse, limits hold. Only [`LayoutDoc::check`] and the stores build one.
//!
//! Pattern: Value Object with a checked constructor (parse, don't validate).

use sha2::{Digest, Sha256};

use crate::doc::{BindFrom, LayoutDoc};
use crate::ids::ElemId;

/// The format version this crate reads.
pub const FORMAT_KNOWN: u32 = 2;

/// A checked element: id plus value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct El<T> {
    /// Element id.
    pub elem: ElemId,
    /// The value.
    pub v: T,
}

/// A checked description. Lines are zeroed; every `elem` is set.
///
/// Invariant (ONE source of the order, `LayoutDoc::check`): bindings of a block are in (slot, nth) byte order, messages in (key, lang),
/// views in (view, owner). The writer and the rows rely on it. When `apply` gets `Insert` and `Move`, it must restore this order at
/// its end.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    doc: LayoutDoc,
}

/// One binding of a block, as the host hands it to the catalog check (stand-in for `accepts::Binding`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BindingView {
    /// Slot name.
    pub slot: String,
    /// Position among the bindings of the slot.
    pub nth: u32,
    /// What it binds to.
    pub from: BindFrom,
    /// Scheme list in suggestion order.
    pub schemes: Option<Vec<String>>,
}

impl Layout {
    pub(crate) fn from_checked(doc: LayoutDoc) -> Self {
        Self { doc }
    }
    pub(crate) fn doc_mut(&mut self) -> &mut LayoutDoc {
        &mut self.doc
    }
    /// Read access to the tree (every `elem` is `Some`, every `line` is 0).
    #[must_use]
    pub fn doc(&self) -> &LayoutDoc {
        &self.doc
    }
    /// Back to layer 1 for the exchange writer: format 2, every id set. `to_doc` then `check` gives the same layout.
    #[must_use]
    pub fn to_doc(&self) -> LayoutDoc {
        LayoutDoc {
            format: FORMAT_KNOWN,
            ..self.doc.clone()
        }
    }
    /// The bindings of one block of one page; empty when page or place is unknown.
    #[must_use]
    pub fn bindings(&self, page: &str, place: &str) -> Vec<BindingView> {
        let found = self
            .doc
            .pages
            .iter()
            .filter(|p| p.v.page == page)
            .flat_map(|p| p.v.leaves())
            .find(|b| b.v.place == place);
        found
            .into_iter()
            .flat_map(|b| &b.v.bindings)
            .map(|b| BindingView {
                slot: b.v.slot.clone(),
                nth: b.v.nth,
                from: b.v.from.clone(),
                schemes: b
                    .v
                    .schemes
                    .as_ref()
                    .map(|l| l.iter().map(|s| s.v.clone()).collect()),
            })
            .collect()
    }
    /// Content hash (sha256 over the checked tree, without lines, revs and times): proves "original untouched".
    /// First cut: hashes the `Debug` text of the tree; stable inside one build, which is all the proofs need.
    #[must_use]
    pub fn digest(&self) -> [u8; 32] {
        Sha256::digest(format!("{:?}", self.doc).as_bytes()).into()
    }
}

impl Layout {
    /// Notices that never block (K20): `set` keys the catalog does not list for their block or frame. The host shows the page
    /// and says which key was ignored (`set-unknown`).
    #[must_use]
    pub fn warnings(
        &self,
        catalog: &dyn crate::check::CatalogRules,
    ) -> Vec<crate::shim::Coded<crate::codes::LayoutCode>> {
        crate::check_places::set_warnings(&self.doc, catalog)
    }
}
