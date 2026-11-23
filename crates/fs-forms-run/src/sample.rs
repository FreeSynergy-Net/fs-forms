//! Sample data that takes changes in memory (and the sample groups, see `sample_groups`): three columns, five cards, a card can be added and moved and a column added. It is
//! "Sample data (not your records)" in every picture and window (`is_sample`), and nothing leaves the process.
//!
//! Pattern: Fixture behind the port `Records`, state behind a `Mutex` (the board asks and changes from the interface thread).

use std::sync::{Mutex, MutexGuard, PoisonError};

use fs_components_kanban::{Answer, Card, Column, Due, DueState, Label, Operation, Tone};

use crate::form_data::FormData;
use crate::records::{Records, SideItem};
use crate::sample_groups::groups;

/// Sample records the reader can change; the changes live only in memory.
#[derive(Debug)]
pub struct SampleBoard {
    columns: Mutex<Vec<Column>>,
    /// The forms of the sample groups module (tree and members), by channel.
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
    fn forms(&self) -> &[FormData] {
        &self.forms
    }
    fn columns(&self) -> Vec<Column> {
        self.lock().clone()
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
    fn move_card(&self, key: &str, from: &str, to: &str) -> Answer {
        let mut cols = self.lock();
        if !cols.iter().any(|c| c.key == to) {
            return refused("no-column");
        }
        let Some(at) = cols
            .iter()
            .find(|c| c.key == from)
            .and_then(|c| c.cards.iter().position(|x| x.key == key))
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
