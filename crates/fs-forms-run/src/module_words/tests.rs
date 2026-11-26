//! The chain of the sentences: the host first, the module second, the key last.
//!
//! Pattern: Characterization test of a Chain of Responsibility (`SayChain`).

use fs_components_mf2::{Mf2Texts, Source};
use fs_components_words::Locale;

use super::*;

const MODULE: &[Source] = &[Source {
    language: "en",
    text: "person.emails = Email addresses\nchannel.label = Module says name\n",
}];

fn chain() -> SayChain {
    let module: Arc<dyn Words> = Arc::new(Mf2Texts::new(MODULE, &Locale::english()));
    let host = HashMap::from([("channel.label".to_owned(), "Name".to_owned())]);
    SayChain::new(host, Some(module))
}

#[test]
fn the_host_answers_first() {
    assert_eq!(chain().sentence("channel.label"), "Name");
}

#[test]
fn the_module_answers_what_the_host_has_not() {
    assert_eq!(chain().sentence("person.emails"), "Email addresses");
}

#[test]
fn nobody_having_a_sentence_the_key_stays() {
    assert_eq!(chain().sentence("person.nothing"), "person.nothing");
    let alone = SayChain::new(HashMap::new(), None);
    assert_eq!(alone.sentence("person.emails"), "person.emails");
}
