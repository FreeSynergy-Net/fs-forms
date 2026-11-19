//! The records a block draws: a port the host asks, so no block holds data of its own. In this version no provider is
//! connected, so the real implementation is [`NoRecords`] (nothing: the blocks stay empty and say so); [`SampleRecords`] is
//! built-in sample data for pictures and development (`--sample-data`), never used by a starter.
//!
//! Pattern: Port (`Records`) with a null object and a fixture.

use fs_components_kanban::{Card, Column, Due, DueState, Label, Tone};

/// One entry of a side list (a project, a label).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SideItem {
    /// The key given back on selection.
    pub key: String,
    /// What is shown.
    pub label: String,
}

/// What the blocks of the host draw their records from.
pub trait Records: Send + Sync {
    /// The columns with their cards of a board.
    fn columns(&self) -> Vec<Column>;
    /// The entries of a side list.
    fn side(&self) -> Vec<SideItem>;
    /// Whether these records are made-up sample data: the window then says so (it never shows invented records as real ones).
    fn is_sample(&self) -> bool {
        false
    }
}

/// No provider is connected: nothing to draw.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoRecords;

impl Records for NoRecords {
    fn columns(&self) -> Vec<Column> {
        Vec::new()
    }
    fn side(&self) -> Vec<SideItem> {
        Vec::new()
    }
}

/// Sample data: three columns, five cards (for pictures).
#[derive(Clone, Copy, Debug, Default)]
pub struct SampleRecords;

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

impl Records for SampleRecords {
    fn is_sample(&self) -> bool {
        true
    }
    fn columns(&self) -> Vec<Column> {
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
    fn side(&self) -> Vec<SideItem> {
        ["Inbox", "Today", "Project A", "Project B"]
            .iter()
            .map(|l| SideItem {
                key: (*l).to_lowercase().replace(' ', "-"),
                label: (*l).to_owned(),
            })
            .collect()
    }
}
