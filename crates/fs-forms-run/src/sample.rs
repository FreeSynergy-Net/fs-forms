//! Sample data that takes changes in memory (and the sample groups, see `sample_groups`): three columns, five cards, a card can be added and moved and a column added. It is
//! "Sample data (not your records)" in every picture and window (`is_sample`), and nothing leaves the process.
//!
//! Pattern: Fixture behind the port `Records`, state behind a `Mutex` (the board asks and changes from the interface thread).

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use fs_components_kanban::{Answer, Card, Column, Due, DueState, Label, Operation, Tone};

use fs_forms_bind::{Basic, FormRef, ModuleForm};

use crate::form_data::{FormData, FormRow};
use crate::records::{Records, SideItem};
use crate::sample_groups::groups;

/// Sample records the reader can change; the changes live only in memory.
#[derive(Debug)]
pub struct SampleBoard {
    columns: Mutex<Vec<Column>>,
    /// The forms of the sample groups module (tree and rows), by channel.
    forms: Vec<FormData>,
}

impl Default for SampleBoard {
    fn default() -> Self {
        Self {
            columns: Mutex::new(initial()),
            forms: groups(),
        }
    }
}

fn card(
    key: &str,
    title: &str,
    label: Option<(&str, Tone)>,
    due: Option<(&str, DueState)>,
    who: Option<&str>,
) -> Card {
    let mut c = Card::new(key, title);
    c.labels = label
        .map(|(t, tone)| Label {
            text: t.to_owned(),
            tone,
        })
        .into_iter()
        .collect();
    c.due = due.map(|(t, state)| Due {
        text: t.to_owned(),
        state,
    });
    c.assignee = who.map(str::to_owned);
    c
}

fn initial() -> Vec<Column> {
    let mut todo = Column::new("todo", "To do");
    todo.cards = vec![
        card(
            "FS-1",
            "Write the spec",
            Some(("docs", Tone::Success)),
            Some(("12 Nov", DueState::Overdue)),
            Some("Ada Lovelace"),
        ),
        card(
            "FS-2",
            "Review the plan",
            None,
            Some(("25 Nov", DueState::Open)),
            None,
        ),
    ];
    let mut doing = Column::new("doing", "Doing");
    doing.cards = vec![
        card(
            "FS-3",
            "Build the board",
            Some(("host", Tone::Accent)),
            None,
            Some("Grace Hopper"),
        ),
        card("FS-4", "Read the description", None, None, None),
    ];
    let mut done = Column::new("done", "Done");
    done.cards = vec![card(
        "FS-5",
        "Draw the first picture",
        Some(("picture", Tone::Success)),
        None,
        None,
    )];
    vec![todo, doing, done]
}

fn refused(code: &str) -> Answer {
    Answer::Refused(code.to_owned())
}

/// The key of a column named `title`: lower case, words joined by `-`.
fn slug(title: &str) -> String {
    title
        .split_whitespace()
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join("-")
}

impl SampleBoard {
    fn lock(&self) -> MutexGuard<'_, Vec<Column>> {
        self.columns.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Records for SampleBoard {
    fn is_sample(&self) -> bool {
        true
    }
    fn forms(&self) -> Arc<Vec<FormData>> {
        let mut all = self.forms.clone();
        all.push(board_form(&self.lock()));
        Arc::new(all)
    }
    fn side(&self) -> Vec<SideItem> {
        ["Inbox", "Today", "Project A", "Project B"]
            .iter()
            .map(|l| SideItem {
                key: (*l).to_lowercase().replace(' ', "-"),
                label: (*l).to_owned(),
            })
            .collect()
    }
    /// What this fixture can do: add a card, move a card, add a column. Rename and limit stay locked.
    fn can(&self, op: Operation) -> bool {
        matches!(
            op,
            Operation::AddCard | Operation::Move | Operation::AddColumn
        )
    }
    fn add_card(&self, column: &str, title: &str) -> Answer {
        let mut cols = self.lock();
        let next = cols
            .iter()
            .flat_map(|c| &c.cards)
            .filter_map(|c| c.key.strip_prefix("FS-")?.parse::<u32>().ok())
            .max()
            .unwrap_or(0)
            + 1;
        match cols.iter_mut().find(|c| c.key == column) {
            Some(c) => {
                c.cards.push(Card::new(&format!("FS-{next}"), title));
                Answer::Accepted
            }
            None => refused("no-column"),
        }
    }
    fn move_card(&self, key: &str, to: &str) -> Answer {
        let mut cols = self.lock();
        if !cols.iter().any(|c| c.key == to) {
            return refused("no-column");
        }
        let Some((from, at)) = cols
            .iter()
            .find_map(|c| Some((c.key.clone(), c.cards.iter().position(|x| x.key == key)?)))
        else {
            return refused("no-card");
        };
        let moved = cols
            .iter_mut()
            .find(|c| c.key == from)
            .map(|c| c.cards.remove(at));
        if let (Some(card), Some(target)) = (moved, cols.iter_mut().find(|c| c.key == to)) {
            target.cards.push(card);
        }
        Answer::Accepted
    }
    fn add_column(&self, title: &str) -> Answer {
        let key = slug(title);
        let mut cols = self.lock();
        // the same column by key or by name (the key of "To do" is `todo`)
        let named = |c: &Column| c.key == key || slug(&c.title) == key;
        if key.is_empty() || cols.iter().any(named) {
            return refused("exists");
        }
        cols.push(Column::new(&key, title));
        Answer::Accepted
    }
}

/// The board as the form `cards-in-columns` a module would give: a row per card, the labels, the due date and the assignee as its
/// facts (the module maps them to the channel `facts`), the place in its column as `order`. Everything the window shows of the sample
/// board goes through the same channels as the board of a real module.
fn board_form(columns: &[Column]) -> FormData {
    let mut rows = Vec::new();
    for column in columns {
        for (at, card) in column.cards.iter().enumerate() {
            let label = card.labels.first().map_or("", |l| l.text.as_str());
            let due = card.due.as_ref().map_or("", |d| d.text.as_str());
            let who = card.assignee.as_deref().unwrap_or("");
            let mut row = FormRow::of(
                &card.key,
                &[
                    ("title", &card.title),
                    ("column", &column.key),
                    ("column-label", &column.title),
                    ("order", &at.to_string()),
                ],
            );
            row.values.push((
                "facts".to_owned(),
                vec![label.to_owned(), due.to_owned(), who.to_owned()],
            ));
            rows.push(row);
        }
    }
    FormData {
        declared: ModuleForm {
            form: FormRef {
                name: "cards-in-columns".to_owned(),
                major: 1,
                minor: 0,
            },
            map: vec![(
                "facts".to_owned(),
                vec!["label".to_owned(), "due".to_owned(), "assignee".to_owned()],
            )],
            fields: vec![
                ("label".to_owned(), Basic::Text),
                ("due".to_owned(), Basic::Text),
                ("assignee".to_owned(), Basic::Text),
            ],
        },
        rows,
        may: Vec::new(),
        commands: Vec::new(),
        columns: columns
            .iter()
            .map(|c| (c.key.clone(), c.title.clone()))
            .collect(),
    }
}
