//! One screen of the window: the parts of a page laid out in the regions of its frame. The frame `workbench` has a `side` strip
//! beside the `main` area (block contract: parts `side` and `main`); a part whose placement is named `side` stands in the strip,
//! every other part in the main area. A wide part takes the full width of its region and scrolls sideways where it is wider
//! (the board does so itself); the main area scrolls down.
//!
//! Pattern: Adapter (the parts of the composition in, the elements of `fs-components` out). The split is a pure function
//! ([`split`]), so the rule of the regions is testable without a window.

use std::sync::{Arc, Mutex};

use fs_components::blinc::layout::div::{Div, div};
use fs_components::blinc::layout::widgets::scroll::{Scroll, ScrollPhysics};
use fs_components_app::{Element, Surface};
use fs_components_widgets::states::{self, EmptyState};

use crate::draw::{self, Ctx};
use crate::parts::{Part, notice};
use crate::unfinished;

/// The frame whose page has a strip beside the main area.
const WORKBENCH: &str = "workbench";
/// The placement that stands in the strip.
const SIDE: &str = "side";
/// The width of the strip (the side list), in pixels; a narrower window gives the main area at least [`MIN_MAIN`].
const SIDE_W: f32 = 220.0;
/// The main area is never narrower than this; below it the strip gives way.
const MIN_MAIN: f32 = 320.0;
/// The padding of a region.
const PAD: f32 = 24.0;

/// The parts of one page, by region.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub(crate) struct Regions {
    /// The strip beside the main area.
    pub side: Vec<Part>,
    /// Everything else.
    pub main: Vec<Part>,
}

fn place_of(part: &Part) -> &str {
    match part {
        Part::Block { place, .. } => place,
        Part::Locked { name, .. } => name,
        Part::Empty(_) => "",
    }
}

/// The parts by region: in a `workbench` page the placement `side` goes to the strip, the others (in order) to the main area.
/// Another frame has no strip: everything is main.
pub(crate) fn split(frame: &str, parts: &[Part]) -> Regions {
    let mut out = Regions::default();
    for part in parts {
        if frame == WORKBENCH && place_of(part) == SIDE {
            out.side.push(part.clone());
        } else {
            out.main.push(part.clone());
        }
    }
    out
}

/// The widths of the strip and of the main area for a window of `width`; no strip when there is none to show.
pub(crate) fn widths(width: f32, has_side: bool) -> (f32, f32) {
    if !has_side {
        return (0.0, width);
    }
    let side = SIDE_W.min((width - MIN_MAIN).max(0.0));
    (side, width - side)
}

/// A page of the window.
pub(crate) struct Page {
    pub ctx: Ctx,
    pub empty: String,
    pub frame: String,
    pub parts: Vec<Part>,
    pub labels: states::Labels,
}

impl Page {
    /// The elements of `parts`, each as wide as `width` allows.
    fn column(&self, parts: &[Part], width: f32, height: f32) -> Div {
        let inner = (width - 2.0 * PAD).max(0.0);
        let mut column = div().flex_col().gap_px(16.0).p_px(PAD).w(width);
        for part in parts {
            column = match part {
                Part::Locked { name, lines } => {
                    column.child(notice(&self.labels, name, lines).build())
                }
                Part::Empty(sentence) => column.child(self.empty_slot("page", sentence)),
                Part::Block {
                    place,
                    block,
                    title,
                    ..
                } if block == "empty-state" => {
                    column.child(self.empty_slot(place, title.as_deref().unwrap_or(&self.empty)))
                }
                Part::Block { place, block, .. } => {
                    match draw::block(
                        block,
                        &self.ctx,
                        inner,
                        (height - 2.0 * PAD).max(draw::MIN_H),
                    ) {
                        Some(element) => column.child(element),
                        None => column
                            .child(states::Skeleton::new(&format!("{place} ({block})")).build()),
                    }
                }
            };
        }
        column
    }

    /// An empty slot: the sentence only when nothing is there; with records present the slot is not bound yet and says so as a
    /// tooltip ("not finished yet"), never "empty".
    fn empty_slot(&self, place: &str, sentence: &str) -> Div {
        match unfinished::slot(unfinished::has_data(&*self.ctx.records)) {
            unfinished::Slot::Empty => div().child(EmptyState::new(sentence).build()),
            unfinished::Slot::Unfinished => unfinished::unfinished(&self.ctx.locale, place, place),
        }
    }

    /// A region that scrolls down: `parts` in a column of `width`, in a box of `height`.
    fn region(&self, parts: &[Part], width: f32, height: f32) -> Div {
        let physics = Arc::new(Mutex::new(ScrollPhysics::default()));
        div().w(width).h(height).child(
            Scroll::with_physics(physics)
                .w(width)
                .h(height)
                .content(self.column(parts, width, height)),
        )
    }
}

impl Surface for Page {
    fn build(&self, width: f32, height: f32) -> Box<dyn Element> {
        let regions = split(&self.frame, &self.parts);
        let (side_w, main_w) = widths(width, !regions.side.is_empty());
        let mut row = div().flex_row().w(width).h(height);
        if side_w > 0.0 {
            row = row.child(
                self.region(&regions.side, side_w, height)
                    .class("fs-forms-side"),
            );
        }
        Box::new(row.child(self.region(&regions.main, main_w, height)))
    }

    fn styles(&self) -> String {
        format!(
            "{}{}{}.fs-forms-side {{ flex-shrink: 0; border-right-width: 1px; border-right-color: var(--border); background: var(--surface); }}\n",
            fs_components_workbenches::people::styles(),
            states::STYLES,
            fs_components_widgets::disabled::STYLES,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{MIN_MAIN, SIDE_W, split, widths};
    use crate::parts::Part;

    fn block(place: &str) -> Part {
        Part::Block {
            place: place.to_owned(),
            block: "board".to_owned(),
            title: None,
        }
    }

    #[test]
    fn in_a_workbench_the_side_goes_beside_the_main_area_and_the_order_is_kept() {
        let parts = [block("main"), block("side"), block("detail")];
        let r = split("workbench", &parts);
        assert_eq!(r.side, vec![block("side")]);
        assert_eq!(r.main, vec![block("main"), block("detail")]);
    }

    #[test]
    fn a_locked_side_stays_in_the_strip_and_another_frame_has_no_strip() {
        let locked = Part::Locked {
            name: "side".into(),
            lines: vec!["x".into()],
        };
        assert_eq!(
            split("workbench", std::slice::from_ref(&locked)).side,
            vec![locked.clone()]
        );
        let r = split("detail-page", &[block("side"), locked]);
        assert_eq!(r.side, Vec::<Part>::new());
        assert_eq!(r.main.len(), 2);
    }

    #[test]
    fn the_strip_gives_way_in_a_narrow_window_and_the_widths_add_up() {
        assert_eq!(widths(1000.0, false), (0.0, 1000.0));
        assert_eq!(widths(1000.0, true), (SIDE_W, 1000.0 - SIDE_W));
        let (side, main) = widths(400.0, true);
        assert!((side - 80.0).abs() < f32::EPSILON && (main - MIN_MAIN).abs() < f32::EPSILON);
        assert_eq!(widths(100.0, true), (0.0, 100.0));
    }
}
