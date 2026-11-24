//! The host's own compositions of the general blocks (`sidebar`, `tree`, `table`): put together from the rows of the forms, nothing painted here.
//!
//! Pattern: Composition (general blocks put together from the rows of a form).

use fs_components::blinc::layout::div::{Div, div};
use fs_components_widgets::grid::Column;
use fs_components_widgets::table::Table;
use fs_components_widgets::{TreeNode, TreeView};

use super::Ctx;
use crate::form_data::FormRow;

pub(super) fn sidebar(ctx: &Ctx, width: f32, _height: f32) -> Div {
    let nodes: Vec<TreeNode> = ctx
        .records
        .side()
        .into_iter()
        .map(|i| TreeNode::new(i.key, i.label))
        .collect();
    div().w(width).child(TreeView::new(nodes).build())
}

/// The rows of the form `name` the connected module gives (none: nothing is connected or the module does not offer it).
pub(super) fn rows(ctx: &Ctx, name: &str) -> Vec<FormRow> {
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

pub(super) fn tree(ctx: &Ctx, width: f32, _height: f32) -> Div {
    div()
        .w(width)
        .child(TreeView::new(roots(&rows(ctx, "tree"))).build())
}

/// The heads of the columns of the form `rows`: the `label` first, then the values of the channel `columns` in the order of the map
/// (the words come from the MF2 files, `host-channel-*`; more columns are not drawn yet).
const ROWS_HEADS: &[&str] = &["person", "role", "badge"];

pub(super) fn table(ctx: &Ctx, width: f32, _height: f32) -> Div {
    let data = rows(ctx, "rows");
    let n_cols = data
        .iter()
        .map(|r| r.all("columns").len())
        .max()
        .unwrap_or(0)
        .min(ROWS_HEADS.len() - 1);
    let cols: Vec<Column> = ROWS_HEADS
        .iter()
        .take(n_cols + 1)
        .map(|c| Column::new(*c, ctx.say.say(&format!("channel.{c}"))))
        .collect();
    let cells: Vec<Vec<String>> = data
        .iter()
        .map(|r| {
            let mut cells = vec![r.get("label").unwrap_or(&r.key).to_owned()];
            cells.extend((0..n_cols).map(|i| r.all("columns").get(i).cloned().unwrap_or_default()));
            cells
        })
        .collect();
    let column_w = (width / cols.len().max(1) as f32).max(80.0);
    div().w(width).child(
        Table::new(cols, cells)
            .id("rows")
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
