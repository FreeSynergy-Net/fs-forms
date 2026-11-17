//! H-K11 (page gate) and H-K12 (hidden fields).
mod common;
use common::*;
use fs_forms_bind::*;

fn rec(values: &[(&str, &str)], hidden: &[(&str, HiddenReason)]) -> RecordView {
    RecordView {
        id: "r".into(),
        may: vec![],
        hidden: hidden.iter().map(|(p, r)| ((*p).to_owned(), *r)).collect(),
        values: values
            .iter()
            .map(|(p, v)| ((*p).to_owned(), (*v).to_owned()))
            .collect(),
    }
}

fn gate(page: RecordPage, size: usize) -> Admitted {
    admit_page(
        &Limits::default(),
        size,
        page,
        Part::Place {
            page: "board".into(),
            place: "main".into(),
        },
    )
}

fn refused(a: &Admitted, code: &str) {
    let Admitted::Refused(l) = a else {
        panic!("page must be refused: {a:?}")
    };
    assert_eq!(code_of(l), "bind-error-page-refused");
    assert_eq!(l.cause.coded().param("kind"), Some(code));
    assert_eq!(
        l.part.text(),
        "place:board/main",
        "the lock sits on the placement"
    );
}

#[test]
fn too_long() {
    let page = RecordPage {
        records: vec![rec(&[], &[]), rec(&[], &[]), rec(&[], &[])],
        ..RecordPage::default()
    };
    refused(&gate(page, 2), "records-error-page-too-long");
}

#[test]
fn hidden_with_value() {
    let page = RecordPage {
        records: vec![rec(
            &[("participants", "x")],
            &[("participants", HiddenReason::NoRight)],
        )],
        ..RecordPage::default()
    };
    refused(&gate(page, 2), "records-error-hidden-has-value");
}

#[test]
fn value_limit_is_inclusive() {
    let at = "x".repeat(65_536);
    let page = RecordPage {
        records: vec![rec(&[("title", at.as_str())], &[])],
        ..RecordPage::default()
    };
    assert!(matches!(gate(page, 2), Admitted::Page(_)));
    let over = "x".repeat(65_537);
    let page = RecordPage {
        records: vec![rec(&[("title", over.as_str())], &[])],
        ..RecordPage::default()
    };
    refused(&gate(page, 2), "records-error-value-too-large");
}

#[test]
fn estimate_mixed() {
    let page = RecordPage {
        records: vec![],
        total: Some(1),
        total_estimated: Some(2),
    };
    refused(&gate(page, 2), "records-error-estimate-mixed");
}

#[test]
fn clean_page_is_unchanged() {
    let page = RecordPage {
        records: vec![rec(&[("title", "A")], &[])],
        total: Some(1),
        total_estimated: None,
    };
    assert_eq!(gate(page.clone(), 2), Admitted::Page(page));
}

fn fill(path: &str) -> FieldFill {
    let f = bind_field(path);
    FieldFill {
        path: f.path.clone(),
        role: Role::Display,
        basic: f.basic,
        field: f,
        block: "field-text".into(),
        schemes: None,
        input: InputState::NotInput,
    }
}

fn bind_field(path: &str) -> FieldDesc {
    FieldDesc {
        path: vec![path.into()],
        basic: Basic::Text,
        target: None,
        read_only: false,
    }
}

#[test]
fn hidden_field_never_shows_a_value() {
    let r = rec(
        &[("title", "A")],
        &[
            ("participants", HiddenReason::NoRight),
            ("secret", HiddenReason::Sensitive),
        ],
    );
    assert_eq!(
        field_shown(&fill("participants"), &r),
        FieldShown::Hidden {
            reason: HiddenReason::NoRight
        }
    );
    assert_eq!(
        field_shown(&fill("secret"), &r),
        FieldShown::Hidden {
            reason: HiddenReason::Sensitive
        }
    );
    assert_eq!(field_shown(&fill("due"), &r), FieldShown::Empty);
    assert_eq!(
        field_shown(&fill("title"), &r),
        FieldShown::Value("A".into())
    );
    let note = field_shown(&fill("participants"), &r)
        .note("participants")
        .unwrap();
    assert_eq!(note.code, "bind-error-field-hidden");
    assert_eq!(note.param("reason"), Some("no-right"));
}
