//! Scheme list edge cases: the K2 fail-closed check refuses prefix hits, trimmed or empty schemes and a trailing colon.
use fs_forms_bind::testing::*;
use fs_forms_bind::*;

fn fill(basic: Basic, schemes: Option<Vec<&str>>) -> FieldFill {
    FieldFill {
        path: vec!["homepage".into()],
        role: Role::Input,
        basic,
        field: field("homepage", basic),
        block: "field-link".into(),
        schemes: schemes.map(|v| v.into_iter().map(str::to_owned).collect()),
        input: InputState::Open,
    }
}

#[test]
fn k2_edges_are_refused() {
    let f = fill(Basic::Link, Some(vec!["https", "mailto"]));
    for s in [
        " https",
        "https ",
        "https:",
        "HTTPS:",
        "httpsx",
        "http",
        "",
        "\u{0443}https",
        "j\u{0430}vascript",
        "\u{0455}sh",
        "JavaScript:",
        "java\nscript",
        "javascript",
        "JAVASCRIPT",
        "mail to",
        "mailto:",
        "\u{212A}https",
    ] {
        assert!(!f.allows_scheme(s), "{s:?}");
    }
    for s in ["https", "HTTPS", "Https", "MAILTO"] {
        assert!(f.allows_scheme(s), "{s:?}");
    }
    for b in [Basic::Link, Basic::Image] {
        for l in [None, Some(vec![])] {
            for s in ["https", "", "javascript", " "] {
                assert!(!fill(b, l.clone()).allows_scheme(s), "{b:?} {l:?} {s:?}");
            }
        }
    }
    for b in [
        Basic::Text,
        Basic::Email,
        Basic::Reference,
        Basic::Number,
        Basic::Unknown,
    ] {
        assert!(
            !fill(b, Some(vec!["https"])).allows_scheme("https"),
            "{b:?}"
        );
    }
}

/// Documents: `allows_scheme` trusts the list; keeping `javascript` out of it is F1b's job (never-list).
#[test]
fn k2_list_is_trusted() {
    assert!(fill(Basic::Link, Some(vec!["javascript"])).allows_scheme("javascript"));
}
