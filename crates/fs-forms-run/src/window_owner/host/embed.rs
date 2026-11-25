//! The frontend nodes of one frontend (stage 6, S6-7): what a frontend holds inside itself when it is built or when its frontend is
//! changed (`use_frontend`). Both go the same way through [`Core::inside_of`]: a panel is built now (its own description, module, grip
//! and held connection; its page goes into the place of the node), a window is listed for the owner to open after the parent is shown,
//! a node that cannot be carried out is a locked part with its own code, never a child with wrong data.
//!
//! Pattern: Composite (a frontend holds frontends; the same code makes the children of a root and of a panel).
//! Abweichung: `scope-from` and `shown = on-open` are not built yet (S6-7b): such a node is a locked part, not a child.

use std::sync::Arc;

use fs_forms_layout::{ElemId, FrontendKey};

use super::build::Built;
use super::entry::Entry;
use super::switch::Core;
use super::{OpenFrontend, Placement, WindowId};
use crate::compose::{Composition, Node};
use crate::panel::{Embedded, Panels};
use crate::view::{Coded, EmbedView};

/// The most levels of frontends inside frontends (a panel in a panel in a panel ...): more is a loop in the descriptions.
const DEPTH: usize = 4;

/// What the frontend nodes of one frontend made.
#[derive(Default)]
pub(super) struct Inside {
    pub(super) panels: Panels,
    pub(super) children: Vec<(WindowId, Entry)>,
    pub(super) windows: Vec<OpenFrontend>,
}

impl Inside {
    /// The heads of the built panels as (place, head), in place order.
    pub(super) fn heads(&self) -> Vec<(String, String)> {
        let mut heads: Vec<(String, String)> = self
            .panels
            .iter()
            .filter_map(|(at, p)| Some((at.clone(), p.as_ref().ok()?.0.clone())))
            .collect();
        heads.sort();
        heads
    }
}

impl Inside {
    /// The places whose frontend is not shown, with the code of the reason, in place order.
    pub(super) fn missing(&self) -> Vec<(String, String)> {
        let mut gone: Vec<(String, String)> = self
            .panels
            .iter()
            .filter_map(|(at, p)| Some((at.clone(), p.as_ref().err()?.code.clone())))
            .collect();
        gone.sort();
        gone
    }
}

/// The id of a child: `<parent id>/<elem>`, stable, and as the parent's id carries the record of the parent, the same node of the same
/// board shown for two records is two children (never one id for both).
pub(super) fn child_id(parent: &WindowId, elem: &str) -> WindowId {
    WindowId::of(parent.as_str(), elem)
}

/// The frontend nodes of the first screen, in tree order, with their place.
fn embeds(comp: &Composition) -> Vec<(&str, &EmbedView)> {
    let nodes = comp.screens.first().map(|s| s.nodes.as_slice());
    nodes
        .unwrap_or_default()
        .iter()
        .filter_map(|n| match n {
            Node::Frontend { place, embed } => Some((place.as_str(), embed)),
            _ => None,
        })
        .collect()
}

/// Whether `id` is already one of `list` (two nodes would hold one place).
fn taken<T>(list: &[(WindowId, T)], id: &WindowId) -> bool {
    list.iter().any(|(c, _)| c == id)
}

impl Core {
    /// Everything the frontend nodes of `comp` make for the open frontend `id` ; `above` is the chain of the
    /// keys from the root down to and including this frontend.
    pub(super) fn inside_of(
        self: &Arc<Self>,
        id: &WindowId,
        comp: &Composition,
        above: &[FrontendKey],
    ) -> Inside {
        let mut inside = Inside::default();
        for (at, e) in embeds(comp) {
            self.embed(&mut inside, id, (at, e), above);
        }
        inside
    }

    /// One frontend node of the parent `id`: a panel is built now (its page goes into `inside.panels` under the place of the node, the
    /// reason when it cannot be built), a window is listed to be opened after the parent is shown.
    fn embed(
        self: &Arc<Self>,
        inside: &mut Inside,
        id: &WindowId,
        (place, e): (&str, &EmbedView),
        above: &[FrontendKey],
    ) {
        let Ok(elem) = ElemId::parse(&e.elem) else {
            return; // the layout check refused such an id already
        };
        let mode = if e.panel {
            Placement::InFrame
        } else {
            Placement::Window
        };
        let said = |code: &str| Coded::new(code).with("frontend", e.key.frontend.clone());
        let wanted = if e.on_open {
            Some(said("host-notice-frontend-on-open-not-built"))
        } else if e.scope_from.is_some() {
            Some(said("host-notice-scope-from-not-built"))
        } else {
            None
        };
        if let Some(why) = wanted {
            if e.panel {
                inside.panels.insert(place.to_owned(), Err(why));
            } else if !e.on_open {
                eprintln!("fs-forms-run: {} {:?}", why.code, why.params);
            }
            return;
        }
        let child = OpenFrontend::new(e.key.clone(), mode).with_parent(id.clone(), elem);
        if !e.panel {
            inside.windows.push(child);
            return;
        }
        let cid = child_id(id, &e.elem);
        let built = if above.len() >= DEPTH || above.contains(&e.key) {
            Err(said("host-notice-frontend-loop"))
        } else if taken(&inside.children, &cid) {
            Err(Coded::new("host-notice-window-id-taken").with("id", cid.as_str()))
        } else {
            self.build(
                &child,
                &cid,
                Some(id.clone()),
                (id.as_str().to_owned(), e.elem.clone()),
                above,
            )
        };
        match built {
            Ok(b) => adopt(inside, place, (cid, e), b),
            Err(c) => {
                inside.panels.insert(place.to_owned(), Err(c));
            }
        }
    }
}

/// A built panel child goes into the place of its node; its own children and windows come with it.
fn adopt(inside: &mut Inside, place: &str, (cid, e): (WindowId, &EmbedView), b: Built) {
    let head = format!("{} ({})", b.head, e.key.extends);
    let cell = Embedded(Arc::clone(&b.entry.cell));
    inside.panels.insert(place.to_owned(), Ok((head, cell)));
    inside.children.push((cid, b.entry));
    inside.children.extend(b.children);
    inside.windows.extend(b.windows);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_child_id_is_made_of_the_parent_id_and_the_elem() {
        let a = WindowId::of_record("p", "board", Some("x"));
        let b = WindowId::of_record("p", "board", Some("y"));
        assert_eq!(child_id(&a, "n").as_str(), format!("{}/n", a.as_str()));
        assert_ne!(child_id(&a, "n"), child_id(&b, "n"));
    }

    #[test]
    fn a_second_child_with_the_same_id_is_taken() {
        let id = WindowId::of("p", "n");
        assert!(taken(&[(id.clone(), ())], &id));
        assert!(!taken(&[(id, ())], &WindowId::of("p", "m")));
    }
}
