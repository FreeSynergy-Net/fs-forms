//! The cell that holds the root of one window: the page of the frontend that is open NOW. The window (the shell of
//! `fs-components-app`) is made once with a single screen that asks the cell at every drawing; changing the frontend puts a new page
//! into the cell and wakes the window. Id, record, records and the held connection are not in the cell, so they cannot change with it.
//!
//! Pattern: Bridge (as in `WindowOwner::use_frontend`): the window and its surface stay, the page that draws them is exchanged.
//! Abweichung: one screen only. A composition with more pages shows its first one (and the notices); the shell cannot exchange its
//! list of screens yet (that needs `fs-components-app`), so a second page is not reachable until it can.

use std::sync::{Arc, Mutex, PoisonError};

use fs_components_app::{Element, Screen, Surface};
use fs_components_mf2::Mf2Texts;
use fs_components_widgets::states;
use fs_components_words::{Locale, Localized};

use crate::after_write::AfterWrite;
use crate::compose::Composition;
use crate::draw::Ctx;
use crate::page::Page;
use crate::panel::Panels;
use crate::parts::{Part, line, parts_with};
use crate::records::Records;
use crate::style::StyleSource;
use crate::view::{Coded, View};
use crate::window::ctx_of;

/// The id of the one screen of a window made over a cell.
pub(crate) const SCREEN: &str = "frontend";

/// The page that is drawn now.
pub(crate) struct FrontendCell {
    page: Mutex<Arc<Page>>,
}

impl FrontendCell {
    pub(crate) fn new(page: Page) -> Arc<Self> {
        Arc::new(Self {
            page: Mutex::new(Arc::new(page)),
        })
    }

    /// The next drawing shows `page`.
    pub(crate) fn set(&self, page: Page) {
        *self.page.lock().unwrap_or_else(PoisonError::into_inner) = Arc::new(page);
    }

    pub(crate) fn page(&self) -> Arc<Page> {
        Arc::clone(&self.page.lock().unwrap_or_else(PoisonError::into_inner))
    }

    /// The one screen of the window: it draws what the cell holds.
    pub(crate) fn screen(self: &Arc<Self>) -> Screen {
        Screen::new(SCREEN, SCREEN, Arc::new(Cell(Arc::clone(self))))
    }
}

struct Cell(Arc<FrontendCell>);

impl Surface for Cell {
    fn build(&self, width: f32, height: f32) -> Box<dyn Element> {
        self.0.page().build(width, height)
    }

    fn styles(&self) -> String {
        self.0.page().styles()
    }
}

/// The page of a composition: its first screen (with the panels `panels` the owner opened for its frontend nodes), and the notices
/// of the window as a locked part when there are any.
#[allow(clippy::too_many_arguments)] // two call sites (open, switch); a struct of these would only be built twice
pub(crate) fn root_page(
    texts: &Mf2Texts,
    locale: &Locale,
    view: &View,
    comp: &Composition,
    records: Arc<dyn Records>,
    program_style: Arc<dyn StyleSource>,
    panels: &Panels,
    after_write: AfterWrite,
) -> Page {
    // a window the owner opens takes the time from its records (no command line there: the clock of the data is the whole answer)
    let clock = records.clock();
    let ctx: Ctx = ctx_of(
        texts,
        locale,
        view,
        records,
        program_style,
        (after_write, clock),
    );
    let first = comp.screens.first();
    let mut shown = first
        .map(|s| parts_with(texts, &s.nodes, panels))
        .unwrap_or_default();
    let mut lines: Vec<String> = comp.notices.iter().map(|c| line(texts, c)).collect();
    if comp.screens.len() > 1 {
        // no silent drop: the pages 2..n are not reachable in this window yet, and the window says so (a cell per page needs a fixed
        // screen count at open, which a change to a frontend with another count would break)
        lines.push(line(
            texts,
            &Coded::new("host-notice-more-pages").with("pages", comp.screens.len().to_string()),
        ));
    }
    if !lines.is_empty() || first.is_none() {
        shown.push(Part::Locked {
            name: String::new(),
            lines,
        });
    }
    Page {
        ctx,
        empty: texts.plain("host-page-empty"),
        frame: first.map(|s| s.frame.clone()).unwrap_or_default(),
        parts: shown,
        labels: states::Labels::for_locale(locale),
    }
}
