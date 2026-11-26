//! The gestures of a place: a component says WHAT the reader did (`drop`, `menu`, `quick-add`, `activate`) and with which values (the
//! `args` of the slot in the contract); this file finds the `[[action]]` of the description for (this place, this trigger) and carries
//! it out: the way is gesture, action, command, then ONE of two ways. A slot WITH `args` goes through the channel of the form the
//! module binds the command to ([`Records::channel_of`], the arguments are named as the channels); a slot WITHOUT `args` calls the
//! command with the record only ([`Records::invoke_command`]), so a menu command the form has no channel for is still possible. No
//! channel name is known here and there is no fallback "the name of the channel is the command": what no action binds, or the
//! module binds to no channel, is refused `gesture-unbound{trigger}`.
//!
//! The ENTRY ([`Gesture::entry`], only at the slot `menu`) is the name of one line of the context menu; the name of an action is its
//! command (the description gives an action no other name). Every action of the place with that name runs; without an entry the one
//! action of the place and trigger is taken, several are `gesture-ambiguous`.
//!
//! Pattern: Command (the action is the command object, the gate the invoker); the choice of way is a Strategy over the slot's `args`.

use fs_components_core::form_write::{Gesture, WriteArg, WriteDone, WriteRefused, trigger};
use fs_components_kanban::Answer;
use fs_forms_layout::CatalogRules;

use super::{WriteGate, text_of};
use crate::view::ActionView;

/// How a gesture reaches the module.
enum Way {
    /// A channel of the form, with the arguments of the slot (named as the channels).
    Channel { channel: String, args: Vec<String> },
    /// A command key with the record only.
    Command(String),
}

impl WriteGate {
    /// The actions of this place for the gesture: none (or an entry no action carries, or an entry at a slot without entries) =
    /// `gesture-unbound`, several without an entry = `gesture-ambiguous`.
    fn actions_of(&self, gesture: Gesture<'_>) -> Result<Vec<&ActionView>, WriteRefused> {
        let here = self.actions.iter().filter(|a| {
            a.at.as_deref() == Some(self.place.as_str()) && a.trigger == gesture.trigger
        });
        let hits: Vec<&ActionView> = match gesture.entry {
            None => here.collect(),
            // only the slot `menu` has entries
            Some(_) if gesture.trigger != trigger::MENU => Vec::new(),
            Some(entry) => here.filter(|a| a.command == entry).collect(),
        };
        match hits.len() {
            0 => Err(WriteRefused::unbound(gesture.trigger)),
            1 => Ok(hits),
            _ if gesture.entry.is_some() => Ok(hits),
            _ => Err(WriteRefused::ambiguous(gesture.trigger)),
        }
    }

    /// The way of ONE action: from the action to the command, then by the `args` of the slot.
    fn way(&self, trigger: &str, action: &ActionView) -> Result<Way, WriteRefused> {
        // an action `open` is carried out by the owner of the windows, not by the records: refused with its own code, never silent
        if action.open.is_some() {
            return Err(WriteRefused::new("open-unsupported"));
        }
        let args = crate::blocks::Blocks::rules()
            .command_args(&self.block, trigger)
            .ok_or_else(|| WriteRefused::unbound(trigger))?;
        if args.is_empty() {
            return Ok(Way::Command(action.command.clone()));
        }
        let channel = self
            .records
            .channel_of(&self.form, &action.command)
            .ok_or_else(|| WriteRefused::unbound(trigger))?;
        Ok(Way::Channel { channel, args })
    }

    /// Every way of the gesture (one per action), or the first refusal.
    fn ways(&self, gesture: Gesture<'_>) -> Result<Vec<Way>, WriteRefused> {
        self.actions_of(gesture)?
            .into_iter()
            .map(|a| self.way(gesture.trigger, a))
            .collect()
    }

    /// May the gesture be used now (the way is there and the module says the person may).
    pub(super) fn may_gesture(
        &self,
        gesture: Gesture<'_>,
        record: Option<&str>,
    ) -> Result<(), WriteRefused> {
        for way in self.ways(gesture)? {
            let yes = match way {
                Way::Channel { channel, .. } => {
                    self.records.can_channel(&self.form, &channel, record)
                }
                // a slot without arguments acts on a record: without one there is nothing to call
                Way::Command(command) => {
                    record.is_some() && self.records.can_command(&self.form, &command, record)
                }
            };
            if !yes {
                return Err(WriteRefused::new("locked"));
            }
        }
        Ok(())
    }

    /// The gesture happened: ONE call of the module per action, or a refusal as a code.
    pub(super) fn gesture(
        &self,
        gesture: Gesture<'_>,
        record: Option<&str>,
        args: &[WriteArg],
    ) -> Result<WriteDone, WriteRefused> {
        self.may_gesture(gesture, record)?;
        for way in self.ways(gesture)? {
            let answer = match way {
                Way::Channel {
                    channel,
                    args: carried,
                } => {
                    let mut values: Vec<(String, String)> = args
                        .iter()
                        .map(|a| (a.name.clone(), text_of(&a.value)))
                        .collect();
                    // an argument of the slot the gesture gives no value (a drop at the end of a column has no `before`) is handed on
                    // as "carried, without value" (empty text): the rule of [`crate::write::args_of`] never fills it from the record
                    for name in carried {
                        if !values.iter().any(|(c, _)| *c == name) {
                            values.push((name, String::new()));
                        }
                    }
                    self.records.invoke(&self.form, &channel, record, &values)
                }
                Way::Command(command) => self.records.invoke_command(&self.form, &command, record),
            };
            self.answer(answer)?;
        }
        Ok(WriteDone::default())
    }
}

impl WriteGate {
    /// The answer of the records as the port wants it: accepted fires the observer, a refusal is its code, a question is its own code.
    pub(super) fn answer(&self, answer: Answer) -> Result<WriteDone, WriteRefused> {
        match answer {
            Answer::Accepted => {
                self.after.fire();
                Ok(WriteDone::default())
            }
            Answer::Refused(reason) => Err(WriteRefused::new(&reason.code)),
            // the gate asks no question of the reader: a question is a refusal with its own code, never a silent "unsupported"
            Answer::Ask(_) => Err(WriteRefused::new(
                fs_components_core::form_write::ASK_UNSUPPORTED,
            )),
        }
    }
}
