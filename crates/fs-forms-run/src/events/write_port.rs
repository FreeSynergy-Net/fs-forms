//! The write port of the board (`FormWrite` of fs-components): the component says WHAT the reader did in the words of the form
//! channels (`FormEvents` of the kanban crate: `move`, `create` and what each carries), this port asks the records (`can`) and
//! sends the gesture to the module (`send`, [`Records::invoke`]); after an accepted call the board is drawn again. The host knows
//! no gesture of the board and no channel name: what a gesture carries is read from the component itself ([`carried`]).
//!
//! A channel the component carries but gives no value (a drop at the end of a column has no `order`) is handed on as "carried,
//! without value" (empty text): the rule of [`crate::write::args_of`] then never fills it from the record (SW-4a, r1).
//!
//! Pattern: Adapter (`FormWrite` over `Records`) + Port (`BoardSource` of the records).

use std::sync::{Arc, Mutex, PoisonError};

use fs_components_core::form_write::{FormWrite, WriteArg, WriteDone, WriteRefused, WriteValue};
use fs_components_kanban::{Answer, BoardEvents, BoardSource, Column, FormEvents, Operation};

use super::Ask;
use crate::records::Records;

/// The records as the source of the board (the columns and what the old way can do); the written gestures go through [`BoardWrite`].
pub(crate) struct RecordsSource(pub Arc<dyn Records>);

impl BoardSource for RecordsSource {
    fn columns(&self) -> Vec<Column> {
        self.0.columns()
    }
    fn supports(&self, op: Operation) -> bool {
        self.0.can(op)
    }
}

/// The write port of one board on one form.
pub(crate) struct BoardWrite {
    records: Arc<dyn Records>,
    ask: Arc<Ask>,
    form: &'static str,
}

impl BoardWrite {
    pub fn new(records: Arc<dyn Records>, ask: Arc<Ask>, form: &'static str) -> Self {
        Self { records, ask, form }
    }
}

/// The text of a value as the module takes it (a whole number without a point).
fn text_of(value: &WriteValue) -> String {
    match value {
        WriteValue::Text(t) | WriteValue::Time(t) => t.clone(),
        WriteValue::Number(n) if n.fract() == 0.0 => format!("{n:.0}"),
        WriteValue::Number(n) => n.to_string(),
        WriteValue::Flag(f) => f.to_string(),
    }
}

/// A port that only records the channels a gesture names.
struct Probe(Mutex<Vec<String>>);

impl FormWrite for Probe {
    fn can(&self, _channel: &str, _record: Option<&str>) -> bool {
        true
    }
    fn send(
        &self,
        _channel: &str,
        _record: Option<&str>,
        args: &[WriteArg],
    ) -> Result<WriteDone, WriteRefused> {
        let mut names = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        names.extend(args.iter().map(|a| a.name.clone()));
        Ok(WriteDone::default())
    }
}

/// The channels the component carries in its gestures (a move with a place, a new card): read from the component, never listed here.
fn carried() -> Vec<String> {
    let probe = Arc::new(Probe(Mutex::default()));
    let events = FormEvents::new(probe.clone());
    let _ = events.move_card_at("", "", "", Some(0));
    let _ = events.add_card("", "");
    let mut names = probe
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    names.sort();
    names.dedup();
    names
}

impl FormWrite for BoardWrite {
    fn can(&self, channel: &str, record: Option<&str>) -> bool {
        self.records.can_channel(self.form, channel, record)
    }

    fn send(
        &self,
        channel: &str,
        record: Option<&str>,
        args: &[WriteArg],
    ) -> Result<WriteDone, WriteRefused> {
        let mut values: Vec<(String, String)> = args
            .iter()
            .map(|a| (a.name.clone(), text_of(&a.value)))
            .collect();
        for name in carried() {
            if !values.iter().any(|(c, _)| *c == name) {
                values.push((name, String::new()));
            }
        }
        match self.records.invoke(self.form, channel, record, &values) {
            Answer::Accepted => {
                self.ask.live.refresh_now();
                Ok(WriteDone::default())
            }
            Answer::Refused(code) => Err(WriteRefused::new(&code)),
        }
    }
}
