//! The status lines of `--scope` and a window brought forward: `host-notice-raised` (`notice_of`), and the sentences of the new lines in both
//! languages.
//!
//! Mutant (run red, see BERICHT-s6-o2): M4 `Raised` gives no line -> `raised_*`.

use fs_forms_run::{WindowId, WindowOpened, notice_of, words};

#[test]
fn raised_says_so_and_the_other_answers_say_nothing() {
    let id = WindowId::of("net.freesynergy.Tasks", "board");
    let n = notice_of(&WindowOpened::Raised(id.clone())).expect("a line");
    assert_eq!(n.code, "host-notice-raised");
    assert!(n.params.iter().any(|(k, v)| k == "id" && v == id.as_str()));
    assert!(notice_of(&WindowOpened::New(id.clone())).is_none());
    assert!(notice_of(&WindowOpened::Switched { id, from: None }).is_none());
}

#[test]
fn the_new_lines_have_a_sentence_in_both_languages() {
    for code in ["host-notice-raised", "host-notice-open-unanswered"] {
        let en = words(&fs_components_words::Locale::english()).plain(code);
        let de = words(&fs_components_words::Locale::parse("de").unwrap()).plain(code);
        assert!(
            !en.is_empty() && !de.is_empty() && en != de && en != code,
            "{code}"
        );
    }
}
