//! The blocks of `fs-components` this host draws as elements of a page, and the data each one is given. Which block has which widget is
//! the catalog's (`widget` of the contract) and which component draws a widget is `fs_components::views()`; the host only feeds the
//! view with the form the block names (`forms=` of the contract) and keeps the mounted component over a rebuild, so its state stays.
//! There is no name of a block here except `sidebar`, the one composition left (it needs `tree@1.1.0`); nothing is painted here and
//! nothing is decided here. A control is unlocked only when the description has an action for it and the records can do it (see
//! `write_gate`); every other one is drawn locked with its hint, never silently.
//!
//! Pattern: Abstract Factory over `fs-components` (block name in, element out), data through the port `Records`.

use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use fs_components::blinc::layout::div::Div;
use fs_components::blinc::theme::{ColorScheme, ThemeState};
use fs_components::views;
use fs_components_core::form_read::FormInput;
use fs_components_core::form_view::{DrawCtx, FormView, Mounted};
use fs_components_fields::Say;
use fs_components_styles::StyleId;
use fs_components_words::Locale;

use crate::form_data::{empty_input, input_of};
use crate::records::Records;
use crate::view::ActionView;
use crate::write_gate::WriteGate;

mod compositions;

use compositions::sidebar;

/// The least height of the board, in pixels.
pub(crate) const MIN_H: f32 = 380.0;

/// A component mounted for a block of a page, with the look it was mounted for.
pub(crate) struct Slot {
    block: String,
    look: (StyleId, bool),
    mounted: Arc<dyn Mounted>,
}

/// The components of a page, kept over a rebuild: their state (selection, search, an open question) lives in them.
pub(crate) type MountSlot = Arc<Mutex<Vec<Slot>>>;

/// What a drawn block needs besides its name.
#[derive(Clone)]
pub(crate) struct Ctx {
    /// The style of the window NOW (asked at every drawing).
    pub style: Arc<dyn Fn() -> StyleId + Send + Sync>,
    pub records: Arc<dyn Records>,
    pub say: Arc<dyn Say>,
    /// The `[[action]]`s of the description: only what they name is unlocked.
    pub actions: Arc<Vec<ActionView>>,
    /// The language of the reader.
    pub locale: Locale,
    /// The components of this page (one set per page).
    pub mounts: MountSlot,
}

/// Whether the reader of this window sees the dark scheme.
fn dark() -> bool {
    ThemeState::try_get().is_some_and(|t| t.scheme() == ColorScheme::Dark)
}

/// The views of `fs-components`, made once.
fn all_views() -> &'static [Box<dyn FormView>] {
    static VIEWS: OnceLock<Vec<Box<dyn FormView>>> = OnceLock::new();
    VIEWS.get_or_init(views)
}

/// The view of the widget the catalog names for `block`.
fn view_of(block: &str) -> Option<&'static dyn FormView> {
    let widget = crate::blocks::widget_of(block)?;
    all_views()
        .iter()
        .find(|v| v.widget() == widget)
        .map(|v| &**v)
}

/// The form `block` is fed with (the first one its contract names), as the module gives it now.
fn input_for(records: &dyn Records, block: &str) -> FormInput {
    let name = crate::blocks::forms_of(block)
        .first()
        .map(|f| f.name().to_owned())
        .unwrap_or_default();
    records
        .forms()
        .iter()
        .find(|d| d.declared.form.name == name)
        .map_or_else(|| empty_input(&name), input_of)
}

/// Whether a function draws the block: the one composition, or a component with a view.
pub(crate) fn has(name: &str) -> bool {
    name == "sidebar" || view_of(name).is_some()
}

/// Why a block of the catalog is not drawn, in words for the list: its widget has no view, or the contract names none.
pub(crate) fn missing(name: &str) -> String {
    match crate::blocks::widget_of(name) {
        Some(w) => format!("widget {w} has no view"),
        None => "no widget in the contract".to_owned(),
    }
}

/// What the view is given besides the data.
fn draw_ctx(ctx: &Ctx, block: &str, look: (StyleId, bool)) -> DrawCtx {
    let form = crate::blocks::forms_of(block)
        .first()
        .map(|f| f.name().to_owned())
        .unwrap_or_default();
    let (records, block, say) = (ctx.records.clone(), block.to_owned(), ctx.say.clone());
    DrawCtx {
        look,
        say: Arc::new(move |key: &str| say.say(key)),
        locale: ctx.locale.clone(),
        write: Arc::new(WriteGate {
            records: ctx.records.clone(),
            actions: ctx.actions.clone(),
            form,
        }),
        reread: Arc::new(move || input_for(&*records, &block)),
    }
}

/// The block as an element, or `None` when this host has no drawing for it (the caller says so).
pub(crate) fn block(name: &str, ctx: &Ctx, width: f32, height: f32) -> Option<Div> {
    if name == "sidebar" {
        return Some(sidebar(ctx, width, height));
    }
    let view = view_of(name)?;
    let look = ((ctx.style)(), dark());
    let input = input_for(&*ctx.records, name);
    let mounted = {
        let mut slots = ctx.mounts.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(slot) = slots.iter().find(|s| s.block == name && s.look == look) {
            slot.mounted.refresh(input);
            slot.mounted.clone()
        } else {
            let mounted: Arc<dyn Mounted> = Arc::from(view.mount(input, draw_ctx(ctx, name, look)));
            slots.retain(|s| s.block != name);
            slots.push(Slot {
                block: name.to_owned(),
                look,
                mounted: mounted.clone(),
            });
            mounted
        }
    };
    Some(mounted.build(width, height))
}

#[cfg(test)]
mod tests {
    use super::all_views;

    #[test]
    fn every_view_is_the_widget_of_a_block_of_the_catalog() {
        for view in all_views() {
            assert!(
                crate::blocks::names_widget(view.widget()),
                "view {} matches no widget in the catalog",
                view.widget()
            );
        }
    }
}
