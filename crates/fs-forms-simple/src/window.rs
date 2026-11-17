//! The window: name, the real icon (own design) directly next to the title, the content on a page that scrolls.
//!
//! Pattern: Adapter: our blocks in, a screen of the fs-components shell out; no widget is painted by hand.

use std::sync::{Arc, Mutex};

use fs_components::blinc::layout::div::div;
use fs_components::blinc::layout::image::img;
use fs_components::blinc::layout::widgets::scroll::{Scroll, ScrollPhysics};
use fs_components_app::{AppError, AppWidget, Element, Menu, MenuEntry, Screen, Status, Surface};
use fs_components_mf2::Mf2Texts;
use fs_components_widgets::file_preview::picture_address;
use fs_components_widgets::{Block, Inline, TextView, styles};
use fs_components_words::{Args, Words};

use crate::content::{coming, start};
use crate::link::Link;

/// The icon of the program (our own drawing, MIT OR Apache-2.0; `packaging/ICON.md`), compiled in as a data address.
const ICON_PNG: &[u8] = include_bytes!("../../../packaging/net.freesynergy.Forms.png");

/// Side of the icon in the window, in pixels.
const ICON_SIZE: f32 = 64.0;
/// Width of the box of the title, in pixels (the text view spreads over what it gets).
const TITLE_WIDTH: f32 = 300.0;
/// How far the box is pulled to the icon (Blinc counts margins in units of 4 px: 13 is about 52 px, which cancels the room the text view keeps on its own left).
const TITLE_PULL: f32 = 13.0;

fn icon() -> impl Element + use<> {
    img(picture_address("image/png", ICON_PNG))
        .w(ICON_SIZE)
        .h(ICON_SIZE)
        .contain()
        // Blinc fades a picture in; drawn at once (headless) it would show half transparent.
        .no_fade()
}

/// One screen of the window.
struct Page {
    name: String,
    body: Vec<Block>,
    /// The start page carries the header with the icon; the page "what comes" is the list alone.
    header: bool,
}

impl Surface for Page {
    fn build(&self, width: f32, height: f32) -> Box<dyn Element> {
        let mut column = div().flex_col().gap(16.0).p(24.0);
        if self.header {
            let title = TextView::new(vec![Block::Heading {
                level: 1,
                text: Inline::plain(self.name.as_str()),
            }])
            .build();
            column = column.child(
                div()
                    .flex_row()
                    .items_center()
                    .gap(16.0)
                    .child(icon())
                    .child(div().w(TITLE_WIDTH).ml(-TITLE_PULL).child(title)),
            );
        }
        let column = column.child(TextView::new(self.body.clone()).build());
        // The page scrolls when it is longer than the window, so nothing is cut off at the bottom.
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
        styles()
    }
}

/// The window for `link`, with the menus whose entries are all locked ("coming soon").
///
/// # Errors
/// The shell could not be made (a name that is empty, for example).
pub fn app(texts: &Mf2Texts, link: &Link) -> Result<AppWidget, AppError> {
    let say = |id: &str| texts.say(&id.into(), &Args::new()).text().to_owned();
    let soon = say("soon");
    let page = |header: bool, body: Vec<Block>| {
        Arc::new(Page {
            name: say("app-name"),
            body,
            header,
        })
    };
    let widget = AppWidget::new(
        say("app-name"),
        vec![
            Screen::new("home", say("screen-start"), page(true, start(texts, link))),
            Screen::new("comes", say("screen-comes"), page(false, coming(texts))),
        ],
    )?;
    let locked = |id: &str| {
        MenuEntry::new(id, format!("{} ({soon})", say(&format!("menu-{id}")))).disabled()
    };
    widget.set_menus(
        vec![Menu::new(
            say("menu-program"),
            vec![locked("new"), locked("open"), locked("export")],
        )],
        |_| {},
    )?;
    widget.set_status(Some(Status::Info(say("status-ready"))));
    Ok(widget)
}
