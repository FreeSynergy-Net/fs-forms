//! A frontend of another program drawn inside the page of its parent (stage 6, S6-7, mode `panel`): a frame with a head that names
//! the program the frontend comes from, and under it the page of that frontend, exactly as its own window would show it. The child is
//! a frontend of ITS module (own grip, own held connection, own description); this file only draws what the owner opened.
//!
//! Pattern: Composite (a page holds another page as a part, drawn by the same code) with a Proxy for the child page (`Embedded`: the
//! parent keeps a handle to the cell of the child, never a copy of its data).

use std::collections::HashMap;
use std::sync::Arc;

use fs_components::blinc::layout::div::{Div, div};
use fs_components::blinc::layout::text::text;
use fs_components_app::Surface as _;

use crate::frontend_cell::FrontendCell;
use crate::view::Coded;

/// The class of the frame of a panel (its look is in the styles of the page).
pub(crate) const CLASS: &str = "fs-forms-embed";
/// The height a panel is given inside a scrolling column.
const HEIGHT: f32 = 420.0;

/// The child's page cell, held by the parent's part.
#[derive(Clone)]
pub(crate) struct Embedded(pub(crate) Arc<FrontendCell>);

impl std::fmt::Debug for Embedded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Embedded(..)")
    }
}

impl PartialEq for Embedded {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Embedded {}

/// What the owner opened for the places of one page: the head and the child, or the reason the child is not there.
pub(crate) type Panels = HashMap<String, Result<(String, Embedded), Coded>>;

/// The panel `head` (program and title of the child) with the page of `child` under it, in a column `width` wide.
pub(crate) fn draw(head: &str, child: &Embedded, width: f32) -> Div {
    let inner = (width - 2.0).max(0.0);
    div()
        .class(CLASS)
        .flex_col()
        .w(width)
        .child(
            div()
                .class("fs-forms-embed-head")
                .p_px(8.0)
                .child(text(head.to_owned()).size(13.0)),
        )
        .child_box(child.0.page().build(inner, HEIGHT))
}

/// The style of the frame and its head (theme variables only).
pub(crate) const STYLES: &str = ".fs-forms-embed { border-width: 1px; border-color: var(--border); border-radius: 8px; overflow: hidden; }\n.fs-forms-embed-head { background: var(--surface); border-bottom-width: 1px; border-bottom-color: var(--border); }\n";
