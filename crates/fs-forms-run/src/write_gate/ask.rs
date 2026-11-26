//! A question of the records is a refusal with its own code: the gate asks nothing of the reader, so it must not hide the question
//! behind the silent "unsupported".
//!
//! Tests: records that bind the card move to a channel (the way back of the command) and answer the call (`invoke`) with `Answer::Ask`.
//!
//! Pattern: none (tests).

use std::sync::Arc;

use fs_components_core::form_write::{FormWrite, Gesture, WriteArg, WriteValue};
use fs_components_kanban::{Answer, Question};

use super::WriteGate;
use crate::records::{Records, SideItem};
use crate::view::ActionView;

/// Records that answer every move with a question (the module wants a yes first).
struct Asking;

impl Records for Asking {
    fn side(&self) -> Vec<SideItem> {
        Vec::new()
    }
    fn channel_of(&self, _form: &str, command: &str) -> Option<String> {
        (command == "card.move").then(|| "move".to_owned())
    }
    fn can_channel(&self, _form: &str, _channel: &str, _record: Option<&str>) -> bool {
        true
    }
    fn invoke(
        &self,
        _form: &str,
        _channel: &str,
        _record: Option<&str>,
        _values: &[(String, String)],
    ) -> Answer {
        Answer::Ask(Question::new("Really?"))
    }
}

#[test]
fn invoke_answers_with_ask_and_the_question_of_the_records_is_refused_with_its_own_code_not_as_unsupported()
 {
    let gate = WriteGate {
        records: Arc::new(Asking),
        actions: Arc::new(vec![ActionView {
            trigger: "drop".into(),
            at: Some("block:board/main".into()),
            command: "card.move".into(),
            open: None,
            confirm: false,
        }]),
        form: "cards-in-columns".into(),
        block: "board".into(),
        place: "block:board/main".into(),
        after: crate::after_write::AfterWrite::default(),
    };
    let column = WriteArg {
        name: "column".into(),
        value: WriteValue::Text("done".into()),
    };
    let refused = gate
        .fire(Gesture::new("drop"), Some("k"), &[column])
        .unwrap_err();
    assert_eq!(
        refused.code,
        fs_components_core::form_write::ASK_UNSUPPORTED
    );
}
