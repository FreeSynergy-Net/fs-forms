//! The blocks of `fs-components` this host draws as elements of a page, and the data each one is given: `board` is the kanban
//! board, `sidebar` a tree of the side list. The mapping is one `match` (block name to component); nothing is painted here.
//! No operation of the board is connected (no provider), so every move is drawn locked with its hint, never silently.
//!
//! Pattern: Abstract Factory over `fs-components` (block name in, element out), data through the port `Records`.

use std::sync::Arc;

use fs_components::blinc::layout::div::{Div, div};
use fs_components::blinc::theme::{ColorScheme, ThemeState};
use fs_components_fields::{Look, Say};
use fs_components_kanban::{FixedBoard, KanbanBoard};
use fs_components_styles::StyleId;
use fs_components_widgets::{TreeNode, TreeView};

use crate::records::Records;

/// What a drawn block needs besides its name.
#[derive(Clone)]
pub(crate) struct Ctx {
    pub records: Arc<dyn Records>,
    pub say: Arc<dyn Say>,
}

/// Whether the reader of this window sees the dark scheme.
fn dark() -> bool {
    ThemeState::try_get().is_some_and(|t| t.scheme() == ColorScheme::Dark)
}

/// The block as an element, or `None` when this host has no drawing for it (the caller says so).
pub(crate) fn block(name: &str, ctx: &Ctx, width: f32) -> Option<Div> {
    match name {
        "board" => {
            let source = Arc::new(FixedBoard {
                columns: ctx.records.columns(),
                ops: Vec::new(),
            });
            let board =
                KanbanBoard::new(source, ctx.say.clone(), Look::new(StyleId::Admin, dark()));
            Some(div().w(width).h(380.0).child(board.build(width, 380.0)))
        }
        "sidebar" => {
            let nodes: Vec<TreeNode> = ctx
                .records
                .side()
                .into_iter()
                .map(|i| TreeNode::new(i.key, i.label))
                .collect();
            Some(div().w(width).child(TreeView::new(nodes).build()))
        }
        _ => None,
    }
}
