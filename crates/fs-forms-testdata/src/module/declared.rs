//! What the `writes` column of a form says about its commands: the one place that reads it.
//!
//! Pattern: Interpreter of the declaration (no `match` on a command name: the shape of `writes` decides) + Value object.

use fs_interfaces::forms::{Art, Channel, Form};

use crate::project::data_channels;

/// How a command runs, read from its declaration.
#[derive(Debug, PartialEq, Eq)]
pub enum Shape {
    /// No `writes` column: the declaration does not say what it writes.
    Unknown,
    /// An empty `writes` column (`open` and `remove` look the same): nothing to write.
    Nothing,
    /// It writes these channels and makes a record: they cover every required data channel of the form.
    Creates(Vec<(String, bool)>),
    /// It writes these channels of an existing record.
    Changes(Vec<(String, bool)>),
}

/// The command channel `name` of a form.
#[must_use]
pub fn command<'a>(form: &'a Form, name: &str) -> Option<&'a Channel> {
    form.channel(name)
        .filter(|c| c.arts.first() == Some(&Art::Command))
}

/// The shape of a command channel.
#[must_use]
pub fn shape(form: &Form, channel: &Channel) -> Shape {
    match channel.writes.as_deref() {
        None => Shape::Unknown,
        Some([]) => Shape::Nothing,
        Some(list) => {
            let writes: Vec<(String, bool)> = list
                .iter()
                .map(|w| (w.channel.clone(), w.required))
                .collect();
            let needed: Vec<&str> = data_channels(form)
                .into_iter()
                .filter(|n| form.channel(n).is_some_and(|c| c.required))
                .collect();
            let covers =
                !needed.is_empty() && needed.iter().all(|n| writes.iter().any(|w| w.0 == *n));
            if covers {
                Shape::Creates(writes)
            } else {
                Shape::Changes(writes)
            }
        }
    }
}

/// The commands a person may be offered on a record (`changes`) or on the set (`creates`): only those that have an effect here.
#[must_use]
pub fn offered(form: &Form) -> (Vec<String>, Vec<String>) {
    let (mut on_record, mut on_set) = (Vec::new(), Vec::new());
    for c in form
        .channels
        .iter()
        .filter(|c| c.arts.first() == Some(&Art::Command))
    {
        match shape(form, c) {
            Shape::Changes(_) => on_record.push(c.name.clone()),
            Shape::Creates(_) => on_set.push(c.name.clone()),
            Shape::Unknown | Shape::Nothing => {}
        }
    }
    (on_record, on_set)
}
