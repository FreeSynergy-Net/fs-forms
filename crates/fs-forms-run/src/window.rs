//! The window: one screen per page of the composition, a menu from the plan, a status line that says how the host stands. Every
//! part is a block of `fs-components` (`partial-notice`, `empty-state`, `skeleton`); no widget is painted by hand.
//!
//! Pattern: Adapter: the composition in, a shell of `fs-components-app` out. The sentences are made before drawing (see `parts`),
//! so the surfaces only draw.

use std::sync::Arc;

use fs_components_app::{AppError, AppWidget};
use fs_components_kanban::{WORD_KEYS, WORD_TEXTS, word_id};
use fs_components_mf2::Mf2Texts;
use fs_components_widgets::states;
use fs_components_words::{Locale, Localized};

use crate::after_write::AfterWrite;
use crate::clock::Clock;
use crate::compose::{Composition, HostState, Node, ScreenTree};
use crate::draw::Ctx;
use crate::error::HostError;
use crate::history::{Edit, edit_menu_for};
use crate::menus::program_menu;
use crate::records::{NoRecords, Records};
use crate::repaint::BlincRepaint;
use crate::style::{StyleCell, StyleSource, resolve};
use crate::view::{Coded, View};
use crate::words::words;

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
        lacking: Vec::new(),
        notices: Vec::new(),
    }
}

/// The head of a column of the member list, by channel (the words come from the MF2 files, `host-channel-*`).
const CHANNEL_KEYS: &[&str] = &["channel.label", "channel.role", "channel.level"];

/// The sentence of each key the board asks for: the words of the board come from its own port (`WORD_KEYS`, `WORD_TEXTS`), the
/// heads of the member list from the host's files (MF2 ids carry a hyphen where the key has a dot).
fn kanban_words(texts: &Mf2Texts, locale: &Locale) -> std::collections::HashMap<String, String> {
    let board = Mf2Texts::new(WORD_TEXTS, locale);
    let from = |t: &Mf2Texts, k: &str| (k.to_owned(), t.plain(&word_id(k)));
    WORD_KEYS
        .iter()
        .map(|k| from(&board, k))
        .chain(CHANNEL_KEYS.iter().map(|k| from(texts, k)))
        .collect()
}

/// The drawing context of a window: the style (asked again at every drawing), the records, the words of the board, the actions.
pub(crate) fn ctx_of(
    texts: &Mf2Texts,
    locale: &Locale,
    view: &View,
    records: Arc<dyn Records>,
    program_style: Arc<dyn StyleSource>,
    (after_write, clock): (AfterWrite, Arc<dyn Clock>),
) -> Ctx {
    // the words of the board are asked by key while drawing; they are read once, here, from the host's own MF2 files
    let said = kanban_words(texts, locale);
    let named = view.style.clone();
    Ctx {
        style: Arc::new(move || resolve(named.as_deref(), program_style.current())),
        records,
        say: Arc::new(move |key: &str| said.get(key).cloned().unwrap_or_else(|| key.to_owned())),
        actions: Arc::new(view.actions.clone()),
        locale: locale.clone(),
        mounts: Arc::default(),
        after_write,
        feeds: Arc::new(view.feeds_now()),
        clock,
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
/// that style, asked again at every drawing). "Now" is the clock of the records (`Records::clock`).
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
    let clock = records.clock();
    app_clocked(texts, locale, view, comp, records, program_style, clock)
}

/// [`app_styled`] with the clock of the view named: `Opened::clock` of `open` (`--sample-now` is decided there, once).
///
/// # Errors
/// As [`app_styled`].
pub fn app_clocked(
    texts: &Mf2Texts,
    locale: &Locale,
    view: &View,
    comp: &Composition,
    records: Arc<dyn Records>,
    program_style: Arc<dyn StyleSource>,
    clock: Arc<dyn Clock>,
) -> Result<AppWidget, AppError> {
    window_parts(texts, locale, view, comp, records, program_style, clock).map(|(widget, _)| widget)
}

/// [`app_styled`] and the hook its gates fire after an accepted write.
pub(crate) fn window_parts(
    texts: &Mf2Texts,
    locale: &Locale,
    view: &View,
    comp: &Composition,
    records: Arc<dyn Records>,
    program_style: Arc<dyn StyleSource>,
    clock: Arc<dyn Clock>,
) -> Result<(AppWidget, AfterWrite), AppError> {
    let (sample, down) = (records.is_sample(), records.unavailable());
    let after = AfterWrite::default();
    let ctx = ctx_of(
        texts,
        locale,
        view,
        Arc::clone(&records),
        program_style,
        (after.clone(), clock),
    );
    let labels = states::Labels::for_locale(locale);
    let title = if view.title.is_empty() {
        texts.plain("host-title")
    } else {
        view.title.clone()
    };
    let screens = crate::screens::screens_of(texts, comp, &ctx, &labels, &title);
    let widget = AppWidget::new(title, screens)?;
    // the program's menu and the menu "Edit" (undo, redo); a click or a shortcut reaches the one handler of `history::Edit`
    let (menu_view, menu_locale, menu_records) =
        (view.clone(), locale.clone(), Arc::clone(&records));
    let edit = Edit {
        records,
        locale: locale.clone(),
        repaint: Arc::new(BlincRepaint),
        menus: Arc::new(move || {
            program_menu(&words(&menu_locale), &menu_view)
                .into_iter()
                .chain([edit_menu_for(&*menu_records, &menu_locale)])
                .collect()
        }),
        inner: Arc::new(|_| {}),
        app: Arc::default(),
        after,
    };
    edit.install(&widget)?;
    widget.set_status(Some(crate::status::status(
        texts,
        comp.state,
        &comp.lacking,
        sample,
        down.as_deref(),
    )));
    Ok((widget, edit.after))
}

#[cfg(test)]
mod after_tests;
#[cfg(test)]
mod tests {
    use super::*;
    use crate::words::words;

    #[test]
    fn every_key_of_the_board_has_a_sentence_in_both_languages_never_its_raw_key() {
        for lang in ["en", "de"] {
            let locale = Locale::from_posix(lang).expect("a locale");
            let said = kanban_words(&words(&locale), &locale);
            assert_eq!(said.len(), WORD_KEYS.len() + CHANNEL_KEYS.len());
            for key in WORD_KEYS.iter().chain(CHANNEL_KEYS) {
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
}
