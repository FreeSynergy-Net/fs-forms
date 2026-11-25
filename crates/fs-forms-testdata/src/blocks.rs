//! Which forms a description needs: what its blocks draw (`forms=` of the block contract) and what the heads of a grouping slot come as.
//!
//! Pattern: Port (consumer side, [`BlockForms`]; the host gives the adapter over the block catalog, this crate links no component crate)
//! + Template method (`needs` reads the layout and asks the port; nothing is decided here).

use fs_forms_bind::{BlockSpec, FormRef};
use fs_forms_layout::{BindFrom, Layout};

use crate::error::{Failure, SampleCode, fail};

/// What the block catalog says; the host implements it over `blocks::forms_of` and `bind_spec::block_spec`.
pub trait BlockForms {
    /// The forms the component of `block` draws (`forms=` of the block contract); none for a block that draws no whole form.
    fn forms(&self, block: &str) -> Vec<FormRef>;

    /// The block with its slots (the `heads` of a grouping slot are the forms its heads come as), or `None` for an unknown block.
    fn spec(&self, block: &str) -> Option<BlockSpec>;
}

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
/// `source-unknown` when a block or a binding names a source the layout does not have (the checked layout never does).
pub fn needs(layout: &Layout, blocks: &dyn BlockForms) -> Result<Vec<Need>, Failure> {
    let doc = layout.doc();
    let first = u32::from(!doc.sources.is_empty());
    let mut out: Vec<Need> = Vec::new();
    let mut add = |source: u32, form: FormRef| -> Result<(), Failure> {
        if source == 0 {
            return Ok(()); // no source at all: nothing is asked of anybody
        }
        let at = usize::try_from(source).ok().and_then(|n| n.checked_sub(1));
        let data_type = at
            .and_then(|i| doc.sources.get(i))
            .map(|s| s.v.data_type.clone())
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
            let source = block.source.unwrap_or(first);
            for form in blocks.forms(&block.block) {
                add(source, form)?;
            }
            for binding in &block.bindings {
                let (BindFrom::Grouping(_), Some(heads)) = (&binding.v.from, binding.v.source)
                else {
                    continue;
                };
                let spec = blocks.spec(&block.block);
                let slot = spec
                    .iter()
                    .flat_map(|s| &s.slots)
                    .filter(|s| s.name == binding.v.slot);
                for form in slot.flat_map(|s| s.heads.iter().cloned()) {
                    add(heads, form)?;
                }
            }
        }
    }
    out.sort_by_key(|n| n.source); // stable: first appearance stays within a source
    Ok(out)
}
