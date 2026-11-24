//! The screens of the window: one per page of the composition; the notices of the host (a source that is not there, an unknown style)
//! are a part of the page when there is ONE page, else a screen of their own. With one page the shell shows no list of screens and the
//! window carries its title, never the raw page id (it was `board` in the list, plus an entry "Problem").
//!
//! Pattern: Builder (the screens out of the composition); split off `window.rs` to keep both files short.

use std::sync::Arc;

use fs_components_app::Screen;
use fs_components_mf2::Mf2Texts;
use fs_components_widgets::states;

use crate::compose::Composition;
use crate::draw::Ctx;
use crate::page::Page;
use crate::parts::{Part, line, parts};

/// The screens for `comp`, in the order of its pages.
pub(crate) fn screens_of(
    texts: &Mf2Texts,
    comp: &Composition,
    ctx: &Ctx,
    labels: &states::Labels,
    title: &str,
) -> Vec<Screen> {
    let one_page = comp.screens.len() == 1;
    let notice_lines: Vec<String> = comp.notices.iter().map(|c| line(texts, c)).collect();
    let page_of = |frame: &str, mut parts: Vec<Part>, with_notices: bool| {
        if with_notices {
            parts.push(Part::Locked {
                name: String::new(),
                lines: notice_lines.clone(),
            });
        }
        Arc::new(Page {
            ctx: Ctx {
                mounts: Arc::default(),
                ..ctx.clone()
            },
            empty: texts.plain("host-page-empty"),
            frame: frame.to_owned(),
            parts,
            labels: labels.clone(),
        })
    };
    let own_notices = !comp.notices.is_empty() && one_page;
    let mut screens: Vec<Screen> = comp
        .screens
        .iter()
        .map(|s| {
            let page = page_of(&s.frame, parts(texts, &s.nodes), own_notices);
            // a lone page carries the NAME of the window, never the raw page id
            let label = if one_page { title } else { &s.page };
            Screen::new(s.page.clone(), label.to_owned(), page)
        })
        .collect();
    if (!comp.notices.is_empty() && !one_page) || screens.is_empty() {
        let part = Part::Locked {
            name: String::new(),
            lines: notice_lines.clone(),
        };
        screens.push(Screen::new(
            "notices",
            texts.plain("host-screen-error"),
            page_of("", vec![part], false),
        ));
    }
    screens
}
