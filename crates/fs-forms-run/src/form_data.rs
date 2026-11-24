//! The records of a form as a module's service gives them: rows with values by CHANNEL name (`label`, `parent` ...), plus the
//! declaration of the module (which of its fields fill which channel). The blocks draw only by channel; no field name of a module is
//! known here. `FormRecords` is the [`Records`] a connected module gives (the starter builds it from the `FormSource` of the module).
//!
//! The board of the form `cards-in-columns` is drawn from the channels: `column` is the KEY of a column (the value as the module
//! gives it, never changed: a move names it), `column-label` its name, `order` the place (read: the cards of a column stand in the sequence of `order`, cards without a number at the end), `progress` the bar, `facts` the facts of
//! the card (one value per field the module maps to the channel, in the order of the map; no value = the slot is kept, not drawn).
//! A column the module names but that holds no card ([`FormData::columns`]) is drawn too.
//!
//! Pattern: Value objects + Adapter (rows by channel in, the port `Records` out; a gesture out through the port `Writer`).

use std::sync::{Arc, PoisonError, RwLock};

use fs_components_core::form_read::{ChannelInfo, FieldInfo, FormInput};
use fs_components_core::states::Load;
use fs_components_kanban::Answer;
use fs_forms_bind::{Basic, ModuleForm};

use crate::records::{Records, SideItem};
use crate::write::{Invocation, Writer, allowed, args_of, command_key};

pub use fs_components_core::form_read::FormRow;

/// One form a module offers: its declaration and its rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormData {
    /// The form, the version the module keeps and the map channel to field (`catalog/forms.json` of the module).
    pub declared: ModuleForm,
    /// The rows.
    pub rows: Vec<FormRow>,
    /// What the person may do without a record (create): the keys of the commands (the `may` of the page).
    pub may: Vec<String>,
    /// The command set of the module (the keys of its `commands.json`): a command that is not in it is never tried.
    pub commands: Vec<String>,
    /// The columns the module names in their order as `(key, label)`, also the ones without a card (the host reads them from the
    /// description and the module). Columns of cards that are not named here follow in order of appearance.
    pub columns: Vec<(String, String)>,
}

impl FormData {
    /// Rows without statements about commands: nothing may be written (the controls stay locked).
    #[must_use]
    pub fn read_only(declared: ModuleForm, rows: Vec<FormRow>) -> Self {
        Self {
            declared,
            rows,
            may: Vec::new(),
            commands: Vec::new(),
            columns: Vec::new(),
        }
    }
}

/// The records of a connected module, by form.
#[derive(Clone, Debug, Default)]
pub struct FormRecords {
    /// The provider named by the service; `fake` is made-up data (the window then says "Sample data").
    pub provider: String,
    /// The forms as first read.
    pub forms: Vec<FormData>,
    /// The connection that takes a gesture to the module; `None` = nothing can be written.
    pub writer: Option<Arc<dyn Writer>>,
    /// The forms read again after an accepted call (replaces `forms` while set).
    pub fresh: Arc<RwLock<Option<Arc<Vec<FormData>>>>>,
    /// The code of the last failed reading after an accepted call (`host-notice-data-reload`): the module HAS written, the board shows
    /// the old state; `None` after a good reading. The notice waits for `fs-notification` (SW-4b).
    pub stale: Arc<RwLock<Option<String>>>,
}

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
        rows: shown_rows(data),
        channels,
        may: data.may.clone(),
        load: Load::Ready,
    }
}

/// The rows as shown: a value of a field of type `DateTime` is given as its display text (`due_text`), the others as the module gives them.
fn shown_rows(data: &FormData) -> Vec<FormRow> {
    let is_time = |field: &str| {
        data.declared
            .fields
            .iter()
            .any(|(n, b)| n == field && *b == Basic::DateTime)
    };
    let mut rows = data.rows.clone();
    for row in &mut rows {
        for (channel, values) in &mut row.values {
            let fields = data.declared.map.iter().find(|(c, _)| c == channel);
            for (value, field) in values.iter_mut().zip(fields.iter().flat_map(|(_, f)| f)) {
                if is_time(field) {
                    *value = crate::due_text::display(value);
                }
            }
        }
    }
    rows
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
    }
}

impl Records for FormRecords {
    fn side(&self) -> Vec<SideItem> {
        Vec::new()
    }
    fn is_sample(&self) -> bool {
        self.provider == "fake"
    }
    fn forms(&self) -> Arc<Vec<FormData>> {
        let fresh = self.fresh.read().unwrap_or_else(PoisonError::into_inner);
        fresh
            .clone()
            .unwrap_or_else(|| Arc::new(self.forms.clone()))
    }
    fn command_of(&self, form: &str, channel: &str) -> Option<String> {
        let forms = self.forms();
        let data = forms.iter().find(|f| f.declared.form.name == form)?;
        command_key(data, channel).map(str::to_owned)
    }
    fn can_channel(&self, form: &str, channel: &str, record: Option<&str>) -> bool {
        self.writer.is_some()
            && self
                .forms()
                .iter()
                .find(|f| f.declared.form.name == form)
                .is_some_and(|d| allowed(d, channel, record))
    }
    fn invoke(
        &self,
        form: &str,
        channel: &str,
        record: Option<&str>,
        values: &[(String, String)],
    ) -> Answer {
        use crate::records::refused;
        let Some(writer) = &self.writer else {
            return refused("unsupported");
        };
        if !self.can_channel(form, channel, record) {
            return refused("locked");
        }
        let forms = self.forms();
        let Some(data) = forms.iter().find(|f| f.declared.form.name == form) else {
            return refused("locked");
        };
        let row = record.and_then(|r| data.rows.iter().find(|row| row.key == r));
        let (Some(command), args) = (
            command_key(data, channel),
            args_of(data, channel, row, values),
        ) else {
            return refused("form-write-unmapped");
        };
        let args = match args {
            Ok(args) => args,
            Err(c) => return refused(&c.code),
        };
        let call = Invocation {
            form: form.to_owned(),
            command: command.to_owned(),
            record: record.map(str::to_owned),
            args,
        };
        let answer = writer.invoke(&call);
        eprintln!(
            "fs-forms-run: {}",
            crate::write::log::line(&self.provider, &call, &answer)
        );
        if let Err(c) = answer {
            return refused(&c.code);
        }
        // accepted: the module is read again, the board shows what it now says (nothing is guessed here)
        if let Ok(all) = writer.reload() {
            *self.fresh.write().unwrap_or_else(PoisonError::into_inner) = Some(Arc::new(all));
            *self.stale.write().unwrap_or_else(PoisonError::into_inner) = None;
        } else {
            // the call was accepted, the module has written: this is no refusal; the old state stays and the failure is kept
            *self.stale.write().unwrap_or_else(PoisonError::into_inner) =
                Some("host-notice-data-reload".to_owned());
        }
        Answer::Accepted
    }
    fn stale(&self) -> Option<String> {
        self.stale
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

#[cfg(test)]
mod tests;
