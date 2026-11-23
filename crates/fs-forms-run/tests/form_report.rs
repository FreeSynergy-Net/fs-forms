//! SQ-4: `form_report` (what `--check` of a host shows) is the match of every declared form: green = nothing, a missing required
//! channel = red `form-channel-unmapped`, a form no component draws = yellow `form-no-component`.

use fs_forms_run::{FormData, FormRecords, form_report, sample_groups};

fn records(forms: Vec<FormData>) -> FormRecords {
    FormRecords {
        provider: "fake".into(),
        forms,
        ..FormRecords::default()
    }
}

#[test]
fn a_module_that_fits_gives_no_finding() {
    assert_eq!(form_report(&records(sample_groups())), Vec::new());
}

#[test]
fn a_required_channel_left_out_is_red_with_the_form_named() {
    let mut forms = sample_groups();
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
    let mut forms = sample_groups();
    forms[0].declared.form.name = "checklist".into();
    forms[0].declared.map = vec![
        ("items".into(), vec!["Group".into()]),
        ("done".into(), vec!["closed".into()]),
        ("label".into(), vec!["name".into()]),
    ];
    forms[0].declared.fields = vec![
        ("closed".into(), fs_forms_bind::Basic::Bool),
        ("name".into(), fs_forms_bind::Basic::Text),
    ];
    forms.truncate(1);
    let found = form_report(&records(forms));
    assert!(found.iter().all(|f| !f.red), "{found:?}");
    assert!(
        found.iter().any(|f| f.code.code == "form-no-component"),
        "{found:?}"
    );
}
