//! The adapters of the widgets of `fs-components`: what each widget is given and from where (the board, the people workbench, the field card).
//!
//! Pattern: Adapter (a widget of `fs-components` behind a block, data through the port `Records`).

use std::sync::{Arc, PoisonError};

use fs_components::blinc::layout::div::{Div, div};
use fs_components_fields::Look;
use fs_components_kanban::KanbanBoard;
use fs_components_widgets::dialogs::modal;
use fs_components_widgets::{FieldCard, Glyph};
use fs_components_workbenches::SourceError;
use fs_components_workbenches::people::{
    PeopleCapabilities, PeopleLabels, PeopleSource, PeopleWidget, PersonField, PersonRow,
    PersonSection, PersonSheet,
};

use super::compositions::rows;
use super::{Ctx, MIN_H, dark};
use crate::events::Wiring;
use crate::form_data::FormRow;

pub(super) fn board(ctx: &Ctx, width: f32, height: f32) -> Div {
    let wiring = Arc::new(Wiring {
        actions: ctx.actions.clone(),
        records: ctx.records.clone(),
        ask: ctx.ask.clone(),
        say: ctx.say.clone(),
        labels: ctx.labels.clone(),
    });
    let board = KanbanBoard::new(
        wiring.clone(),
        ctx.say.clone(),
        Look::new((ctx.style)(), dark()),
    )
    .events(wiring);
    let height = height.max(MIN_H);
    let ask = ctx.ask.clone();
    // the question to the reader (a column name) stands over the board while it is open
    let area = ctx.ask.live.region("ask", move || {
        let page = board.build(width, height);
        match ask.current() {
            Some(dialog) => {
                dialog.focus();
                modal(page, dialog.build())
            }
            None => div().w(width).h(height).child(page),
        }
    });
    div().w(width).h(height).child(area)
}

/// The people the form `rows` gives, for the contact list: the channel `label` is the name; the first two values of `columns` (role, level) are the fields of the sheet.
/// Read only: the source can do nothing else (no edit, no create), so the workbench shows those controls locked, never working.
struct MemberPeople(Vec<FormRow>);

impl PeopleSource for MemberPeople {
    fn capabilities(&self) -> PeopleCapabilities {
        PeopleCapabilities::default()
    }

    fn rows(&self, term: &str, offset: usize, limit: usize) -> Result<Vec<PersonRow>, SourceError> {
        let term = term.to_lowercase();
        Ok(self
            .0
            .iter()
            .filter(|r| {
                r.get("label")
                    .unwrap_or(&r.key)
                    .to_lowercase()
                    .contains(&term)
            })
            .skip(offset)
            .take(limit)
            .map(|r| PersonRow::new(&r.key, vec![r.get("label").unwrap_or(&r.key).to_owned()]))
            .collect())
    }

    fn read(&self, key: &str) -> Result<PersonSheet, SourceError> {
        let row = self.0.iter().find(|r| r.key == key);
        let fields = ["role", "badge"]
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                let value = row.and_then(|r| r.all("columns").get(i))?;
                (!value.is_empty()).then(|| PersonField::new(*c, value))
            })
            .collect();
        Ok(PersonSheet::new(
            row.and_then(|r| r.get("label")).unwrap_or(key),
            vec![PersonSection::new("", fields)],
        ))
    }
}

/// The block `contact-row`: the people workbench (compact: list and detail) over the rows of the form `rows`.
/// The widget holds the state (selection, search, "more"), so it is kept in the context over a rebuild and made anew only when the rows change.
pub(super) fn contact_row(ctx: &Ctx, width: f32, height: f32) -> Div {
    let members = rows(ctx, "rows");
    let people = {
        let mut slot = ctx.people.lock().unwrap_or_else(PoisonError::into_inner);
        match slot.as_ref() {
            Some((seen, widget)) if *seen == members => widget.clone(),
            _ => {
                let widget = PeopleWidget::compact(
                    Arc::new(MemberPeople(members.clone())),
                    PeopleLabels::default(),
                );
                *slot = Some((members, widget.clone()));
                widget
            }
        }
    };
    div()
        .w(width)
        .h(height.max(MIN_H))
        .child(people.build(width, height.max(MIN_H)))
}

/// The block `field-card`: one card per row of the form `rows` (the role is the label, the person the value).
pub(super) fn field_card(ctx: &Ctx, width: f32, _height: f32) -> Div {
    let mut out = div().w(width).flex_col();
    for (n, row) in rows(ctx, "rows").iter().enumerate() {
        let card = FieldCard::new(
            Glyph::Person,
            row.all("columns").first().map_or("", String::as_str),
            row.get("label").unwrap_or(&row.key),
        );
        out = out.child(card.build(&format!("field-card-{n}")));
    }
    out
}
