//! The style of a child frontend (O2, G6): it runs in the process of its parent, so the style of the parent is the only one; a wish for
//! another style is said as the yellow line `style-of-parent`, an unknown name with the existing `style-unknown`. Also the line for a
//! window that was brought forward (`host-notice-raised`).
//!
//! Tests: G6 `apple` in an `admin` parent -> admin + line; `default`/none/same -> no line; unknown name -> existing line; the lines have
//! sentences in both languages; `Raised` gives `raised`, the other answers none.
//! Mutants (each run red, see BERICHT-s6-o2): M1 the child style wins -> `g6_*`; M2 no line for another style -> `g6_*`;
//! M3 unknown name gets `style-of-parent` -> `g6_unknown_*`; M4 `Raised` gives no line -> `raised_*`.

use fs_components_styles::StyleId;
use fs_forms_run::{WindowId, WindowOpened, in_parent, notice_of, words};

#[test]
fn g6_a_child_that_names_another_style_is_drawn_in_the_style_of_the_parent_with_a_yellow_line() {
    let c = in_parent(Some("apple"), StyleId::Admin);
    assert_eq!(c.style, StyleId::Admin);
    let n = c.notice.expect("a line");
    assert_eq!(n.code, "host-notice-style-of-parent");
    let args: Vec<_> = n
        .params
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    assert!(
        args.contains(&("wanted", "apple")) && args.contains(&("used", "admin")),
        "{args:?}"
    );
}

#[test]
fn g6_no_wish_or_the_same_style_is_no_line() {
    for word in [
        None,
        Some(""),
        Some("default"),
        Some(" admin "),
        Some("admin"),
    ] {
        let c = in_parent(word, StyleId::Admin);
        assert_eq!((c.style, c.notice), (StyleId::Admin, None), "{word:?}");
    }
    assert_eq!(in_parent(Some("apple"), StyleId::Apple).notice, None);
}

#[test]
fn g6_unknown_name_is_the_existing_line_not_the_new_one() {
    let c = in_parent(Some("no-such"), StyleId::Simple);
    assert_eq!(c.style, StyleId::Simple);
    assert_eq!(c.notice.expect("a line").code, "host-notice-style-unknown");
}

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
    for code in [
        "host-notice-raised",
        "host-notice-style-of-parent",
        "host-notice-open-unanswered",
    ] {
        let en = words(&fs_components_words::Locale::english()).plain(code);
        let de = words(&fs_components_words::Locale::parse("de").unwrap()).plain(code);
        assert!(
            !en.is_empty() && !de.is_empty() && en != de && en != code,
            "{code}"
        );
    }
}
