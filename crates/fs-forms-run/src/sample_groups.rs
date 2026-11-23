//! Sample data of the groups module for pictures and development (`--sample-data`): a group tree and a member list, given by
//! channel exactly as the service of a module would give them. Made up; the window says "Sample data" (`is_sample`).
//!
//! Pattern: Fixture (the same values a `FormSource` of the module returns), never used by a starter.

use fs_forms_bind::{Basic, FormRef, ModuleForm};

use crate::form_data::{FormData, FormRow};

fn declared(name: &str, map: &[(&str, &str)], fields: &[(&str, Basic)]) -> ModuleForm {
    ModuleForm {
        form: FormRef {
            name: name.to_owned(),
            major: 1,
            minor: 0,
        },
        map: map
            .iter()
            .map(|(c, f)| ((*c).to_owned(), vec![(*f).to_owned()]))
            .collect(),
        fields: fields.iter().map(|(f, b)| ((*f).to_owned(), *b)).collect(),
    }
}

/// The group tree: nodes with parent, label, the mode of the edge and the owner mark.
fn tree() -> FormData {
    let node = |key: &str, parent: &str, label: &str, mode: &str, owner: &str| {
        FormRow::of(
            key,
            &[
                ("parent", parent),
                ("label", label),
                ("edge-kind", mode),
                ("owner-mark", owner),
            ],
        )
    };
    FormData {
        declared: declared(
            "tree",
            &[
                ("nodes", "Group"),
                ("parent", "parent"),
                ("label", "name"),
                ("edge-kind", "mode"),
                ("owner-mark", "mine"),
            ],
            &[
                ("parent", Basic::Reference),
                ("name", Basic::Text),
                ("mode", Basic::Choice),
                ("mine", Basic::Bool),
            ],
        ),
        // two trees of unequal depth (helfa UMFRAGE-ergebnis): regions down to topic teams, and a flat academy
        rows: vec![
            node("nord", "", "Region Nordland", "", "true"),
            node(
                "birken",
                "nord",
                "Local group Birkenhain",
                "bottom-up",
                "true",
            ),
            node("garten", "birken", "Garden team", "top-down", ""),
            node("beete", "garten", "Bed keepers", "single", ""),
            node("kompost", "garten", "Compost crew", "single", ""),
            node("repair", "birken", "Repair meeting", "single", ""),
            node("seeblick", "nord", "Local group Seeblick", "bottom-up", ""),
            node("akademie", "", "Academy Kleinwerk", "", ""),
            node("holz", "akademie", "Course timber building", "single", ""),
        ],
    }
}

/// The member list of the groups.
fn members() -> FormData {
    let row = |key: &str, person: &str, role: &str, badge: &str| {
        FormRow::of(key, &[("person", person), ("role", role), ("badge", badge)])
    };
    FormData {
        declared: declared(
            "members",
            &[
                ("records", "Member"),
                ("person", "person"),
                ("role", "role"),
                ("badge", "level"),
            ],
            &[
                ("person", Basic::Reference),
                ("role", Basic::Reference),
                ("level", Basic::Text),
            ],
        ),
        rows: vec![
            row("m1", "Mara Weidner", "Moderator", "Owner"),
            row("m2", "Jonas Brink", "Deputy", "Admin"),
            row("m3", "Tilda Ronne", "Gardener", ""),
            row("m4", "Piet Aldermann", "Treasurer", "Admin"),
            row("m5", "Sunniva Klar", "Cook", ""),
        ],
    }
}

/// The forms of the sample groups module.
#[must_use]
pub fn groups() -> Vec<FormData> {
    vec![tree(), members()]
}

#[cfg(test)]
mod tests {
    use super::groups;
    use crate::form_check::problems;
    use crate::form_data::FormRecords;

    #[test]
    fn the_sample_module_fits_the_blocks_tree_and_table() {
        let r = FormRecords {
            provider: "fake".into(),
            forms: groups(),
        };
        assert!(problems("tree", &r).is_empty() && problems("table", &r).is_empty());
    }

    #[test]
    fn every_node_has_a_known_parent_or_is_a_root() {
        let data = groups();
        let keys: Vec<_> = data[0].rows.iter().map(|r| r.key.as_str()).collect();
        for r in &data[0].rows {
            assert!(
                r.get("parent").is_none_or(|p| keys.contains(&p)),
                "{}",
                r.key
            );
        }
    }
}
