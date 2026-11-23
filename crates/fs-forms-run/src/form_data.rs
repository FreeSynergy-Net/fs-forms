//! The records of a form as a module's service gives them: rows with values by CHANNEL name (`label`, `parent` ...), plus the
//! declaration of the module (which of its fields fill which channel). The blocks draw only by channel; no field name of a module is
//! known here. `FormRecords` is the [`Records`] a connected module gives (the starter builds it from the `FormSource` of the module).
//!
//! Pattern: Value objects + Adapter (rows by channel in, the port `Records` out).

use fs_components_kanban::{Card, Column};
use fs_forms_bind::ModuleForm;

use crate::records::{Records, SideItem};

/// One record: a key and its values by channel (a channel may have several values; a record may leave a channel out).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormRow {
    /// The key of the record.
    pub key: String,
    /// The values by channel name.
    pub values: Vec<(String, Vec<String>)>,
}

impl FormRow {
    /// A row with these `(channel, value)` pairs (one value each).
    #[must_use]
    pub fn of(key: &str, pairs: &[(&str, &str)]) -> Self {
        Self {
            key: key.to_owned(),
            values: pairs
                .iter()
                .map(|(c, v)| ((*c).to_owned(), vec![(*v).to_owned()]))
                .collect(),
        }
    }

    /// The values of `channel` (none when the record leaves it out).
    #[must_use]
    pub fn all(&self, channel: &str) -> &[String] {
        self.values
            .iter()
            .find(|(c, _)| c == channel)
            .map_or(&[], |(_, v)| v.as_slice())
    }

    /// The first value of `channel`; empty values count as none.
    #[must_use]
    pub fn get(&self, channel: &str) -> Option<&str> {
        self.all(channel)
            .first()
            .map(String::as_str)
            .filter(|v| !v.is_empty())
    }
}

/// One form a module offers: its declaration and its rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormData {
    /// The form, the version the module keeps and the map channel to field (`catalog/forms.json` of the module).
    pub declared: ModuleForm,
    /// The rows.
    pub rows: Vec<FormRow>,
}

/// The records of a connected module, by form.
#[derive(Clone, Debug, Default)]
pub struct FormRecords {
    /// The provider named by the service; `fake` is made-up data (the window then says "Sample data").
    pub provider: String,
    /// The forms.
    pub forms: Vec<FormData>,
}

/// The key of a column named `title`: lower case, words joined by `-`.
fn slug(title: &str) -> String {
    title
        .split_whitespace()
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join("-")
}

impl FormRecords {
    fn data(&self, form: &str) -> Option<&FormData> {
        self.forms.iter().find(|f| f.declared.form.name == form)
    }
}

impl Records for FormRecords {
    /// The cards of the form `cards-in-columns`: one column per value of the channel `column`, in order of appearance.
    fn columns(&self) -> Vec<Column> {
        let mut out: Vec<Column> = Vec::new();
        for row in self.data("cards-in-columns").map_or(&[][..], |d| &d.rows) {
            let (Some(title), Some(column)) = (row.get("title"), row.get("column")) else {
                continue; // required channels: a row without them cannot be placed
            };
            let key = slug(column);
            let at = out.iter().position(|c| c.key == key).unwrap_or_else(|| {
                out.push(Column::new(&key, column));
                out.len() - 1
            });
            out[at].cards.push(Card::new(&row.key, title));
        }
        out
    }
    fn side(&self) -> Vec<SideItem> {
        Vec::new()
    }
    fn is_sample(&self) -> bool {
        self.provider == "fake"
    }
    fn forms(&self) -> &[FormData] {
        &self.forms
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fs_forms_bind::FormRef;

    fn cards(rows: Vec<FormRow>) -> FormRecords {
        FormRecords {
            provider: "fake".into(),
            forms: vec![FormData {
                declared: ModuleForm {
                    form: FormRef {
                        name: "cards-in-columns".into(),
                        major: 1,
                        minor: 0,
                    },
                    map: Vec::new(),
                    fields: Vec::new(),
                },
                rows,
            }],
        }
    }

    #[test]
    fn the_board_is_made_from_the_channels_column_and_title_in_order_of_appearance() {
        let r = cards(vec![
            FormRow::of("a", &[("title", "One"), ("column", "To do")]),
            FormRow::of("b", &[("title", "Two"), ("column", "Doing")]),
            FormRow::of("c", &[("title", "Three"), ("column", "To do")]),
            FormRow::of("d", &[("title", "No column")]),
        ]);
        let cols = r.columns();
        let names: Vec<_> = cols.iter().map(|c| c.title.as_str()).collect();
        assert_eq!(names, ["To do", "Doing"]);
        let keys: Vec<Vec<_>> = cols
            .iter()
            .map(|c| c.cards.iter().map(|k| k.key.as_str()).collect())
            .collect();
        assert_eq!(keys, [vec!["a", "c"], vec!["b"]]);
        assert!(r.is_sample());
    }

    #[test]
    fn only_the_provider_fake_is_sample_data_and_a_missing_form_gives_no_columns() {
        let mut r = cards(Vec::new());
        r.provider = "tasks".into();
        assert!(!r.is_sample() && r.columns().is_empty());
        assert_eq!(FormRecords::default().columns().len(), 0);
    }

    #[test]
    fn a_value_is_read_by_channel_and_an_empty_one_is_none() {
        let row = FormRow::of("k", &[("label", "x"), ("parent", "")]);
        assert_eq!(row.get("label"), Some("x"));
        assert_eq!(row.get("parent"), None);
        assert_eq!(row.all("nope").len(), 0);
    }
}
