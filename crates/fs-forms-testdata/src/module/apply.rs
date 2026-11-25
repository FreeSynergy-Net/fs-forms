//! Running a command from the declaration: the arguments are checked against the `let channel = .find(|w| w.1 && !args.iter().any(|(p, _)| p == &w.0))rites.iter().find(|w| &w.0 == path).and_then(|w| form.channel(&w.0));rites` column and written into the record.
//!
//! Pattern: Command + Interpreter (the effect is the same for every command: write the arguments into the channels of `writes`;
//! no name is matched) + Memento (`revert` restores the values before). Failures are refusals that carry a `SampleCode`.

use fs_interfaces::forms::{Art, Channel};
use fs_interfaces::module::{
    ChangeKind, Conflict, FieldClash, Invocation, Origin, Refusal, StepRef,
};
use fs_types::coded::message_key;

use super::declared::{self, Shape};
use super::store::{Entry, Store, uuid_number};
use crate::error::SampleCode;
use crate::project::Values;

/// A refusal: the bare kind and status of the contract, the code of the failure as its detail.
pub fn refuse(kind: &str, status: u16, code: SampleCode) -> Refusal {
    let mut r = Refusal::default();
    kind.clone_into(&mut r.kind);
    r.status = status;
    r.detail = message_key(code);
    r
}

fn bad(what: &str) -> Refusal {
    let mut r = refuse("bad-arguments", 400, SampleCode::ArgumentBad);
    r.detail = format!("{}: {what}", r.detail);
    r
}

