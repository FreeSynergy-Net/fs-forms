//! What the test data is: the board the wish `[sample]` of `tests/fixtures/sample/board-parity.toml` makes (five cards in three
//! columns, a card without an assignee, labels, gestures that move and add) and the groups of `groups-parity.toml` (a tree of two
//! depths, a member list, no finding of the form checks). The cases are read from the data (nothing is named here).
//!
//! Pattern: Characterization test (the facts of the generated data, stated once).

#![allow(clippy::struct_excessive_bools)] // a list of yes/no answers, compared as a whole

mod sample_source;

use fs_components_kanban::Answer;
use fs_forms_run::{FormData, FormRecords, FormRow, Records, form_problems, form_report};
use sample_source::{
    FORM, add_card, board, card, column, columns, groups, move_card, other_column, rows, rows_of,
};

/// What a source of board data shows, by the questions the old tests asked of it.
#[derive(Debug, PartialEq, Eq)]
struct Shape {
    cards: usize,
    columns: usize,
    without_assignee: bool,
    with_label: bool,
    is_sample: bool,
    gestures: Vec<&'static str>,
}

fn shape_of(records: &dyn Records) -> Shape {
    let rows = rows(records);
    let given = |r: &fs_forms_run::FormRow| r.all("assignee").iter().any(|v| !v.is_empty());
    Shape {
        cards: rows.len(),
        columns: columns(records).len(),
        without_assignee: rows.iter().any(|r| !given(r)),
        with_label: rows
            .iter()
            .any(|r| r.all("facts").iter().any(|f| !f.is_empty())),
        is_sample: records.is_sample(),
        gestures: ["move", "create"]
            .into_iter()
            .filter(|c| records.command_of(FORM, c).is_some())
            .collect(),
    }
}

#[test]
fn the_board_has_the_cases_the_tests_of_the_host_need() {
    let n = shape_of(&*board());
    // five cards in three columns, some have no assignee (the builder cannot make one WITH an assignee: card in the report of
    // package 5a), labels in the facts, sample data, the gestures move and create
    assert_eq!(
        n,
        Shape {
            cards: 5,
            columns: 3,
            without_assignee: true,
            with_label: true,
            is_sample: true,
            gestures: vec!["move", "create"],
        }
    );
}

#[test]
fn a_card_moves_to_another_column_and_a_card_is_added_by_the_bound_gestures() {
    let b = board();
    let (key, to) = (card(0), other_column(&card(0)));
    assert_eq!(move_card(&*b, &key, &to), Answer::Accepted);
    let at = |records: &dyn Records, k: &str| {
        rows(records)
            .iter()
            .find(|r| r.key == k)
            .and_then(|r| r.get("column").map(str::to_owned))
    };
    assert_eq!(at(&*b, &key), Some(to), "the card is in its new column");
    let before = rows(&*b).len();
    assert_eq!(add_card(&*b, &column(0), "a new card"), Answer::Accepted);
    assert_eq!(rows(&*b).len(), before + 1);
    assert!(
        rows(&*b)
            .iter()
            .any(|r| r.get("title") == Some("a new card"))
    );
}

#[test]
fn a_move_of_an_unknown_card_or_into_no_column_is_refused_and_the_data_is_sound() {
    let b = board();
    for (key, to) in [("NOPE", "nowhere"), ("NOPE", "")] {
        assert_ne!(move_card(&*b, key, to), Answer::Accepted);
    }
    let to = column(0);
    assert_ne!(move_card(&*b, "NOPE", &to), Answer::Accepted);
    assert!(
        form_report(&fs_forms_run::FormRecords {
            forms: b.forms().to_vec(),
            ..fs_forms_run::FormRecords::default()
        })
        .is_empty(),
        "the form checks of the host find nothing wrong with the data"
    );
}

#[derive(Debug, PartialEq, Eq)]
struct GroupShape {
    forms: Vec<String>,
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

fn group_shape(forms: &[FormData]) -> GroupShape {
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
    GroupShape {
        forms: forms.iter().map(|f| f.declared.form.name.clone()).collect(),
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
fn the_groups_are_a_tree_of_two_depths_and_a_member_list() {
    let s = group_shape(&groups());
    assert_eq!(s.forms, ["tree", "rows"]);
    assert_eq!(
        s.tree_depths,
        [1, 1, 1],
        "two chains, the deeper one has three levels or more"
    );
    assert!(s.parents_known && s.every_node_named, "{s:?}");
    assert_eq!(s.members, 5);
    assert!(
        s.members_with_level && s.members_without_level && s.every_member_has_role,
        "{s:?}"
    );
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
