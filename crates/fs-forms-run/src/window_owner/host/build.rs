//! Building what one open frontend holds, a window or a panel (stage 6, S6-7). A frontend node of the description (`PlaceNode::Frontend`)
//! becomes a child here, in the same call and recursively (`embed.rs`): a panel child is built the same way (its own description, its
//! OWN module, its own grip and held connection) and its page goes into the place of the parent; a window child is only listed, the
//! owner opens it through the surface once the parent is shown. The parent holds the children only through the table of the open
//! frontends (`parent`), so closing the parent lets every child fall, and with it its connection (= it signs off at its module).
//!
//! Pattern: Composite (a frontend holds frontends; the same `build` makes the root and every panel) with a Builder for the entry.
//! Abweichung: `scope-from` (the selection of a neighbour as the record of the child) is not carried out yet: such a node is a locked part.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use fs_components_app::AppWidget;
use fs_forms_layout::FrontendKey;

use super::entry::Entry;
use super::switch::{Core, rows};
use super::{OpenFrontend, Placement, WindowId};
use crate::compose::compose;
use crate::frontend_cell::{FrontendCell, root_page};
use crate::history::Attached;
use crate::record_hub::RecordKey;
use crate::status::set_status_line;
use crate::style::StyleCell;
use crate::view::{Coded, View};
use crate::words::words;

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

/// A frontend that has a module (a grip of its own) is connected: the notice "data sources are not connected" is no longer true.
pub(super) fn connected(mut view: View) -> View {
    view.notices
        .retain(|n| n.code != "host-notice-no-providers");
    view
}

impl Core {
    /// The entry of `req` (a window, or with `req.mode == InFrame` a panel) and its children.
    pub(super) fn build(
        self: &Arc<Self>,
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
        let view = connected(self.descriptions.view(&req.key)?);
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
        let inside = self.inside_of(id, &comp, &above);
        let page = root_page(
            &texts,
            &self.locale,
            &view,
            &comp,
            grip.records(),
            Arc::new(StyleCell::default()),
            &inside.panels,
        );
        let (heads, missing) = (inside.heads(), inside.missing());
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
        let attached = Attached::default();
        let app = if req.mode == Placement::Window {
            let app = AppWidget::new(head.clone(), vec![cell.screen()]).map_err(failed)?;
            set_status_line(&app, &texts, &comp, &*grip.records());
            let rows = rows(&*self.descriptions, &req.key);
            Self::install(
                self,
                &app,
                token,
                &view,
                rows,
                &req.key.frontend,
                &grip.records(),
                &attached,
            )
            .map_err(failed)?;
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
            attached,
            cell,
            panels: heads,
            missing,
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
}
