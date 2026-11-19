//! The window: one screen per page of the composition, a menu from the plan, a status line that says how the host stands. Every
//! part is a block of `fs-components` (`partial-notice`, `empty-state`, `skeleton`); no widget is painted by hand.
//!
//! Pattern: Adapter: the composition in, a shell of `fs-components-app` out. The sentences are made before drawing (`Said`), so the
//! surfaces only draw. A locked part is one `partial-notice` listing every code with its sentence.

use std::fmt::Write as _;
use std::sync::{Arc, Mutex};

use fs_components::blinc::layout::div::div;
use fs_components::blinc::layout::widgets::scroll::{Scroll, ScrollPhysics};
use fs_components_app::{AppError, AppWidget, Element, Menu, MenuEntry, Screen, Status, Surface};
use fs_components_mf2::Mf2Texts;
use fs_components_widgets::states::{self, EmptyState, PartialNotice};
use fs_components_widgets::styles;
use fs_components_words::{Locale, Localized};

use crate::compose::{Composition, HostState, Node, ScreenTree};
use crate::draw::{self, Ctx};
use crate::error::HostError;
use crate::records::{NoRecords, Records};
use crate::view::{Coded, MenuItem, MenuState, View};

/// A part, with its sentences already made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Part {
    /// A block that is built; the host has a drawing for the state blocks only.
    Block {
        place: String,
        block: String,
        title: Option<String>,
    },
    /// Not drawn: the name of the part and one line per code.
    Locked { name: String, lines: Vec<String> },
    /// An empty page with its sentence.
    Empty(String),
}

/// The sentence of a code, with the code and its parameters behind it; a code without a sentence is shown as itself.
pub(crate) fn line(texts: &Mf2Texts, c: &Coded) -> String {
    let said = texts.plain(&c.code);
    let mut out = if said.is_empty() || said == c.code {
        c.code.clone()
    } else {
        format!("{said} ({})", c.code)
    };
    for (k, v) in &c.params {
        let _ = write!(out, " {k}={v}");
    }
    out
}

pub(crate) fn parts(texts: &Mf2Texts, nodes: &[Node]) -> Vec<Part> {
    nodes
        .iter()
        .map(|n| match n {
            Node::Block {
                place,
                block,
                title,
                ..
            } => Part::Block {
                place: place.clone(),
                block: block.clone(),
                title: title.clone(),
            },
            Node::Locked { place, codes } => Part::Locked {
                name: place.clone().unwrap_or_default(),
                lines: codes.iter().map(|c| line(texts, c)).collect(),
            },
            Node::Empty { .. } => Part::Empty(texts.plain("host-page-empty")),
        })
        .collect()
}

/// The lines of a notice, the name of the part in front of the first one, and how many parts the notice counts (one per line).
pub(crate) fn notice_items(name: &str, lines: &[String]) -> (Vec<String>, usize) {
    let mut missing = lines.to_vec();
    if let (false, Some(first)) = (name.is_empty(), missing.first_mut()) {
        *first = format!("{name}: {first}");
    }
    let total = missing.len();
    (missing, total)
}

struct Page {
    ctx: Ctx,
    empty: String,
    parts: Vec<Part>,
    labels: states::Labels,
}

impl Surface for Page {
    fn build(&self, width: f32, height: f32) -> Box<dyn Element> {
        let mut column = div().flex_col().gap(16.0).p(24.0);
        for part in &self.parts {
            column = match part {
                Part::Locked { name, lines } => {
                    let (missing, total) = notice_items(name, lines);
                    column.child(PartialNotice::new(&self.labels, &missing, total).build())
                }
                Part::Empty(sentence) => column.child(EmptyState::new(sentence).build()),
                Part::Block {
                    place,
                    block,
                    title,
                } if block == "empty-state" => {
                    let _ = place;
                    column.child(EmptyState::new(title.as_deref().unwrap_or(&self.empty)).build())
                }
                Part::Block { place, block, .. } => {
                    match draw::block(block, &self.ctx, width - 48.0) {
                        Some(element) => column.child(element),
                        None => column
                            .child(states::Skeleton::new(&format!("{place} ({block})")).build()),
                    }
                }
            };
        }
        let physics = Arc::new(Mutex::new(ScrollPhysics::default()));
        Box::new(
            div().w(width).h(height).child(
                Scroll::with_physics(physics)
                    .w(width)
                    .h(height)
                    .content(column),
            ),
        )
    }

    fn styles(&self) -> String {
        format!("{}{}", styles(), states::STYLES)
    }
}

/// The composition of a failure that is shown in the window instead of ending the host.
#[must_use]
pub fn failure(error: &HostError) -> Composition {
    let mut coded = Coded::new(format!("host-error-{}", error.code()));
    for (k, v) in error.params() {
        coded = coded.with(k, v);
    }
    Composition {
        state: HostState::Refused,
        screens: vec![ScreenTree {
            page: "error".to_owned(),
            frame: "workbench".to_owned(),
            nodes: vec![Node::Locked {
                place: None,
                codes: vec![coded],
            }],
        }],
        notices: Vec::new(),
    }
}

