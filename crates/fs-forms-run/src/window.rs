//! The window: one screen per page of the composition, a menu from the plan, a status line that says how the host stands. Every
//! part is a block of `fs-components` (`partial-notice`, `empty-state`, `skeleton`); no widget is painted by hand.
//!
//! Pattern: Adapter: the composition in, a shell of `fs-components-app` out. The sentences are made before drawing (see `parts`),
//! so the surfaces only draw.

use std::sync::Arc;

use fs_components_app::{AppError, AppWidget, Screen, Status};
use fs_components_mf2::Mf2Texts;
use fs_components_widgets::states;
use fs_components_words::{Locale, Localized};

use crate::compose::{Composition, HostState, Node, ScreenTree};
use crate::draw::Ctx;
use crate::error::HostError;
use crate::menus::program_menu;
use crate::page::Page;
use crate::parts::{Part, line, parts};
use crate::records::{NoRecords, Records};
use crate::style::{StyleCell, StyleSource, resolve};
use crate::view::{Coded, View};

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

/// The keys the kanban board asks for (`kanban.refused.*` codes come from the provider and show as their key until it names them).
const KANBAN_KEYS: &[&str] = &[
    "kanban.add_column",
    "kanban.add_card",
    "kanban.add_card_hint",
    "kanban.add_card_ok",
    "kanban.new_column",
    "kanban.column_name",
    "kanban.rename",
    "kanban.limit",
    "kanban.limit_placeholder",
    "kanban.limit_hint",
    "kanban.limit_invalid",
    "kanban.drop",
    "kanban.grab",
    "kanban.move_left",
    "kanban.move_right",
    "kanban.more_facts",
    "kanban.empty",
    "kanban.soon",
    "kanban.refused.locked",
    "kanban.refused.unsupported",
    "kanban.refused.unreachable",
    "kanban.refused.conflict",
    "kanban.refused.rejected",
    "kanban.refused.no-board",
    "kanban.refused.failed",
];

/// The head of a column of the member list, by channel (the words come from the MF2 files, `host-channel-*`).
const CHANNEL_KEYS: &[&str] = &["channel.label", "channel.role", "channel.level"];

/// The sentence of each key of the board (MF2 ids carry a hyphen where the key has a dot).
fn kanban_words(texts: &Mf2Texts) -> std::collections::HashMap<String, String> {
    KANBAN_KEYS
        .iter()
        .chain(CHANNEL_KEYS)
        .map(|k| ((*k).to_owned(), texts.plain(&k.replace('.', "-"))))
        .collect()
}

/// The drawing context of a window: the style (asked again at every drawing), the records, the words of the board, the actions.
pub(crate) fn ctx_of(
    texts: &Mf2Texts,
    locale: &Locale,
    view: &View,
    records: Arc<dyn Records>,
    program_style: Arc<dyn StyleSource>,
) -> Ctx {
    // the words of the board are asked by key while drawing; they are read once, here, from the host's own MF2 files
    let said = kanban_words(texts);
    let named = view.style.clone();
    Ctx {
        style: Arc::new(move || resolve(named.as_deref(), program_style.current())),
        records,
        say: Arc::new(move |key: &str| said.get(key).cloned().unwrap_or_else(|| key.to_owned())),
        actions: Arc::new(view.actions.clone()),
        locale: locale.clone(),
        mounts: Arc::default(),
    }
}

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
    app_styled(
        texts,
        locale,
        view,
        comp,
        records,
        Arc::new(StyleCell::default()),
    )
}

/// The window for `comp`, with records and the source of the style set for the program (a description that says `default` gets
/// that style, asked again at every drawing).
///
/// # Errors
/// The shell could not be made (no screen, a name that is empty).
pub fn app_styled(
    texts: &Mf2Texts,
    locale: &Locale,
    view: &View,
    comp: &Composition,
    records: Arc<dyn Records>,
    program_style: Arc<dyn StyleSource>,
) -> Result<AppWidget, AppError> {
    let (sample, down) = (records.is_sample(), records.unavailable());
    let ctx = ctx_of(texts, locale, view, records, program_style);
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
                ctx: Ctx {
                    mounts: Arc::default(),
                    ..ctx.clone()
                },
                empty: texts.plain("host-page-empty"),
                frame: s.frame.clone(),
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
            frame: String::new(),
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
    if let Some(menu) = program_menu(texts, view) {
        widget.set_menus(vec![menu], |_| {})?;
    }
    widget.set_status(Some(status(texts, comp.state, sample, down.as_deref())));
    Ok(widget)
}

/// How the host stands; sample data is said in the same line (a made-up record is never shown as a real one), and so is a data
/// source that is not there (`down`: always a problem).
fn status(texts: &Mf2Texts, state: HostState, sample: bool, down: Option<&str>) -> Status {
    if let Some(why) = down {
        return Status::Problem(why.to_owned());
    }
    let (problem, mut text) = match state {
        HostState::Ready => (false, texts.plain("host-status-ready")),
        HostState::Partial => (false, texts.plain("host-status-partial")),
        HostState::Refused => (true, texts.plain("host-status-refused")),
    };
    if sample {
        text = format!("{text} · {}", texts.plain("host-status-sample"));
    }
    if problem {
        Status::Problem(text)
    } else {
        Status::Info(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::words::words;

    fn info(s: &Status) -> &str {
        match s {
            Status::Info(t) | Status::Problem(t) => t,
            _ => "",
        }
    }

    #[test]
    fn every_key_of_the_board_has_a_sentence_in_both_languages_never_its_raw_key() {
        for lang in ["en", "de"] {
            let locale = Locale::from_posix(lang).expect("a locale");
            let said = kanban_words(&words(&locale));
            assert_eq!(said.len(), KANBAN_KEYS.len() + CHANNEL_KEYS.len());
            for key in KANBAN_KEYS.iter().chain(CHANNEL_KEYS) {
                let text = &said[*key];
                assert!(
                    !text.is_empty()
                        && text != key
                        && text != &key.replace('.', "-")
                        && !text.contains("kanban")
                        && !text.contains("channel"),
                    "{lang}: `{key}` shows raw as `{text}`"
                );
            }
        }
    }

    #[test]
    fn sample_data_is_said_in_the_status_in_both_languages_and_real_data_says_nothing() {
        for (lang, said) in [("en", "Sample data"), ("de", "Beispieldaten")] {
            let locale = Locale::from_posix(lang).expect("a locale");
            let texts = words(&locale);
            let with = status(&texts, HostState::Partial, true, None);
            assert!(info(&with).contains(said), "{lang}: {}", info(&with));
            assert!(
                info(&with).contains(&texts.plain("host-status-partial")),
                "the state stays in the line"
            );
            let without = status(&texts, HostState::Partial, false, None);
            assert!(!info(&without).contains(said), "{lang}: {}", info(&without));
            let refused = status(&texts, HostState::Refused, true, None);
            assert!(matches!(refused, Status::Problem(_)), "a problem stays one");
        }
    }

    #[test]
    fn a_data_source_that_is_not_there_is_a_problem_in_the_status_line_with_its_reason() {
        let texts = words(&Locale::from_posix("en").expect("a locale"));
        let down = status(&texts, HostState::Ready, false, Some("no answer: socket x"));
        assert!(matches!(&down, Status::Problem(t) if t == "no answer: socket x"));
    }
}
