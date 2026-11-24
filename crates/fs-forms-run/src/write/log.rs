//! One log line per bound command: what the host sent (Host -> service) and what the service answered (-> answer).
//! The line names the provider, the form, the command key, the record and the argument keys (never the values), and the answer: `accepted`,
//! or `refused <code>`. It holds no program name and no field name of its own; everything in it comes from the call.
//!
//! Pattern: Decorator (a call and its answer become one text line; the one caller in `FormRecords::invoke` writes it) – deviation: nothing wrapped, a pure formatter.

use super::Invocation;
use crate::view::Coded;

/// The line for one call and its answer. `provider` is the one the service names (`planning`, `fake`); empty = `unknown`.
#[must_use]
pub fn line(provider: &str, call: &Invocation, answer: &Result<(), Coded>) -> String {
    let who = if provider.is_empty() {
        "unknown"
    } else {
        provider
    };
    let record = call.record.as_deref().unwrap_or("-");
    let args: Vec<&str> = call.args.iter().map(|(k, _)| k.as_str()).collect();
    let answer = match answer {
        Ok(()) => "accepted".to_owned(),
        Err(c) => format!("refused {}", c.code),
    };
    format!(
        "command {} form={} record={record} args=[{}]: host -> {who} -> {answer}",
        call.command,
        call.form,
        args.join(",")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call() -> Invocation {
        Invocation {
            form: "cards-in-columns".into(),
            command: "card.move".into(),
            record: Some("a".into()),
            args: vec![("column".into(), "doing".into())],
        }
    }

    #[test]
    fn an_accepted_call_names_host_service_and_answer() {
        let l = line("planning", &call(), &Ok(()));
        assert_eq!(
            l,
            "command card.move form=cards-in-columns record=a args=[column]: host -> planning -> accepted"
        );
    }

    #[test]
    fn a_refusal_carries_its_code() {
        let l = line("", &call(), &Err(Coded::new("host-notice-invoke-refused")));
        assert!(l.ends_with("host -> unknown -> refused host-notice-invoke-refused"));
    }
}
