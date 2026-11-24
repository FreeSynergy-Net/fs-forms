//! The host's own composition of a general block that has no component (`sidebar`): put together from the side list of the records,
//! nothing painted here. It stays until `tree@1.1.0` lets a component draw it.
//!
//! Pattern: Composition (a general block put together from a list of the records).

use fs_components::blinc::layout::div::{Div, div};
use fs_components_widgets::{TreeNode, TreeView};

use super::Ctx;

pub(super) fn sidebar(ctx: &Ctx, width: f32, _height: f32) -> Div {
    let nodes: Vec<TreeNode> = ctx
        .records
        .side()
        .into_iter()
        .map(|i| TreeNode::new(i.key, i.label))
        .collect();
    div().w(width).child(TreeView::new(nodes).build())
}
