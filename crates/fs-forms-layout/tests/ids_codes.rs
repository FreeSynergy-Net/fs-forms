//! K59 (codes), K62 (element ids), natural ids.
#![allow(clippy::assert_is_empty)] // tests: plain asserts read best
use fs_forms_layout::*;

#[test]
fn k62_elem_id_rule() {
    let ok = ["a", "block:x/y", &"a".repeat(128)];
    for t in ok {
        assert!(ElemId::parse(t).is_ok(), "{t}");
    }
    let bad = ["", "-a", "A", "a b", &"a".repeat(129), "\u{e4}"];
    for t in bad {
        assert_eq!(
            ElemId::parse(t).unwrap_err().code,
            LayoutCode::ElemInvalid,
            "{t}"
        );
    }
}

#[test]
fn natural_ids_are_deterministic_and_valid() {
    let a = ElemId::natural(ElemKind::Block, &["board", "main"]);
    assert_eq!(a.as_str(), "block:board/main");
    assert_eq!(a, ElemId::natural(ElemKind::Block, &["board", "main"]));
    assert!(ElemId::parse(ElemId::natural(ElemKind::Message, &["Key X", "de"]).as_str()).is_ok());
}

#[test]
fn k59_codes_are_unique_kebab_and_round_trip() {
    assert_eq!(LayoutCode::ALL.len(), 45);
    let mut seen = std::collections::HashSet::new();
    for c in LayoutCode::ALL {
        let name = c.as_str();
        assert!(seen.insert(name), "{name} twice");
        assert!(
            name.chars().all(|x| x.is_ascii_lowercase() || x == '-'),
            "{name}"
        );
        assert_eq!(c.message_key(), format!("layout-error-{name}"));
        assert_eq!(LayoutCode::parse(name), Some(*c));
    }
}

#[test]
fn coded_prints_code_and_parameters() {
    let c = Coded::new(LayoutCode::RevChanged)
        .with("id", "x")
        .with("expected", 1);
    assert_eq!(c.to_string(), "layout-error-rev-changed id=x expected=1");
}

/// K59: texts in German and English for EVERY code, nothing extra, every `{$name}` is closed, and the new codes are at the end.
#[test]
fn k59_every_code_has_a_german_and_an_english_text() {
    let keys = |text: &str| -> Vec<String> {
        text.lines()
            .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
            .map(|l| l.split(" = ").next().unwrap().to_owned())
            .collect()
    };
    let want: Vec<String> = LayoutCode::ALL.iter().map(|c| c.message_key()).collect();
    assert_eq!(keys(TEXTS_DE), want);
    assert_eq!(keys(TEXTS_EN), want);
    for line in TEXTS_DE
        .lines()
        .chain(TEXTS_EN.lines())
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
    {
        let text = line.split_once(" = ").unwrap().1;
        assert!(!text.trim().is_empty());
        assert_eq!(
            text.matches('{').count(),
            text.matches('}').count(),
            "{line}"
        );
    }
    let last_two: Vec<&str> = LayoutCode::ALL[43..].iter().map(|c| c.as_str()).collect();
    assert_eq!(last_two, ["menu-invalid", "source-unknown"]); // append only
}

/// Q7: the parameters named in the texts are the parameters of the code: the same set in German, in English and in the doc comment
/// of the code in `codes.rs`.
#[test]
fn k59_texts_name_the_parameters_of_their_code() {
    let source = include_str!("../src/codes.rs");
    let braces = |text: &str| -> std::collections::BTreeSet<String> {
        text.split("{$")
            .skip(1)
            .map(|r| r.split('}').next().unwrap().to_owned())
            .collect()
    };
    let mut documented = std::collections::BTreeMap::new();
    let mut pending: Option<String> = None;
    for line in source.lines().map(str::trim) {
        if let Some(rest) = line.strip_prefix("/// `{") {
            pending = Some(rest.split('}').next().unwrap().to_owned());
        } else if let (Some(p), Some((_, name))) = (&pending, line.split_once(" => \"")) {
            let key = format!("layout-error-{}", name.trim_end_matches("\","));
            let set = p
                .split(',')
                .map(|x| x.trim().split('=').next().unwrap().to_owned())
                .filter(|x| !x.is_empty())
                .collect();
            documented.insert(key, set);
            pending = None;
        }
    }
    assert_eq!(documented.len(), LayoutCode::ALL.len());
    for (lang, texts) in [("de", TEXTS_DE), ("en", TEXTS_EN)] {
        for line in texts
            .lines()
            .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        {
            let (key, text) = line.split_once(" = ").unwrap();
            assert_eq!(&braces(text), &documented[key], "{lang} {key}");
        }
    }
}
