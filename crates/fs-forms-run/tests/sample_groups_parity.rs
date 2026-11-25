//! Package 5b, parity: the groups module made by the generator (`sample_source::groups`) answers the same questions as the old
//! hand-built `sample_groups()`: the same forms in the same order, the same channels mapped, a tree whose two chains have two
//! depths, every parent a known node, a member list in which some members have a level and some have none, and no finding of the
//! match (`form_report`, `problems`). It goes with the old side.
//!
//! Pattern: Characterization test (the old and the new source behind the same questions).

mod sample_source;

use fs_forms_run::{FormData, FormRecords, FormRow, form_problems, form_report, sample_groups};
use sample_source::{groups, rows_of};

#[derive(Debug, PartialEq, Eq)]
struct Shape {
    forms: Vec<String>,
    channels_kept: bool,
    tree_depths: Vec<usize>,
    parents_known: bool,
    every_node_named: bool,
    members: usize,
    members_with_level: bool,
    members_without_level: bool,
    every_member_has_role: bool,
}

fn depth(rows: &[FormRow], key: &str) -> usize {
    let mut at = key;
    let mut n = 1;
    while let Some(p) = rows
        .iter()
        .find(|r| r.key == at)
        .and_then(|r| r.get("parent"))
    {
        at = p;
        n += 1;
    }
    n
}

fn shape(forms: &[FormData], old: &[FormData]) -> Shape {
    let tree = rows_of(forms, "tree");
    let members = rows_of(forms, "rows");
    // the leaves of the chains give the depths: the deepest node of each root
    let mut tree_depths: Vec<usize> = tree
        .iter()
        .filter(|r| !tree.iter().any(|c| c.get("parent") == Some(r.key.as_str())))
        .map(|r| depth(tree, &r.key))
        .collect();
    tree_depths.sort_unstable();
    tree_depths.dedup();
    Shape {
        forms: forms.iter().map(|f| f.declared.form.name.clone()).collect(),
        channels_kept: old.iter().zip(forms).all(|(o, n)| {
            o.declared
                .map
                .iter()
                .all(|(c, _)| n.declared.map.iter().any(|(m, _)| m == c))
        }),
        tree_depths: vec![
            usize::from(tree_depths.first().is_some_and(|d| *d >= 2)),
            usize::from(tree_depths.last().is_some_and(|d| *d >= 3)),
            usize::from(tree_depths.len() >= 2),
        ],
        parents_known: tree.iter().all(|r| {
            r.get("parent")
                .is_none_or(|p| tree.iter().any(|o| o.key == p))
        }),
        every_node_named: tree.iter().all(|r| r.get("label").is_some()),
        members: members.len(),
        members_with_level: members.iter().any(|r| r.get("state").is_some()),
        members_without_level: members.iter().any(|r| r.get("state").is_none()),
        every_member_has_role: members.iter().all(|r| r.get("columns").is_some()),
    }
}

fn records(forms: Vec<FormData>) -> FormRecords {
    FormRecords {
        provider: "fake".into(),
        forms,
        ..FormRecords::default()
    }
}

#[test]
fn the_generated_groups_answer_what_the_hand_built_ones_did() {
    let (old, new) = (sample_groups(), groups());
    assert_eq!(shape(&old, &old), shape(&new, &old));
}

#[test]
fn the_generated_groups_fit_the_blocks_and_give_no_finding() {
    let r = records(groups());
    assert_eq!(form_report(&r), Vec::new());
    assert!(form_problems("tree", &r).is_empty() && form_problems("table", &r).is_empty());
}

#[test]
fn the_generated_groups_are_the_same_on_every_call() {
    assert_eq!(groups(), groups());
}
