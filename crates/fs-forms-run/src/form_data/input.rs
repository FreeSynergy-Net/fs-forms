//! The form as a component is given it ([`input_of`]): the channels of the form with the fields the module maps to them.
//!
//! Pattern: Adapter (`FormData` -> `FormInput`, the one reading side of a component); what the module says of its type
//! (`FormData.type_description`) gives each field its word id.

use fs_components_core::form_read::{ChannelInfo, FieldInfo, FormInput};
use fs_components_core::states::Load;
use fs_forms_bind::Basic;

use super::FormData;

/// The building block that draws a fact of this basic type (`fs-components-fields`: the catalog of kinds); a type without a
/// component is drawn as its text.
fn block_of(basic: Basic) -> &'static str {
    match basic {
        Basic::Email => "field-email",
        Basic::Bool => "field-flag",
        Basic::Date => "field-date",
        Basic::DateTime => "field-date-time",
        Basic::Choice => "field-choice",
        Basic::Reference => "field-reference",
        Basic::Keywords => "field-keywords",
        Basic::Link => "field-link",
        _ => "field-text",
    }
}

/// The form as a component is given it: the channels with the fields the module maps to them (and the block that draws a value of the
/// field's type), the named columns as the choices of the channel `column`.
#[must_use]
pub fn input_of(data: &FormData) -> FormInput {
    let channels = data
        .declared
        .map
        .iter()
        .map(|(channel, fields)| ChannelInfo {
            name: channel.clone(),
            fields: fields
                .iter()
                .map(|name| FieldInfo {
                    name: name.clone(),
                    block: block_of(
                        data.declared
                            .fields
                            .iter()
                            .find(|(n, _)| n == name)
                            .map_or(Basic::Text, |(_, b)| *b),
                    )
                    .to_owned(),
                    label: label_of(data, name),
                })
                .collect(),
            choices: if channel == "column" {
                data.columns.clone()
            } else {
                Vec::new()
            },
        })
        .collect();
    let f = &data.declared.form;
    FormInput {
        form: format!("{}@{}.{}.0", f.name, f.major, f.minor),
        rows: data.rows.clone(),
        channels,
        may: data.may.clone(),
        load: Load::Ready,
        heads: None,
    }
}

/// Nothing to draw: the module does not offer the form (or the block names none).
#[must_use]
pub fn empty_input(form: &str) -> FormInput {
    FormInput {
        form: form.to_owned(),
        rows: Vec::new(),
        channels: Vec::new(),
        may: Vec::new(),
        load: Load::Ready,
        heads: None,
    }
}

/// The word id of the field `name` as the type description of the module gives it; empty = it gives none (the head is the name).
fn label_of(data: &FormData, name: &str) -> String {
    data.type_description
        .as_ref()
        .and_then(|t| t.fields.iter().find(|f| f.name == name))
        .map_or_else(String::new, |f| f.label.clone())
}
