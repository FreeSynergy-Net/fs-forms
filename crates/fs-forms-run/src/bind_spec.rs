//! The block of the contract as bind reads it (`fs_forms_bind::BlockSpec`): the slots with their counts and, for a grouping slot, the
//! forms its heads come as (`heads = ["rows@^1.0.0"]`, from the real catalog, never written here), and the forms a module declares as
//! bind's `ProviderFacts::forms` wants them.
//!
//! Pattern: Adapter (the catalog's `BlockSpec` in, bind's `BlockSpec` out); the version rule is the one of `form_check::drawn`.

use fs_components_catalog::SlotKind;
use fs_forms_bind::{BlockSpec, ModuleForm, SlotSpec};

use crate::form_check::drawn;
use crate::records::Records;

/// The block `name` with its slots, or `None` for a block the contract does not know.
#[must_use]
pub fn block_spec(name: &str) -> Option<BlockSpec> {
    let b = crate::blocks::contract().block(name)?;
    Some(BlockSpec {
        name: b.name.clone(),
        slots: b
            .accepts
            .iter()
            .map(|s| SlotSpec {
                name: s.name.clone(),
                min: u32::from(s.required),
                max: Some(s.repeat.max(1)),
                heads: if s.kind == SlotKind::Grouping {
                    s.heads.iter().filter_map(drawn).collect()
                } else {
                    Vec::new()
                },
            })
            .collect(),
    })
}

/// The forms the connected module declares, each with its data type (the `type` of its `forms.json` entry): what the host answers
/// `ProviderFacts::forms` with. Two entries of the same form (`rows` for `Lane`, `rows` for `Task`) stay two entries.
#[must_use]
pub fn module_forms(records: &dyn Records) -> Vec<ModuleForm> {
    records
        .forms()
        .iter()
        .map(|d| ModuleForm {
            data_type: if d.declared.data_type.is_empty() {
                d.data_type.clone()
            } else {
                d.declared.data_type.clone()
            },
            ..d.declared.clone()
        })
        .collect()
}
