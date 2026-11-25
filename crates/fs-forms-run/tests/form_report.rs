//! SQ-4: `form_report` (what `--check` of a host shows) is the match of every declared form: green = nothing, a missing required
//! channel = red `form-channel-unmapped`, a form no component draws = yellow `form-no-component`.

mod sample_source;

use fs_forms_run::{FormData, FormRecords, form_report};
use sample_source::groups;

fn records(forms: Vec<FormData>) -> FormRecords {
    FormRecords {
        provider: "fake".into(),
        forms,
        ..FormRecords::default()
    }
}

#[test]
fn a_module_that_fits_gives_no_finding() {
    assert_eq!(form_report(&records(groups())), Vec::new());
}

#[test]
fn a_required_channel_left_out_is_red_with_the_form_named() {
    let mut forms = groups();
    forms[0].declared.map.retain(|(c, _)| c != "label");
    let found = form_report(&records(forms));
    let red: Vec<_> = found.iter().filter(|f| f.red).collect();
    assert!(
        red.iter().any(|f| f.code.code == "form-channel-unmapped"
            && f.code.params.contains(&("form".into(), "tree".into()))),
        "{found:?}"
    );
}

#[test]
fn a_form_no_component_draws_is_yellow_not_red() {
    let mut forms = groups();
    // `members` is in the form catalog but frozen (superseded by `rows`): no component draws it.
    forms[0].declared.form.name = "members".into();
    forms[0].declared.map = vec![
        ("records".into(), vec!["Group".into()]),
        ("person".into(), vec!["who".into()]),
        ("role".into(), vec!["role".into()]),
    ];
    forms[0].declared.fields = vec![
        ("who".into(), fs_forms_bind::Basic::Reference),
        ("role".into(), fs_forms_bind::Basic::Reference),
    ];
    forms.truncate(1);
    let found = form_report(&records(forms));
    assert!(found.iter().all(|f| !f.red), "{found:?}");
    assert!(
        found.iter().any(|f| f.code.code == "form-no-component"),
        "{found:?}"
    );
}
