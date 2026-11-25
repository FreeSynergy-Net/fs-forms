//! What the host reads for each `[[source]]` of the description (a [`Want`]) and which source feeds which block (a [`Feed`]): the
//! assignment of data is by (source, form, type), never by the form name alone, because a module may declare the same form twice
//! (`rows@1.0.0` for `Lane` and for `Task`).
//!
//! Pattern: Value objects read from the layout (the plan of the description in, plain lists out); no decision is made here.

use fs_forms_layout::{BindFrom, BlockDoc, Layout, PageDoc, PlaceNode};

use crate::view::Coded;

/// One source of the description as the host reads it: the contract range, the data type and the scope. `all` and `selected-list` are
/// words of the host and mean no scope (`""`); any other scope goes WORD FOR WORD to the module (`lanes:assignee`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Want {
    /// The number of the source (counts from 1, as `source` of a block and a binding).
    pub n: u32,
    /// The contract range (`planning@>=0.0.1, <0.1.0`).
    pub range: String,
    /// The data type the records of this source have (`Task`, `Lane`).
    pub data_type: String,
    /// The scope for `FormSource.page` and `watch`; `""` = all.
    pub scope: String,
}

/// The heads of a grouping: the records of another source are the groups.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeadsRef {
    /// The source that gives the heads.
    pub source: u32,
    /// Its data type.
    pub data_type: String,
    /// The channel the cards are grouped by (`assignee`).
    pub grouping: String,
}

/// The data one block is fed with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Feed {
    /// The block of the catalog.
    pub block: String,
    /// The source of its records (counts from 1; `0` = none named).
    pub source: u32,
    /// The data type of that source (`""` = none named).
    pub data_type: String,
    /// The source of the heads of a grouping, when a binding names one.
    pub heads: Option<HeadsRef>,
    /// The locks of single slots of this place (the plan of `fs-forms-bind` says why a slot is shut, e.g. the lane list).
    pub slot_locks: Vec<Coded>,
}

/// What to read for every source of `layout`, in order.
#[must_use]
pub fn wants_of(layout: &Layout) -> Vec<Want> {
    layout
        .doc()
        .sources
        .iter()
        .zip(1u32..)
        .map(|(s, n)| Want {
            n,
            range: s.v.range.clone(),
            data_type: s.v.data_type.clone(),
            scope: s.v.scope.as_ref().map_or_else(String::new, host_text),
        })
        .collect()
}

/// The text the module is given: the host words mean "no scope".
fn host_text(scope: &fs_forms_layout::SourceScope) -> String {
    match scope.text() {
        "all" | "selected-list" => String::new(),
        other => other.to_owned(),
    }
}

fn blocks_of(page: &PageDoc) -> Vec<&BlockDoc> {
    let flat = page.blocks.iter().map(|b| &b.v);
    let tree = page.places.iter().filter_map(|n| match &n.v {
        PlaceNode::Block(b) => Some(b),
        _ => None,
    });
    flat.chain(tree).collect()
}

/// Which source feeds which block (the blocks of the first level, as the host draws them).
#[must_use]
pub fn feeds_of(layout: &Layout) -> Vec<Feed> {
    let doc = layout.doc();
    let type_of = |n: u32| {
        usize::try_from(n)
            .ok()
            .and_then(|n| n.checked_sub(1))
            .and_then(|i| doc.sources.get(i))
            .map(|s| s.v.data_type.clone())
    };
    let first = u32::from(!doc.sources.is_empty());
    doc.pages
        .iter()
        .flat_map(|p| blocks_of(&p.v))
        .map(|b| {
            let source = b.source.unwrap_or(first);
            let heads = b
                .bindings
                .iter()
                .find_map(|x| match (&x.v.from, x.v.source) {
                    (BindFrom::Grouping(g), Some(h)) => Some(HeadsRef {
                        source: h,
                        data_type: type_of(h).unwrap_or_default(),
                        grouping: g.clone(),
                    }),
                    _ => None,
                });
            Feed {
                block: b.block.clone(),
                source,
                data_type: type_of(source).unwrap_or_default(),
                heads,
                slot_locks: Vec::new(),
            }
        })
        .collect()
}
