//! The adapters of the people blocks: `contact-row` (widget `PeopleWidget`) and `field-card` (widget `FieldCard`) over the rows
//! of the form `members`. Split from `draw` (one file, one job); the table that picks them is `draw::ADAPTERS`.
//!
//! Pattern: Adapter (rows of a form in, the element of the widget out).

use std::sync::Arc;

use fs_components::blinc::layout::div::{Div, div};
use fs_components_widgets::{FieldCard, Glyph};
use fs_components_workbenches::SourceError;
use fs_components_workbenches::people::{
    PeopleCapabilities, PeopleLabels, PeopleSource, PeopleWidget, PersonField, PersonRow,
    PersonSection, PersonSheet,
};

use crate::draw::{Ctx, MIN_H, rows};
use crate::form_data::FormRow;

/// The people the form `members` gives, for the contact list: the channel `person` is the name; `role` and `badge` are the fields of the sheet.
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
                r.get("person")
                    .unwrap_or(&r.key)
                    .to_lowercase()
                    .contains(&term)
            })
            .skip(offset)
            .take(limit)
            .map(|r| PersonRow::new(&r.key, vec![r.get("person").unwrap_or(&r.key).to_owned()]))
            .collect())
    }

    fn read(&self, key: &str) -> Result<PersonSheet, SourceError> {
        let row = self.0.iter().find(|r| r.key == key);
        let fields = ["role", "badge"]
            .iter()
            .filter_map(|c| row.and_then(|r| r.get(c)).map(|v| PersonField::new(*c, v)))
            .collect();
        Ok(PersonSheet::new(
            row.and_then(|r| r.get("person")).unwrap_or(key),
            vec![PersonSection::new("", fields)],
        ))
    }
}

/// The block `contact-row`: the people workbench (compact: list and detail) over the rows of the form `members`.
pub(crate) fn contact_row(ctx: &Ctx, width: f32, height: f32) -> Div {
    let people = PeopleWidget::compact(
        Arc::new(MemberPeople(rows(ctx, "members"))),
        PeopleLabels::default(),
    );
    div()
        .w(width)
        .h(height.max(MIN_H))
        .child(people.build(width, height.max(MIN_H)))
}

/// The block `field-card`: one card per row of the form `members` (the role is the label, the person the value).
pub(crate) fn field_card(ctx: &Ctx, width: f32, _height: f32) -> Div {
    let mut out = div().w(width).flex_col();
    for (n, row) in rows(ctx, "members").iter().enumerate() {
        let card = FieldCard::new(
            Glyph::Person,
            row.get("role").unwrap_or(""),
            row.get("person").unwrap_or(&row.key),
        );
        out = out.child(card.build(&format!("field-card-{n}")));
    }
    out
}