fn menu_entry(texts: &Mf2Texts, item: &MenuItem) -> Option<MenuEntry> {
    match item.state {
        MenuState::Hidden => None, // rights come before "coming": a command she may not run is not shown at all
        MenuState::Enabled => Some(MenuEntry::new(&item.id, &item.label)),
        MenuState::Coming => Some(
            MenuEntry::new(
                &item.id,
                format!("{} ({})", item.label, texts.plain("host-menu-coming")),
            )
            .disabled(),
        ),
    }
}

/// The keys the kanban board asks for (`kanban.refused.*` codes come from the provider and show as their key until it names them).
const KANBAN_KEYS: &[&str] = &[
    "kanban.add_column",
    "kanban.rename",
    "kanban.limit",
    "kanban.drop",
    "kanban.grab",
    "kanban.move_left",
    "kanban.move_right",
    "kanban.more_facts",
    "kanban.empty",
    "kanban.soon",
];

/// The window for `comp`.
///
/// # Errors
/// The shell could not be made (no screen, a name that is empty).
pub fn app(
    texts: &Mf2Texts,
    locale: &Locale,
    view: &View,
    comp: &Composition,
) -> Result<AppWidget, AppError> {
    app_with(texts, locale, view, comp, Arc::new(NoRecords))
}

/// The window for `comp`, with the blocks that draw records given them by `records`.
///
/// # Errors
/// The shell could not be made (no screen, a name that is empty).
pub fn app_with(
    texts: &Mf2Texts,
    locale: &Locale,
    view: &View,
    comp: &Composition,
    records: Arc<dyn Records>,
) -> Result<AppWidget, AppError> {
    // the words of the board are asked by key while drawing; they are read once, here, from the host's own MF2 files
    let said: std::collections::HashMap<String, String> = KANBAN_KEYS
        .iter()
        .map(|k| ((*k).to_owned(), texts.plain(&k.replace('.', "-"))))
        .collect();
    let ctx = Ctx {
        records,
        say: Arc::new(move |key: &str| said.get(key).cloned().unwrap_or_else(|| key.to_owned())),
    };
    let labels = states::Labels::for_locale(locale);
    let title = if view.title.is_empty() {
        texts.plain("host-title")
    } else {
        view.title.clone()
    };
    let mut screens: Vec<Screen> = comp
        .screens
        .iter()
        .map(|s| {
            let page = Page {
                ctx: ctx.clone(),
                empty: texts.plain("host-page-empty"),
                parts: parts(texts, &s.nodes),
                labels: labels.clone(),
            };
            Screen::new(s.page.clone(), s.page.clone(), Arc::new(page))
        })
        .collect();
    if !comp.notices.is_empty() || screens.is_empty() {
        let lines: Vec<String> = comp.notices.iter().map(|c| line(texts, c)).collect();
        let page = Page {
            ctx: ctx.clone(),
            empty: texts.plain("host-page-empty"),
            parts: vec![Part::Locked {
                name: String::new(),
                lines,
            }],
            labels,
        };
        screens.push(Screen::new(
            "notices",
            texts.plain("host-screen-error"),
            Arc::new(page),
        ));
    }
    let widget = AppWidget::new(title, screens)?;
    let entries: Vec<MenuEntry> = view
        .menu
        .iter()
        .filter_map(|m| menu_entry(texts, m))
        .collect();
    if !entries.is_empty() {
        widget.set_menus(vec![Menu::new(texts.plain("host-title"), entries)], |_| {})?;
    }
    let status = match comp.state {
        HostState::Ready => Status::Info(texts.plain("host-status-ready")),
        HostState::Partial => Status::Info(texts.plain("host-status-partial")),
        HostState::Refused => Status::Problem(texts.plain("host-status-refused")),
    };
    widget.set_status(Some(status));
    Ok(widget)
}

#[cfg(test)]
mod tests {
    use super::notice_items;

    #[test]
    fn a_notice_counts_one_per_line_and_names_the_part_in_the_first() {
        let lines = vec!["a".to_owned(), "b".to_owned()];
        assert_eq!(
            notice_items("side", &lines),
            (vec!["side: a".to_owned(), "b".to_owned()], 2)
        );
        assert_eq!(notice_items("", &lines), (lines.clone(), 2));
        assert_eq!(notice_items("x", &[]), (Vec::new(), 0));
    }

    use super::{Part, line, parts};
    use crate::compose::Node;
    use crate::view::Coded;
    use crate::words::words;
    use fs_components_words::Locale;

    #[test]
    fn a_code_without_a_sentence_is_shown_as_its_code() {
        let t = words(&Locale::english());
        let unknown = Coded::new("bind-error-locked").with("part", "page:board");
        assert_eq!(line(&t, &unknown), "bind-error-locked part=page:board");
        let known = Coded::new("host-error-no-pages");
        let l = line(&t, &known);
        assert!(
            l.contains("host-error-no-pages") && l.contains("The layout has no pages"),
            "{l}"
        );
    }

    #[test]
    fn every_code_of_a_locked_part_is_a_line() {
        let t = words(&Locale::english());
        let node = Node::Locked {
            place: Some("board".into()),
            codes: vec![Coded::new("a-code"), Coded::new("b-code")],
        };
        match &parts(&t, &[node])[0] {
            Part::Locked { name, lines } => assert_eq!((name.as_str(), lines.len()), ("board", 2)),
            other => panic!("{other:?}"),
        }
    }
}
