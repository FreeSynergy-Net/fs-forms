//! The window: one screen per page of the composition, a menu from the plan, a status line that says how the host stands. Every
//! part is a block of `fs-components` (`partial-notice`, `empty-state`, `skeleton`); no widget is painted by hand.
//!
//! Pattern: Adapter: the composition in, a shell of `fs-components-app` out. The sentences are made before drawing (see `parts`),
//! so the surfaces only draw.

use std::sync::Arc;

use fs_components_app::{AppError, AppWidget};
use fs_components_mf2::Mf2Texts;
use fs_components_widgets::states;
use fs_components_words::{Locale, Localized};

use crate::compose::{Composition, HostState, Node, ScreenTree};
use crate::draw::Ctx;
use crate::error::HostError;
use crate::menus::program_menu;
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
        lacking: Vec::new(),
        notices: Vec::new(),
    }
}

/// The keys the kanban board asks for (`kanban.refused.*` codes come from the provider; the port names its own codes in `fs_components_kanban::form::words`).
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
    "kanban.refused.ask-unsupported",
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
    let screens = crate::screens::screens_of(texts, comp, &ctx, &labels, &title);
    let widget = AppWidget::new(title, screens)?;
    if let Some(menu) = program_menu(texts, view) {
        widget.set_menus(vec![menu], |_| {})?;
    }
    widget.set_status(Some(crate::status::status(
        texts,
        comp.state,
        &comp.lacking,
        sample,
        down.as_deref(),
    )));
    Ok(widget)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::words::words;

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
}
