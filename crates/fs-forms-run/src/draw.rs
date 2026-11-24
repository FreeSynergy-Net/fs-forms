//! The blocks of `fs-components` this host draws as elements of a page, and the data each one is given. Which block has which widget is the catalog's (`widget` of the contract); `board` is the kanban
//! board, `sidebar` a tree of the side list, `tree` and `table` draw the forms `tree` and `members` by channel. The assignment is one table keyed by the WIDGET the catalog names (and three compositions of general blocks); nothing is painted here and nothing is decided here.
//! A control of the board is unlocked only when the description has an action for it and the records can do it (see `events`);
//! every other one is drawn locked with its hint, never silently.
//!
//! Pattern: Abstract Factory over `fs-components` (block name in, element out), data through the port `Records`.

use std::sync::Arc;

use fs_components::blinc::layout::div::{Div, div};
use fs_components::blinc::theme::{ColorScheme, ThemeState};
use fs_components_fields::{Look, Say};
use fs_components_kanban::KanbanBoard;
use fs_components_styles::StyleId;
use fs_components_widgets::dialogs::{Labels, modal};
use fs_components_widgets::grid::Column;
use fs_components_widgets::table::Table;
use fs_components_widgets::{FieldCard, Glyph, TreeNode, TreeView};
use fs_components_workbenches::SourceError;
use fs_components_workbenches::people::{
    PeopleCapabilities, PeopleLabels, PeopleSource, PeopleWidget, PersonField, PersonRow,
    PersonSection, PersonSheet,
};

use crate::events::{Ask, Wiring};
use crate::form_data::FormRow;
use crate::records::Records;
use crate::view::ActionView;

/// The least height of the board, in pixels.
pub(crate) const MIN_H: f32 = 380.0;

/// What a drawn block needs besides its name.
#[derive(Clone)]
pub(crate) struct Ctx {
    /// The style of the window NOW (asked at every drawing).
    pub style: Arc<dyn Fn() -> StyleId + Send + Sync>,
    pub records: Arc<dyn Records>,
    pub say: Arc<dyn Say>,
    /// The `[[action]]`s of the description: only what they name is unlocked on a board.
    pub actions: Arc<Vec<ActionView>>,
    /// The open question to the reader, if any.
    pub ask: Arc<Ask>,
    /// The words of the buttons of a dialog.
    pub labels: Labels,
}

/// Whether the reader of this window sees the dark scheme.
fn dark() -> bool {
    ThemeState::try_get().is_some_and(|t| t.scheme() == ColorScheme::Dark)
}

/// The function that draws a block as an element: width and height in, element out.
type Drawing = fn(&Ctx, f32, f32) -> Div;

/// The host's own compositions of the general blocks: the contract names no widget for them (scope `general`), the host puts
/// them together from the records. A block with a widget in the contract is NOT listed here (see [`ADAPTERS`]).
const COMPOSITIONS: &[(&str, Drawing)] = &[("sidebar", sidebar), ("tree", tree), ("table", table)];

/// Which function puts a widget of `fs-components` behind a block. The catalog says which widget a block has (`widget` of the
/// block file, `fs-components-catalog`); this table is keyed by the WIDGET, never by a block, so a block that gets a widget in the
/// contract is drawn as soon as its widget has an adapter here and a block that is renamed or added needs nothing. A widget needs
/// data of its own kind, so the adapter, not a name, says where it takes it from; a widget without an adapter is `not built`
/// (listed by `--list-blocks`), never silently dropped.
const ADAPTERS: &[(&str, Drawing)] = &[
    ("fs-components-kanban", board),
    ("PeopleWidget", contact_row),
    ("FieldCard", field_card),
];

/// The function that draws the block: the host's composition of a general block, or the adapter of the widget the catalog names.
fn drawing(name: &str) -> Option<Drawing> {
    if let Some((_, f)) = COMPOSITIONS.iter().find(|(n, _)| *n == name) {
        return Some(*f);
    }
    let widget = crate::blocks::widget_of(name)?;
    ADAPTERS.iter().find(|(w, _)| *w == widget).map(|(_, f)| *f)
}

/// Whether a function draws the block.
pub(crate) fn has(name: &str) -> bool {
    drawing(name).is_some()
}

/// Why a block of the catalog is not drawn, in words for the list: its widget has no adapter, or the contract names none.
pub(crate) fn missing(name: &str) -> String {
    match crate::blocks::widget_of(name) {
        Some(w) => format!("widget {w} has no adapter"),
        None => "no widget in the contract".to_owned(),
    }
}

/// The block as an element, or `None` when this host has no drawing for it (the caller says so).
pub(crate) fn block(name: &str, ctx: &Ctx, width: f32, height: f32) -> Option<Div> {
    drawing(name).map(|draw| draw(ctx, width, height))
}

