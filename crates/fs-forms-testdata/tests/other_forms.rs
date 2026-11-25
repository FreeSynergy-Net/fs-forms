//! The same builder for other kinds of form: no program, no channel name in the code; plain AS2 content, the empty cases and trees.

use fs_forms_testdata::{Anchor, Case, Period, Relation, SampleBuilder, SampleSpec, SetWish};
use fs_types::Zone;

mod common;

fn anchor() -> Anchor {
    Anchor {
        period: Period::Week(2026, 22),
        zone: Zone::iana("Europe/Berlin").unwrap(),
    }
}

#[test]
fn a_board_gets_distinct_columns_and_empty_lanes() {
    let wish = SetWish::new("cards-in-columns@=1.0.0", "Task", 12)
        .distinct("column", 3)
        .distinct("lane", 2)
        .empty("facts", 2);
    let set = SampleBuilder::new(SampleSpec {
        seed: 7,
        anchor: anchor(),
        sets: vec![wish],
    })
    .build()
    .unwrap();
    let rows = set.rows("cards-in-columns@1.0.0").unwrap();
    assert_eq!(rows.len(), 12);
    let columns: std::collections::BTreeSet<_> =
        rows.iter().filter_map(|r| r.get("column")).collect();
    assert_eq!(columns.len(), 3);
    assert!(rows.iter().all(|r| r.get("title").is_some()));
    assert!(
        set.objects
            .iter()
            .all(|o| o.content_type == "application/activity+json"
                && o.envelope.object_type == "Task")
    );
    common::golden("board-week", &set.render());
}

#[test]
fn a_tree_has_chains_and_parents() {
    let wish = SetWish::new("tree@=1.0.0", "Group", 9).case(Case::Tree { depth: 3, roots: 2 });
    let spec = SampleSpec {
        seed: 3,
        anchor: anchor(),
        sets: vec![wish],
    };
    let set = SampleBuilder::new(spec)
        .relation(Relation::Parent {
            child: "parent".to_owned(),
        })
        .build()
        .unwrap();
    let rows = set.rows("tree@1.0.0").unwrap();
    assert_eq!(rows.len(), 9);
    let with_parent = rows.iter().filter(|r| r.get("parent").is_some()).count();
    assert!(
        with_parent >= 4,
        "two chains of three give four edges: {with_parent}"
    );
    assert!(
        set.objects
            .iter()
            .filter(|o| o.envelope.in_reply_to.is_some())
            .count()
            >= 4
    );
}

#[test]
fn long_and_non_latin_text() {
    let wish = SetWish::new("cards-in-columns@=1.0.0", "Task", 4)
        .case(Case::LongText(1))
        .case(Case::NonLatin(1));
    let set = SampleBuilder::new(SampleSpec {
        seed: 5,
        anchor: anchor(),
        sets: vec![wish],
    })
    .build()
    .unwrap();
    let rows = set.rows("cards-in-columns@1.0.0").unwrap();
    assert!(
        rows.iter()
            .any(|r| r.get("title").is_some_and(|t| t.len() > 300))
    );
    assert!(
        rows.iter()
            .any(|r| r.get("title").is_some_and(|t| !t.is_ascii()))
    );
}

#[test]
fn a_form_set_may_hold_two_forms_without_shifting_each_other() {
    let board = SetWish::new("cards-in-columns@=1.0.0", "Task", 5).distinct("column", 2);
    let alone = SampleBuilder::new(SampleSpec {
        seed: 9,
        anchor: anchor(),
        sets: vec![board.clone()],
    })
    .build()
    .unwrap();
    let wish = SetWish::new("tree@=1.0.0", "Group", 4);
    let both = SampleBuilder::new(SampleSpec {
        seed: 9,
        anchor: anchor(),
        sets: vec![wish, board],
    })
    .build()
    .unwrap();
    assert_eq!(
        alone.rows("cards-in-columns@1.0.0").unwrap(),
        both.rows("cards-in-columns@1.0.0").unwrap()
    );
}
