//! The 20 tables: names, word lists against the model enums (K59), foreign keys, strict DDL, and the way a layout goes to rows and back.
use std::collections::HashSet;

use fs_forms_layout::*;
use fs_forms_store::spec::{Logical, Step};
use fs_forms_store::*;

#[test]
fn there_are_twenty_tables_and_four_steps() {
    assert_eq!(ALL.len(), 20);
    assert_eq!(STEPS.len(), 4);
    let names: HashSet<_> = ALL.iter().map(|t| t.name).collect();
    assert_eq!(names.len(), 20);
    assert!(names.iter().all(|n| n.starts_with("forms_")));
}

#[test]
fn every_foreign_key_names_a_table_and_its_columns() {
    for t in &ALL {
        for c in t
            .constraints
            .iter()
            .filter(|c| c.starts_with("FOREIGN KEY"))
        {
            let target = c.split("REFERENCES ").nth(1).unwrap();
            let (name, cols) = target.split_once('(').unwrap();
            let other = ALL
                .iter()
                .find(|o| o.name == name)
                .unwrap_or_else(|| panic!("{} -> {name}", t.name));
            for col in cols.trim_end_matches(')').split(',').map(str::trim) {
                assert!(
                    other.cols.iter().any(|x| x.name == col),
                    "{} -> {name}.{col}",
                    t.name
                );
            }
        }
    }
}

#[test]
fn model_words_are_in_the_closed_lists() {
    use fs_forms_store::tables as w;
    let in_list = |list: &[&str], word: &str| assert!(list.contains(&word), "{word}");
    for k in ElemKind::ALL_NAMES {
        in_list(w::TARGET_KIND, k);
    }
    for c in [
        Cause::TargetRemoved,
        Cause::TargetChanged,
        Cause::AnchorRemoved,
        Cause::FieldUnknown,
        Cause::TypeMismatch,
        Cause::Locked,
        Cause::ResultInvalid,
        Cause::AllowlistWidened,
    ] {
        in_list(w::CAUSE, c.as_str());
    }
    for s in [SourceScope::All, SourceScope::SelectedList] {
        in_list(w::SCOPE_SRC, s.word());
    }
    for n in [Navigation::Single, Navigation::ListDetail] {
        in_list(w::NAVIGATION, n.word());
    }
    for t in [
        Trigger::Button,
        Trigger::Menu,
        Trigger::Key,
        Trigger::Drop,
        Trigger::QuickAdd,
    ] {
        in_list(w::TRIGGER, t.word());
    }
    for c in [Confirm::None, Confirm::Confirm, Confirm::Choice] {
        in_list(w::CONFIRM, c.word());
    }
    for o in [OrderBy::Field, OrderBy::Order] {
        in_list(w::ORDER_BY, o.word());
    }
    assert_eq!(w::TARGET_KIND.len(), ElemKind::ALL_NAMES.len());
}

#[test]
fn log_is_append_only_by_steps_two_and_three() {
    let s: Vec<String> = STEPS.iter().flat_map(Step::statements).collect();
    assert!(
        s.iter()
            .any(|x| x.contains("forms_log_no_update") && x.contains("BEFORE UPDATE"))
    );
    assert!(
        s.iter()
            .any(|x| x.contains("forms_log_no_delete") && x.contains("BEFORE DELETE"))
    );
    assert!(s.iter().any(|x| x.starts_with("INSERT INTO forms_rev")));
}

#[test]
fn every_table_is_strict_and_enums_have_checks() {
    for t in &ALL {
        assert!(t.ddl().ends_with("STRICT"), "{}", t.name);
        for c in t.cols {
            if let Logical::Enum([first, ..]) = c.logical {
                assert!(t.ddl().contains(&format!("'{first}'")));
            }
        }
    }
}

/// Writes the DDL of all steps to the file named by `FORMS_DDL_OUT` (used by the SQL check outside of cargo).
#[test]
fn dump_ddl_for_the_sql_check() {
    if let Ok(path) = std::env::var("FORMS_DDL_OUT") {
        let sql: Vec<String> = STEPS
            .iter()
            .flat_map(Step::statements)
            .map(|s| format!("{s};"))
            .collect();
        std::fs::write(path, sql.join("\n")).unwrap();
    }
}
