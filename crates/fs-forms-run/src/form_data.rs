//! The records of a form as a module's service gives them: rows with values by CHANNEL name (`label`, `parent` ...), plus the
//! declaration of the module (which of its fields fill which channel). The blocks draw only by channel; no field name of a module is
//! known here. `FormRecords` is the [`Records`] a connected module gives (the starter builds it from the `FormSource` of the module).
//!
//! The board of the form `cards-in-columns` is drawn from the channels: `column` is the KEY of a column (the value as the module
//! gives it, never changed: a move names it), `column-label` its name, `order` the place, `progress` the bar, `facts` the facts of
//! the card (one value per field the module maps to the channel, in the order of the map; no value = the slot is kept, not drawn).
//! A column the module names but that holds no card ([`FormData::columns`]) is drawn too.
//!
//! Pattern: Value objects + Adapter (rows by channel in, the port `Records` out; a gesture out through the port `Writer`).

use std::sync::{Arc, PoisonError, RwLock};

use fs_components_fields::FieldSlot;
use fs_components_kanban::{Answer, Card, Column};
use fs_forms_bind::{Basic, ModuleForm};

use crate::records::{Records, SideItem};
use crate::write::{Invocation, Writer, allowed, args_of, command_key};

/// One record: a key and its values by channel (a channel may have several values; a record may leave a channel out).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormRow {
    /// The key of the record.
    pub key: String,
    /// The values by channel name.
    pub values: Vec<(String, Vec<String>)>,
    /// What the module lets the asking person do with this record: the keys of its commands (`card.move`). No statement = nothing.
    pub may: Vec<String>,
}

impl FormRow {
    /// A row with these `(channel, value)` pairs (one value each).
    #[must_use]
    pub fn of(key: &str, pairs: &[(&str, &str)]) -> Self {
        Self {
            key: key.to_owned(),
            values: pairs
                .iter()
                .map(|(c, v)| ((*c).to_owned(), vec![(*v).to_owned()]))
                .collect(),
            may: Vec::new(),
        }
    }

    /// The same row with the commands the person may use on it.
    #[must_use]
    pub fn allowing(mut self, keys: &[&str]) -> Self {
        self.may = keys.iter().map(|k| (*k).to_owned()).collect();
        self
    }

    /// The values of `channel` (none when the record leaves it out).
    #[must_use]
    pub fn all(&self, channel: &str) -> &[String] {
        self.values
            .iter()
            .find(|(c, _)| c == channel)
            .map_or(&[], |(_, v)| v.as_slice())
    }

    /// The first value of `channel`; empty values count as none.
    #[must_use]
    pub fn get(&self, channel: &str) -> Option<&str> {
        self.all(channel)
            .first()
            .map(String::as_str)
            .filter(|v| !v.is_empty())
    }
}

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

/// The bar of a card from the text of the channel `progress`: a number, cut to 0 to 100 (a text that is no number: no bar).
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is cut to 0..=100 first"
)]
fn percent(text: &str) -> Option<u8> {
    let p: f64 = text.trim().parse().ok()?;
    p.is_finite().then(|| p.clamp(0.0, 100.0).round() as u8)
}

/// The card of a row: title, bar and the facts in the order of the map of the channel `facts`.
fn card_of(data: &FormData, row: &FormRow, title: &str) -> Card {
    let mut card = Card::new(&row.key, title);
    card.progress = row.get("progress").and_then(percent);
    let names: &[String] = data
        .declared
        .map
        .iter()
        .find(|(c, _)| c == "facts")
        .map_or(&[], |(_, f)| f.as_slice());
    for (i, field) in names.iter().enumerate() {
        let basic = data
            .declared
            .fields
            .iter()
            .find(|(n, _)| n == field)
            .map_or(Basic::Text, |(_, b)| *b);
        let (block, label) = (block_of(basic), format!("fact.{field}"));
        let value = row.all("facts").get(i).map_or("", String::as_str);
        card = card.fact_or_more(if value.is_empty() {
            FieldSlot::unset(block, &label)
        } else {
            FieldSlot::new(block, &label, value)
        });
    }
    card
}

impl Records for FormRecords {
    /// The cards of the form `cards-in-columns`: the columns the module names (also empty ones), then the others in order of
    /// appearance. The key of a column is the value of the channel `column` unchanged; its name is `column-label` or the key.
    fn columns(&self) -> Vec<Column> {
        let forms = self.forms();
        let Some(data) = forms
            .iter()
            .find(|f| f.declared.form.name == "cards-in-columns")
        else {
            return Vec::new();
        };
        let mut out: Vec<Column> = data
            .columns
            .iter()
            .map(|(key, label)| Column::new(key, label))
            .collect();
        for row in &data.rows {
            let (Some(title), Some(column)) = (row.get("title"), row.get("column")) else {
                continue; // required channels: a row without them cannot be placed
            };
            let at = out.iter().position(|c| c.key == column).unwrap_or_else(|| {
                out.push(Column::new(
                    column,
                    row.get("column-label").unwrap_or(column),
                ));
                out.len() - 1
            });
            out[at].cards.push(card_of(data, row, title));
        }
        out
    }
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
        let refused = |code: &str| Answer::Refused(code.to_owned());
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
            Err(c) => return Answer::Refused(c.code),
        };
        let call = Invocation {
            form: form.to_owned(),
            command: command.to_owned(),
            record: record.map(str::to_owned),
            args,
        };
        if let Err(c) = writer.invoke(&call) {
            return Answer::Refused(c.code);
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
mod card_tests;
#[cfg(test)]
mod tests;