/// Whether `text` fits the kind of the channel.
fn fits(channel: &Channel, text: &str) -> bool {
    match channel.arts.first() {
        Some(Art::Time) => {
            <fs_types::DateTime as fs_types::BasicValue>::check(text).is_ok()
                || (text.len() == 10 && chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d").is_ok())
        }
        Some(Art::Number) => text.parse::<f64>().is_ok_and(f64::is_finite),
        Some(Art::Flag) => matches!(text, "true" | "false"),
        _ => true,
    }
}

fn with(values: &Values, args: &[(String, String)]) -> Values {
    let mut out: Values = values
        .iter()
        .filter(|(c, _)| !args.iter().any(|(a, _)| a == c))
        .cloned()
        .collect();
    out.extend(
        args.iter()
            .filter(|(_, t)| !t.is_empty())
            .map(|(c, t)| (c.clone(), vec![t.clone()])),
    );
    out
}

impl Store {
    /// Runs `inv`. `Ok` is the index of the entry in the log.
    ///
    /// # Errors
    /// The refusal of the contract: `unknown-command`, `unknown-record`, `bad-arguments`, `conflict`, `unsupported`.
    pub fn invoke(&mut self, who: &Origin, inv: &Invocation) -> Result<usize, Refusal> {
        let step = inv.step.clone().unwrap_or_default();
        if let Some(at) = self.done.get(&(step.run.clone(), step.step.clone(), false)) {
            return Ok(*at);
        }
        let record = inv.record.as_deref().filter(|r| !r.is_empty());
        let (t, rec_at) = match record {
            Some(r) => {
                let (t, i) = self
                    .find(r)
                    .ok_or_else(|| refuse("unknown-record", 404, SampleCode::RecordUnknown))?;
                (Some(t), Some(i))
            }
            None => (None, None),
        };
        let unknown = || refuse("unknown-command", 404, SampleCode::CommandUnknown);
        let table = match t {
            Some(t) => t,
            None => self
                .tables
                .iter()
                .position(|tb| declared::command(&tb.form, &inv.command).is_some())
                .ok_or_else(unknown)?,
        };
        let form = self.tables[table].form.clone();
        let channel = declared::command(&form, &inv.command).ok_or_else(unknown)?;
        let (writes, creates) = match declared::shape(&form, channel) {
            Shape::Unknown => return Err(refuse("unsupported", 501, SampleCode::WritesUnknown)),
            Shape::Nothing => return Err(refuse("unsupported", 501, SampleCode::EffectUnknown)),
            Shape::Creates(w) => (w, true),
            Shape::Changes(w) => (w, false),
        };
        if creates == record.is_some() {
            return Err(bad(if creates {
                "a create names no record"
            } else {
                "this command needs a record"
            }));
        }
        let args: Vec<(String, String)> = inv
            .args
            .iter()
            .map(|v| (v.path.clone(), v.text.clone()))
            .collect();
        for (i, (path, text)) in args.iter().enumerate() {
            let target = writes.iter().find(|w| &w.0 == path);
            let channel = target.and_then(|w| form.channel(&w.0));
            if channel.is_none_or(|c| !fits(c, text)) || args[..i].iter().any(|(p, _)| p == path) {
                return Err(bad(path));
            }
        }
        if let Some(w) = writes
            .iter()
            .find(|w| w.1 && !args.iter().any(|(p, _)| p == &w.0))
        {
            return Err(bad(&format!("{} is missing", w.0)));
        }
        let (object, rev, before) = match (t, rec_at) {
            (Some(t), Some(i)) => {
                let r = &self.tables[t].recs[i];
                (r.id.clone(), r.rev, Some(r.values.clone()))
            }
            _ => (self.free_id(table), 0, None),
        };
        if let Some(expect) = inv.expect.as_deref().filter(|e| !e.is_empty())
            && (before.is_none() || expect != rev.to_string())
        {
            return Err(self.clash(&object, &args, before.as_ref()));
        }
        let after = with(&before.clone().unwrap_or_default(), &args);
        let (id, at) = self.next_id();
        let entry = Entry {
            id,
            table,
            object,
            kind: if before.is_some() {
                ChangeKind::Update
            } else {
                ChangeKind::Create
            },
            rev: rev + 1,
            before_rev: before.is_some().then_some(rev),
            before,
            after: Some(after),
            origin: who.clone(),
            run: step.run.clone(),
            undoes: None,
            at,
        };
        Ok(self.record(entry, (&step.run, &step.step), false))
    }

    /// Takes the change `change` back (the whole unit).
    ///
    /// # Errors
    /// `unknown-change` 404, or `conflict` 409 when the record changed after it.
    pub fn revert(&mut self, who: &Origin, change: &str, step: &StepRef) -> Result<usize, Refusal> {
        if let Some(at) = self.done.get(&(step.run.clone(), step.step.clone(), true)) {
            return Ok(*at);
        }
        let target = uuid_number(change)
            .and_then(|n| self.log.iter().position(|e| e.id == n))
            .ok_or_else(|| refuse("unknown-change", 404, SampleCode::ChangeUnknown))?;
        let e = self.log[target].clone();
        let latest = self.latest(&e.object).cloned();
        if let Some(l) = latest.filter(|l| l.id != e.id) {
            let mut r = Refusal::conflict(
                &message_key(SampleCode::Stale),
                Conflict::new(l.change_ref(), Vec::new()),
            );
            r.status = 409;
            return Err(r);
        }
        let (id, at) = self.next_id();
        let entry = Entry {
            id,
            table: e.table,
            object: e.object.clone(),
            kind: ChangeKind::Undo,
            rev: e.rev + 1,
            before_rev: Some(e.rev),
            before: e.after.clone(),
            after: e.before.clone(),
            origin: who.clone(),
            run: step.run.clone(),
            undoes: Some(e.id),
            at,
        };
        Ok(self.record(entry, (&step.run, &step.step), true))
    }

    fn clash(&self, object: &str, args: &[(String, String)], before: Option<&Values>) -> Refusal {
        let fields: Vec<FieldClash> = args
            .iter()
            .map(|(p, mine)| {
                let now = before
                    .into_iter()
                    .flatten()
                    .find(|(c, _)| c == p)
                    .and_then(|(_, v)| v.first().cloned())
                    .unwrap_or_default();
                FieldClash::new(p, &now, mine)
            })
            .collect();
        match self.latest(object) {
            Some(by) => Refusal::conflict(
                &message_key(SampleCode::Stale),
                Conflict::new(by.change_ref(), fields),
            ),
            None => refuse("conflict", 409, SampleCode::Stale),
        }
    }
}

impl Store {
    /// The first key of the table that no record (alive or taken back) has: `urn:fs-sample:<form>:<n>`.
    fn free_id(&self, table: usize) -> String {
        let t = &self.tables[table];
        (1..=t.recs.len() + 1)
            .map(|n| format!("urn:fs-sample:{}:{n}", t.key))
            .find(|id| t.recs.iter().all(|r| &r.id != id))
            .unwrap_or_default()
    }
}
