//! The way of a gesture to the module (SW-4a): a gesture is a channel of the form (`move`), a record and values by channel name
//! (`column`, `order`); this module turns it into ONE call of the module: the key of its command, the arguments by the field
//! names of the module. The module answers, the host reads the form again. The host holds no program name and no field name: the
//! channels and what each command writes are in the form catalog (`forms.txt`, column `writes`), the binding of channel to command
//! key and to field is in the declaration of the module (`map`), what the person may do is in `may`.
//!
//! The rule for the arguments (one function, [`args_of`]): for every channel the command writes, the value of the gesture is taken;
//! without one, only when the gesture does not name the channel at all, the current value of the record (its context, e.g. `lane`);
//! a channel the gesture names without a value (the place `order` of a drop at the end) is left out, NEVER filled from the record (the old
//! place would be sent again); a channel that has neither is left out. A channel the
//! gesture must deliver (`!`) without a value ends the call BEFORE it is sent (`form-write-missing`): the module is never asked.
//!
//! Pattern: Port (`Writer`: invoke and read again) + pure functions (the rule); a refusal is a code, never a panic.

use fs_interfaces::forms::{Art, Form, catalog};

use crate::form_data::{FormData, FormRow};
use crate::view::Coded;

/// One call of a command of a module: what the port [`Writer`] sends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invocation {
    /// The form the gesture is on (`cards-in-columns`).
    pub form: String,
    /// The key of the command in the contract (`card.move`), as the module declares it in its `map`.
    pub command: String,
    /// The record the gesture is on; `None` for a command without one (create).
    pub record: Option<String>,
    /// The arguments: the field of the module (`column`, `index`) and the value as text, in the order of the form.
    pub args: Vec<(String, String)>,
}

/// The connection to the module that takes a call and gives the forms again. The host that connects the module (the starter)
/// implements it; this crate links no module client.
pub trait Writer: Send + Sync + std::fmt::Debug {
    /// Sends one call to the module.
    ///
    /// # Errors
    /// The code of the refusal or of the failure (`host-notice-invoke-refused`, `form-source-bad-arguments`, ...).
    fn invoke(&self, call: &Invocation) -> Result<(), Coded>;

    /// Reads all forms of the module again (after an accepted call).
    ///
    /// # Errors
    /// The code of the failure.
    fn reload(&self) -> Result<Vec<FormData>, Coded>;
}

/// The form the module declares, in the catalog: the same name and the same `major.minor` (the highest patch). Two versions of one
/// form are in the catalog (`1.0.0` and `1.1.0`); the one the module keeps decides what a command writes.
fn definition(data: &FormData) -> Option<&'static Form> {
    let (name, major, minor) = (
        &data.declared.form.name,
        u64::from(data.declared.form.major),
        u64::from(data.declared.form.minor),
    );
    catalog()
        .iter()
        .filter(|f| f.name == *name && f.version.major == major && f.version.minor == minor)
        .max_by_key(|f| f.version.patch)
}

/// The field of the module a channel is bound to (first entry of its `map`), `None` = not bound.
fn bound<'a>(data: &'a FormData, channel: &str) -> Option<&'a str> {
    data.declared
        .map
        .iter()
        .find(|(c, _)| c == channel)
        .and_then(|(_, f)| f.first())
        .map(String::as_str)
}

/// The key of the command a channel of the form is bound to (`move` -> `card.move`), only for a channel of art `command`.
#[must_use]
pub fn command_key<'a>(data: &'a FormData, channel: &str) -> Option<&'a str> {
    let ch = definition(data)?.channel(channel)?;
    ch.arts.contains(&Art::Command).then_some(())?;
    bound(data, channel)
}

/// The arguments of the call of `channel` for the gesture `values` (channel name and text), by the rule in the head of this module.
///
/// # Errors
/// `form-write-missing{channel}` when a channel the gesture must deliver has no value; `form-write-unmapped{channel}` when a
/// channel to be written is not bound to a field of the module (the call is then not possible at all).
pub fn args_of(
    data: &FormData,
    channel: &str,
    record: Option<&FormRow>,
    values: &[(String, String)],
) -> Result<Vec<(String, String)>, Coded> {
    let writes = definition(data)
        .and_then(|f| f.channel(channel))
        .and_then(|c| c.writes.as_ref())
        .ok_or_else(|| Coded::new("form-write-unmapped").with("channel", channel))?;
    let mut out = Vec::new();
    for w in writes {
        // a channel the gesture names is the gesture's alone (an empty value: carried without a value, e.g. no place = the end);
        // only a channel it never names takes the context of the record (`lane`). The place must never come from the record.
        let value = match values.iter().find(|(c, _)| *c == w.channel) {
            Some((_, v)) => Some(v.as_str()).filter(|v| !v.is_empty()),
            None => record.and_then(|r| r.get(&w.channel)),
        };
        let Some(value) = value else {
            if w.required {
                return Err(Coded::new("form-write-missing").with("channel", w.channel.clone()));
            }
            continue;
        };
        let field = bound(data, &w.channel)
            .ok_or_else(|| Coded::new("form-write-unmapped").with("channel", w.channel.clone()))?;
        out.push((field.to_owned(), value.to_owned()));
    }
    Ok(out)
}

/// Whether the call of `channel` on `record` may be tried now, by the conditions of the schnitt: (ii) the channel is a command of
/// the form and bound to a key by the module, (iii) the key is in the command set of the module, (iv) the module says the person
/// may (`may` of the record, or of the page for a call without a record). Condition (i), that the description has an `[[action]]`
/// with this key, is asked by the caller that knows the description. A missing statement is `false`.
///
/// Without a record the page says (create); a command that acts on a record (move) is "may be tried" when ONE record allows it:
/// the question of the whole board (review v3-sw2) is answered so, the exact answer is asked per record when the gesture has one.
#[must_use]
pub fn allowed(data: &FormData, channel: &str, record: Option<&str>) -> bool {
    let Some(key) = command_key(data, channel) else {
        return false;
    };
    if !data.commands.iter().any(|c| c == key) {
        return false;
    }
    let says = |may: &[String]| may.iter().any(|m| m == key);
    match record {
        Some(r) => data
            .rows
            .iter()
            .find(|row| row.key == r)
            .is_some_and(|row| says(&row.may)),
        // `create` comes only from the page: a row's `may` never opens it
        None => {
            says(&data.may) || (channel != "create" && data.rows.iter().any(|row| says(&row.may)))
        }
    }
}

#[cfg(test)]
pub(crate) mod fixture;
#[cfg(test)]
mod tests;
