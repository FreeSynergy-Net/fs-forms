//! The lanes the host matched (`placed`) reach the board component: `FormInput::heads` of the cards is `None` when no lane list is asked,
//! `Missing` when it is asked and not usable (the notice stays), `Rows` with the key and label `align` made otherwise; the component's
//! `FormSource::lane_plan` turns them into the bands. The decision stays in `lanes.rs`; nothing here decides again.
//!
//! Mutants: heads of the input ignored (`rows_become_bands_in_order_with_the_empty_one`, `missing_says_so_and_none_draws_no_lanes`).
//!
//! Pattern: none (tests).
mod lanes_support;

use std::sync::Arc;

use fs_components_core::form_read::{FormInput, HeadRow, Heads};
use fs_components_core::form_write::{FormWrite, WriteArg, WriteDone, WriteRefused};
use fs_components_fields::KeysAsText;
use fs_components_kanban::{BoardSource, FormSource, LaneCard, LaneHead, LanePlan, lanes_of};
use fs_forms_run::placed;
use lanes_support::{
    all_forms, card, cards_entry, codes, feeds, lane, lanes_rows, records, rows_entry,
};

struct Host;

impl FormWrite for Host {
    fn can(&self, _: &str, _: Option<&str>) -> bool {
        false
    }
    fn send(&self, _: &str, _: Option<&str>, _: &[WriteArg]) -> Result<WriteDone, WriteRefused> {
        Ok(WriteDone::default())
    }
}

/// What the board component makes of the input the host drew with.
fn plan_of(input: FormInput) -> LanePlan {
    FormSource::new(input, Arc::new(Host), Arc::new(KeysAsText)).lane_plan()
}

fn head(key: &str, label: &str) -> HeadRow {
    HeadRow {
        key: key.into(),
        label: label.into(),
    }
}

#[test]
fn rows_become_bands_in_order_with_the_empty_one() {
    let p = placed(&records(all_forms()), "board", &feeds());
    // the cards carry names: keys are names (the one decision of `align`), labels the display names; Cleo has no card
    assert_eq!(
        p.input.heads,
        Some(Heads::Rows(vec![
            head("Anna", "Anna"),
            head("Ben", "Ben"),
            head("Cleo", "Cleo")
        ]))
    );
    let LanePlan::On { grouping, heads } = plan_of(p.input) else {
        panic!("the component turns Rows into lanes");
    };
    assert_eq!(grouping, "assignee");
    let keys: Vec<_> = heads.iter().map(|h| h.key.as_str()).collect();
    assert_eq!(keys, ["Anna", "Ben", "Cleo"]);
}

#[test]
fn iri_keys_and_display_labels_pass_unchanged() {
    let mut forms = all_forms();
    forms[0] = cards_entry(vec![card("t1", "urn:p:anna")]);
    let p = placed(&records(forms), "board", &feeds());
    let Some(Heads::Rows(rows)) = p.input.heads else {
        panic!("rows");
    };
    assert_eq!(rows[0], head("urn:p:anna", "Anna"));
    assert_eq!(rows[2], head("urn:p:cleo", "Cleo"));
}

#[test]
fn missing_says_so_and_none_draws_no_lanes() {
    // asked, list not installed: Missing for the component AND the notice of the host stays
    let p = placed(&records(vec![all_forms().remove(0)]), "board", &feeds());
    assert_eq!(p.input.heads, Some(Heads::Missing));
    assert_eq!(codes(&p)[0], "host-notice-lanes-missing");
    assert_eq!(plan_of(p.input), LanePlan::Missing);
    // not asked: None, plain columns, nothing to say
    let mut f = feeds();
    f[0].heads = None;
    let p = placed(&records(all_forms()), "board", &f);
    assert_eq!(p.input.heads, None);
    assert_eq!(p.notices, vec![]);
    assert_eq!(plan_of(p.input), LanePlan::Off);
}

#[test]
fn one_decision_the_input_agrees_with_the_plan_of_the_host() {
    for forms in [all_forms(), vec![all_forms().remove(0)]] {
        let p = placed(&records(forms), "board", &feeds());
        let fed = plan_of(p.input.clone());
        assert_eq!(fed, p.plan);
    }
}

#[test]
fn a_card_with_two_people_stands_in_two_lanes_and_the_empty_lane_stays() {
    let mut forms = all_forms();
    let mut two = card("t1", "Anna");
    two.values.retain(|(c, _)| c != "assignee");
    two.values
        .push(("assignee".into(), vec!["Anna".into(), "Ben".into()]));
    forms[0] = cards_entry(vec![two]);
    forms[2] = rows_entry(2, "Lane", true, {
        let mut r = lanes_rows();
        r.push(lane("urn:p:dora", "Dora"));
        r
    });
    let p = placed(&records(forms), "board", &feeds());
    let LanePlan::On { grouping, heads } = plan_of(p.input.clone()) else {
        panic!("on");
    };
    let cards: Vec<LaneCard> = p
        .input
        .rows
        .iter()
        .map(|r| {
            let v: Vec<&str> = r.all("assignee").iter().map(String::as_str).collect();
            LaneCard::new(&r.key).with(&grouping, &v)
        })
        .collect();
    let lanes = lanes_of(&heads, &cards, &grouping);
    let at = |k: &str| lanes.iter().find(|l| l.key == k).map(|l| l.cards.clone());
    assert_eq!(at("Anna"), Some(vec!["t1".to_owned()]));
    assert_eq!(at("Ben"), Some(vec!["t1".to_owned()]));
    assert_eq!(at("Dora"), Some(Vec::new()), "the empty lane has a head");
    assert_eq!(heads[0], LaneHead::new("Anna", "Anna"));
}
