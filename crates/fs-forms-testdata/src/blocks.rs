//! Which forms a description needs: what its blocks draw (`forms=` of the block contract) and what the heads of a grouping slot come as.
//!
//! Pattern: Port (consumer side, [`BlockForms`] lives in `fs-forms-bind`; the host gives the adapter over the block catalog, this crate links no component crate)
//! + Template method (`needs` reads the layout and asks the port; nothing is decided here).

pub use fs_forms_bind::BlockForms;
use fs_forms_bind::FormRef;
use fs_forms_layout::{BindFrom, Layout};

use crate::error::{Failure, SampleCode, fail};

/// One form a source must serve for the blocks of the description.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Need {
    /// The number of the source (counts from 1, as `source` of a block).
    pub source: u32,
    /// The data type of that source (`type` of the `[[source]]`).
    pub data_type: String,
    /// The form with the lowest version a block draws (several blocks asking for the same form keep the highest minor).
    pub form: FormRef,
}

/// The forms every source of `layout` must serve, in order of source and first appearance. A block without `source` reads the first
/// source (as the host does); the heads of a grouping binding with a `source` ask the forms of the slot from THAT source.
///
/// # Errors
/// `source-unknown` when a block or a binding names a source the layout does not have (the checked layout never does);
/// `block-unknown` when the catalog gives no block or no slot for a grouping binding (never a head wish left out in silence).
pub fn needs(layout: &Layout, blocks: &dyn BlockForms) -> Result<Vec<Need>, Failure> {
    let doc = layout.doc();
    let mut out: Vec<Need> = Vec::new();
    let mut add = |source: u32, form: FormRef| -> Result<(), Failure> {
        if source == 0 {
            return Ok(()); // no source at all: nothing is asked of anybody
        }
        let data_type = doc
            .source_type(source)
            .map(str::to_owned)
            .ok_or_else(|| fail(SampleCode::SourceUnknown, "source", source.to_string()))?;
        let same = |n: &&mut Need| {
            n.source == source && n.form.name == form.name && n.form.major == form.major
        };
        match out.iter_mut().find(same) {
            Some(n) => n.form.minor = n.form.minor.max(form.minor),
            None => out.push(Need {
                source,
                data_type,
                form,
            }),
        }
        Ok(())
    };
    for page in &doc.pages {
        for leaf in page.v.leaves() {
            let block = leaf.v;
            let source = doc.source_of(block);
            for form in blocks.forms(&block.block) {
                add(source, form)?;
            }
            for binding in &block.bindings {
                let (BindFrom::Grouping(_), Some(heads)) = (&binding.v.from, binding.v.source)
                else {
                    continue;
                };
                let slot = blocks
                    .spec(&block.block)
                    .and_then(|s| s.slots.into_iter().find(|s| s.name == binding.v.slot))
                    .ok_or_else(|| fail(SampleCode::BlockUnknown, "block", block.block.clone()))?;
                for form in slot.heads {
                    add(heads, form)?;
                }
            }
        }
    }
    out.sort_by_key(|n| n.source); // stable: first appearance stays within a source
    Ok(out)
}