fn board(ctx: &Ctx, width: f32, height: f32) -> Div {
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

fn sidebar(ctx: &Ctx, width: f32, _height: f32) -> Div {
    let nodes: Vec<TreeNode> = ctx
        .records
        .side()
        .into_iter()
        .map(|i| TreeNode::new(i.key, i.label))
        .collect();
    div().w(width).child(TreeView::new(nodes).build())
}

/// The rows of the form `name` the connected module gives (none: nothing is connected or the module does not offer it).
fn rows(ctx: &Ctx, name: &str) -> Vec<FormRow> {
    ctx.records
        .forms()
        .iter()
        .find(|d| d.declared.form.name == name)
        .map_or_else(Vec::new, |d| d.rows.clone())
}

/// The nodes under `parent` of the form `tree`: channels `parent` (the key of the node above, none for a root) and `label`.
/// A row seen twice on the way (a loop in the data) is cut, never followed forever.
fn nodes(rows: &[FormRow], parent: Option<&str>, seen: &mut Vec<String>) -> Vec<TreeNode> {
    let mut out = Vec::new();
    for row in rows.iter().filter(|r| r.get("parent") == parent) {
        if seen.contains(&row.key) {
            continue;
        }
        seen.push(row.key.clone());
        let mut node = TreeNode::new(&row.key, row.get("label").unwrap_or(&row.key));
        node.children = nodes(rows, Some(&row.key), seen);
        node.open = true;
        out.push(node);
    }
    out
}

/// The roots: rows without a parent, rows whose parent is not in the data (shown at the top), and then every row still unseen,
/// which can only be a pure loop (a -> b -> a): the first of it becomes a root, the loop is cut there. Never dropped.
fn roots(rows: &[FormRow]) -> Vec<TreeNode> {
    let mut seen = Vec::new();
    let mut out = nodes(rows, None, &mut seen);
    let mut root = |row: &FormRow, seen: &mut Vec<String>| {
        seen.push(row.key.clone());
        let mut node = TreeNode::new(&row.key, row.get("label").unwrap_or(&row.key));
        node.children = nodes(rows, Some(&row.key), seen);
        node.open = true;
        out.push(node);
    };
    for row in rows {
        let orphan = row
            .get("parent")
            .is_some_and(|p| !rows.iter().any(|r| r.key == p));
        if orphan && !seen.contains(&row.key) {
            root(row, &mut seen);
        }
    }
    for row in rows {
        if !seen.contains(&row.key) {
            root(row, &mut seen);
        }
    }
    out
}

fn tree(ctx: &Ctx, width: f32, _height: f32) -> Div {
    div()
        .w(width)
        .child(TreeView::new(roots(&rows(ctx, "tree"))).build())
}

/// The channels the member list shows, in this order (the channel `records` is the list itself).
const MEMBER_CHANNELS: &[&str] = &["person", "role", "badge"];

fn table(ctx: &Ctx, width: f32, _height: f32) -> Div {
    let cols: Vec<Column> = MEMBER_CHANNELS
        .iter()
        .map(|c| Column::new(*c, ctx.say.say(&format!("channel.{c}"))))
        .collect();
    let cells: Vec<Vec<String>> = rows(ctx, "members")
        .iter()
        .map(|r| {
            MEMBER_CHANNELS
                .iter()
                .map(|c| r.get(c).unwrap_or("").to_owned())
                .collect()
        })
        .collect();
    let column_w = (width / 3.0).max(80.0);
    div().w(width).child(
        Table::new(cols, cells)
            .id("members")
            .column_width(column_w)
            .build(),
    )
}

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
fn contact_row(ctx: &Ctx, width: f32, height: f32) -> Div {
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
fn field_card(ctx: &Ctx, width: f32, _height: f32) -> Div {
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

#[cfg(test)]
mod tests {
    use super::{nodes, roots};
    use crate::form_data::FormRow;

    fn row(key: &str, parent: &str) -> FormRow {
        FormRow::of(key, &[("parent", parent), ("label", key)])
    }

    #[test]
    fn the_tree_is_built_from_the_channels_parent_and_label_and_an_orphan_is_a_root() {
        let rows = [row("a", ""), row("b", "a"), row("c", "b"), row("x", "gone")];
        let r = roots(&rows);
        assert_eq!(
            r.iter().map(|n| n.key.as_str()).collect::<Vec<_>>(),
            ["a", "x"]
        );
        assert_eq!(r[0].count(), 3);
    }

    #[test]
    fn a_loop_in_the_data_ends() {
        let rows = [row("a", "b"), row("b", "a")];
        assert!(nodes(&rows, Some("b"), &mut Vec::new()).len() <= 1);
        // a pure loop is not lost: both records are shown, once each
        assert_eq!(
            roots(&rows)
                .iter()
                .map(fs_components_widgets::TreeNode::count)
                .sum::<usize>(),
            2
        );
    }

    #[test]
    fn a_loop_beside_a_normal_tree_loses_nothing() {
        let rows = [row("a", ""), row("b", "a"), row("x", "y"), row("y", "x")];
        let total: usize = roots(&rows)
            .iter()
            .map(fs_components_widgets::TreeNode::count)
            .sum();
        assert_eq!(total, 4);
    }
}
