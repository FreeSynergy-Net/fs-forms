//! The blocks of `fs-components` this host draws as elements of a page, and the data each one is given: `board` is the kanban
//! board, `sidebar` a tree of the side list, `tree` and `table` draw the forms `tree` and `members` by channel. The assignment is one table (block name to drawing function); nothing is painted here and nothing is decided here.
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
use fs_components_widgets::{TreeNode, TreeView};

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

/// Which function draws which block of the catalog. This is an assignment, not a decision: whether a block exists and what it
/// takes is the catalog's (`fs-components-catalog`, see `blocks`); here stands only the component that stands behind its name.
const DRAWINGS: &[(&str, Drawing)] = &[
    ("board", board),
    ("sidebar", sidebar),
    ("tree", tree),
    ("table", table),
];

/// Whether a function draws the block.
pub(crate) fn has(name: &str) -> bool {
    DRAWINGS.iter().any(|(n, _)| *n == name)
}

/// The block as an element, or `None` when this host has no drawing for it (the caller says so).
pub(crate) fn block(name: &str, ctx: &Ctx, width: f32, height: f32) -> Option<Div> {
    DRAWINGS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, draw)| draw(ctx, width, height))
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
fn rows<'a>(ctx: &'a Ctx, name: &str) -> &'a [FormRow] {
    ctx.records
        .forms()
        .iter()
        .find(|d| d.declared.form.name == name)
        .map_or(&[], |d| d.rows.as_slice())
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

/// The roots: rows without a parent, and rows whose parent is not in the data (shown at the top, never dropped).
fn roots(rows: &[FormRow]) -> Vec<TreeNode> {
    let mut seen = Vec::new();
    let mut out = nodes(rows, None, &mut seen);
    for row in rows {
        let orphan = row
            .get("parent")
            .is_some_and(|p| !rows.iter().any(|r| r.key == p));
        if orphan && !seen.contains(&row.key) {
            seen.push(row.key.clone());
            let mut node = TreeNode::new(&row.key, row.get("label").unwrap_or(&row.key));
            node.children = nodes(rows, Some(&row.key), &mut seen);
            node.open = true;
            out.push(node);
        }
    }
    out
}

fn tree(ctx: &Ctx, width: f32, _height: f32) -> Div {
    div()
        .w(width)
        .child(TreeView::new(roots(rows(ctx, "tree"))).build())
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
        assert!(
            roots(&rows)
                .iter()
                .map(fs_components_widgets::TreeNode::count)
                .sum::<usize>()
                <= 2
        );
    }
}
