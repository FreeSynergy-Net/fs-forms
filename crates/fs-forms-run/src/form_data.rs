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

use fs_types::describe::TypeDescription;
use std::sync::{Arc, PoisonError, RwLock};

use fs_components_kanban::Answer;
use fs_forms_bind::ModuleForm;

use crate::clock::{Clock, SystemClock};
use crate::records::{Records, SideItem};
use crate::view::Coded;
use crate::write::{Invocation, Writer, allowed, allowed_command, args_of, command_key};

pub use fs_components_core::form_read::FormRow;

/// One form a module offers: its declaration and its rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormData {
    /// The number of the source of the description these rows were read for (counts from 1, as `source` of a block); `0` = not said
    /// (old callers: such rows serve every source).
    pub source: u32,
    /// The data type of the records of that source (`type` of the `[[source]]`, `Lane`, `Task`); `""` = not said.
    pub data_type: String,
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
    /// What the module says of the TYPE of these records (`catalog/types.bin`, read by the starter): the word id of each field.
    /// `None` = it says nothing (as before: the head of a field is its name).
    pub type_description: Option<TypeDescription>,
}

impl FormData {
    /// Rows without statements about commands: nothing may be written (the controls stay locked).
    #[must_use]
    pub fn read_only(declared: ModuleForm, rows: Vec<FormRow>) -> Self {
        Self {
            source: 0,
            data_type: String::new(),
            declared,
            rows,
            may: Vec::new(),
            commands: Vec::new(),
            columns: Vec::new(),
            type_description: None,
        }
    }

    /// The same data with what the module says of their type.
    #[must_use]
    pub fn with_type_description(mut self, description: TypeDescription) -> Self {
        self.type_description = Some(description);
        self
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
    /// A source of the description the host could not read, as `(source number, code)`: `form-source-unknown-scope{scope}` (the module
    /// does not know the scope), `host-notice-data-unreachable` ... The rest of the window runs; lanes whose heads are this source say it.
    pub problems: Vec<(u32, Coded)>,
    /// The clock of the data; `None` = the system clock. Sample data set the moment they were made (`FixedClock`).
    pub clock: Option<Arc<dyn Clock>>,
    /// The words of the module (`catalog/texts/`, read by the starter); `None` = none: the heads then stay the names of the fields.
    pub module_words: Option<Arc<dyn crate::module_words::ModuleWords>>,
}

impl Records for FormRecords {
    fn side(&self) -> Vec<SideItem> {
        Vec::new()
    }
    fn module_words(&self) -> Option<Arc<dyn crate::module_words::ModuleWords>> {
        self.module_words.clone()
    }
    fn clock(&self) -> Arc<dyn Clock> {
        self.clock.clone().unwrap_or_else(|| Arc::new(SystemClock))
    }
    fn is_sample(&self) -> bool {
        fs_interfaces::module::ProviderKind::of(&self.provider)
            == fs_interfaces::module::ProviderKind::Sample
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
        self.commit(writer, &call)
    }
    fn channel_of(&self, form: &str, command: &str) -> Option<String> {
        let forms = self.forms();
        let data = forms.iter().find(|f| f.declared.form.name == form)?;
        data.declared
            .map
            .iter()
            .map(|(c, _)| c.as_str())
            .find(|c| command_key(data, c) == Some(command))
            .map(str::to_owned)
    }
    fn can_command(&self, form: &str, command: &str, record: Option<&str>) -> bool {
        self.writer.is_some()
            && self
                .forms()
                .iter()
                .find(|f| f.declared.form.name == form)
                .is_some_and(|d| allowed_command(d, command, record, false))
    }
    fn invoke_command(&self, form: &str, command: &str, record: Option<&str>) -> Answer {
        use crate::records::refused;
        let Some(writer) = &self.writer else {
            return refused("unsupported");
        };
        if !self.can_command(form, command, record) {
            return refused("locked");
        }
        let call = Invocation {
            form: form.to_owned(),
            command: command.to_owned(),
            record: record.map(str::to_owned),
            args: Vec::new(),
        };
        self.commit(writer, &call)
    }
    fn stale(&self) -> Option<String> {
        self.stale
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
    fn source_problems(&self) -> Vec<(u32, Coded)> {
        self.problems.clone()
    }
}

impl FormRecords {
    /// Sends one call to the module and, when it is accepted, reads the forms again (the board shows what the module now says,
    /// nothing is guessed here). A refusal is its code.
    fn commit(&self, writer: &Arc<dyn Writer>, call: &Invocation) -> Answer {
        let answer = writer.invoke(call);
        eprintln!(
            "fs-forms-run: {}",
            crate::write::log::line(&self.provider, call, &answer)
        );
        if let Err(c) = answer {
            return crate::records::refused(&c.code);
        }
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
}

#[cfg(test)]
mod tests;

mod input;
pub use input::{empty_input, input_of};
