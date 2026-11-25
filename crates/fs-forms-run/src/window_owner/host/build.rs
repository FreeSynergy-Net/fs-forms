//! Building what one open frontend holds, a window or a panel (stage 6, S6-7). A frontend node of the description (`PlaceNode::Frontend`)
//! becomes a child here, in the same call and recursively: a panel child is built the same way (its own description, its OWN module,
//! its own grip and held connection) and its page goes into the place of the parent; a window child is only listed, the owner opens it
//! through the surface once the parent is shown. The parent holds the children only through the table of the open frontends (`parent`),
//! so closing the parent lets every child fall, and with it its connection (= it signs off at its module).
//!
//! Pattern: Composite (a frontend holds frontends; the same `build` makes the root and every panel) with a Builder for the entry.
//! Abweichung: `scope-from` (the selection of a neighbour as the record of the child) is not carried out yet: the child shows no record.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use fs_components_app::AppWidget;
use fs_forms_layout::{ElemId, FrontendKey};

use super::entry::Entry;
use super::switch::{Core, rows};
use super::{HostWindows, OpenFrontend, Placement, WindowId};
use crate::compose::{Composition, Node, compose};
use crate::frontend_cell::{FrontendCell, root_page};
use crate::menus::menus_of;
use crate::panel::{Embedded, Panels};
use crate::record_hub::RecordKey;
use crate::status::set_status_line;
use crate::style::StyleCell;
use crate::view::{Coded, EmbedView};
use crate::words::words;

/// The most levels of frontends inside frontends (a panel in a panel in a panel ...): more is a loop in the descriptions.
const DEPTH: usize = 4;

/// One built frontend with everything that belongs to it.
pub(super) struct Built {
    pub(super) entry: Entry,
    /// The panels inside it (and inside them), by id: they fall with it.
    pub(super) children: Vec<(WindowId, Entry)>,
    /// The windows of its own the description asks for at the start (opened by the owner after this one is shown).
    pub(super) windows: Vec<OpenFrontend>,
    /// The title of its window, for the head of the panel it may become.
    pub(super) head: String,
}

/// The frontend nodes of the first screen that start with the parent (`shown = start`), in tree order, with their place.
fn embeds(comp: &Composition) -> Vec<(&str, &EmbedView)> {
    let nodes = comp.screens.first().map(|s| s.nodes.as_slice());
    nodes
        .unwrap_or_default()
        .iter()
        .filter_map(|n| match n {
            Node::Frontend { place, embed } if !embed.on_open => Some((place.as_str(), embed)),
            _ => None,
        })
        .collect()
}

impl HostWindows {
    /// The entry of `req` (a window, or with `req.mode == InFrame` a panel) and its children.
    pub(super) fn build(
        &self,
        req: &OpenFrontend,
        id: &WindowId,
        parent: Option<WindowId>,
        place: (String, String),
        chain: &[FrontendKey],
    ) -> Result<Built, Coded> {
        let refused = |cause: &Coded| {
            Coded::new("host-notice-frontend-refused")
                .with("extends", req.key.extends.clone())
                .with("frontend", req.key.frontend.clone())
                .with("cause", cause.code.clone())
        };
        let view = self.descriptions.view(&req.key)?;
        let module = self
            .modules
            .module(&req.key.extends)
            .map_err(|c| refused(&c))?;
        let key = RecordKey {
            range: req.key.extends.clone(),
            scope: req.scope.clone(),
        };
        let grip = self.hub.attach(&key, &module).map_err(|c| refused(&c))?;
        let comp = compose(&view, &*self.catalog);
        let texts = words(&self.locale);
        let mut above = chain.to_vec();
        above.push(req.key.clone());
        let mut inside = Inside::default();
        for (at, e) in embeds(&comp) {
            self.embed(&mut inside, id, &req.key.extends, (at, e), &above);
        }
        let page = root_page(
            &texts,
            &self.locale,
            &view,
            &comp,
            grip.records(),
            Arc::new(StyleCell::default()),
            &inside.panels,
        );
        let mut heads: Vec<(String, String)> = inside
            .panels
            .iter()
            .filter_map(|(at, p)| Some((at.clone(), p.as_ref().ok()?.0.clone())))
            .collect();
        heads.sort();
        let cell = FrontendCell::new(page);
        let token = self.serial.fetch_add(1, Ordering::SeqCst);
        let failed = |_: fs_components_app::AppError| {
            Coded::new("host-notice-window-failed").with("id", id.as_str())
        };
        let head = if view.title.is_empty() {
            texts.plain("host-title")
        } else {
            view.title.clone()
        };
        let app = if req.mode == Placement::Window {
            let app = AppWidget::new(head.clone(), vec![cell.screen()]).map_err(failed)?;
            set_status_line(&app, &texts, &comp, &*grip.records());
            let menus = menus_of(
                &texts,
                &view,
                &rows(&*self.descriptions, &req.key),
                &req.key.frontend,
            );
            if !menus.is_empty() {
                app.set_menus(menus, Core::handler(&self.core, token))
                    .map_err(failed)?;
            }
            Some(app)
        } else {
            None
        };
        let entry = Entry {
            key: req.key.clone(),
            place,
            scope: req.scope.clone(),
            parent,
            grip,
            module,
            app,
            cell,
            panels: heads,
            raised: 0,
            token,
        };
        Ok(Built {
            entry,
            children: inside.children,
            windows: inside.windows,
            head,
        })
    }

    /// One frontend node of the parent `id`: a panel is built now (its page goes into `inside.panels` under the place of the node, the
    /// reason when it cannot be built), a window is listed to be opened after the parent is shown.
    fn embed(
        &self,
        inside: &mut Inside,
        id: &WindowId,
        program: &str,
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
        let child = OpenFrontend::new(e.key.clone(), mode).with_parent(id.clone(), elem);
        if !e.panel {
            inside.windows.push(child);
            return;
        }
        let cid = WindowId::of(program, &e.elem);
        let built = if above.len() >= DEPTH || above.contains(&e.key) {
            Err(Coded::new("host-notice-frontend-loop").with("frontend", e.key.frontend.clone()))
        } else {
            self.build(
                &child,
                &cid,
                Some(id.clone()),
                (program.to_owned(), e.elem.clone()),
                above,
            )
        };
        match built {
            Ok(b) => {
                let head = format!("{} ({})", b.head, e.key.extends);
                let cell = Embedded(Arc::clone(&b.entry.cell));
                inside.panels.insert(place.to_owned(), Ok((head, cell)));
                inside.children.push((cid, b.entry));
                inside.children.extend(b.children);
                inside.windows.extend(b.windows);
            }
            Err(c) => {
                inside.panels.insert(place.to_owned(), Err(c));
            }
        }
    }
}

/// What the frontend nodes of one frontend made while it is built.
#[derive(Default)]
struct Inside {
    panels: Panels,
    children: Vec<(WindowId, Entry)>,
    windows: Vec<OpenFrontend>,
}
