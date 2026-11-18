//! Layer 1 to layer 2, offline (no provider needed): all problems at once, never only the first, each with its element or line.
//!
//! Pattern: Template method: the checks run in a fixed order; each check is a small function that only appends problems.

use std::collections::HashSet;

use crate::check_text::{check_messages, check_range};
use crate::codes::LayoutCode;
use crate::doc::{BindingDoc, Item, LayoutDoc};
use crate::ids::{ElemId, ElemKind};
use crate::layout::{FORMAT_KNOWN, Layout};
use crate::limits::{Limits, Technical};
use crate::schemes::scheme_names_rule;
use crate::shim::Coded;
use crate::walk::{menu_depth, walk};

type Problems = Vec<Coded<LayoutCode>>;

/// What the check needs to know of the component catalog (stand-in for `fs-components-catalog` until it lands).
pub trait CatalogRules {
    /// Is this block in the catalog?
    fn block_known(&self, block: &str) -> bool;
    /// Does the block have this slot?
    fn slot_known(&self, block: &str, slot: &str) -> bool;
    /// How many bindings the slot takes (`repeat`, 1..=16); `nth` must be below it.
    fn slot_repeat(&self, block: &str, slot: &str) -> u32;
}

/// A catalog that knows everything; for tests and for checks that must not depend on a catalog.
#[derive(Clone, Copy, Debug, Default)]
pub struct OpenCatalog;
impl CatalogRules for OpenCatalog {
    fn block_known(&self, _: &str) -> bool {
        true
    }
    fn slot_known(&self, _: &str, _: &str) -> bool {
        true
    }
    fn slot_repeat(&self, _: &str, _: &str) -> u32 {
        Technical::NTH_MAX
    }
}

impl LayoutDoc {
    /// Layer 1 to layer 2. Format 1 gets natural element ids; format 2 needs them.
    ///
    /// # Errors
    /// Every problem found, as codes with parameters.
    pub fn check(
        mut self,
        catalog: &dyn CatalogRules,
        limits: &Limits,
    ) -> Result<Layout, Problems> {
        let mut out: Problems = Vec::new();
        if self.format == 0 {
            out.push(Coded::new(LayoutCode::FormatInvalid).with("found", self.format));
        } else if self.format > FORMAT_KNOWN {
            out.push(
                Coded::new(LayoutCode::FormatNewer)
                    .with("found", self.format)
                    .with("known", FORMAT_KNOWN),
            );
        }
        if !out.is_empty() {
            return Err(out);
        }
        let count = self.fill_elems(&mut out);
        self.canonical_order();
        if count > limits.elements_max as usize {
            out.push(
                Coded::new(LayoutCode::TooMany)
                    .with("what", "elements")
                    .with("max", limits.elements_max)
                    .with("by", limits.by.elements.as_str()),
            );
        }
        if menu_depth(&self.menu) > limits.menu_depth_max {
            out.push(
                Coded::new(LayoutCode::TooMany)
                    .with("what", "menu-depth")
                    .with("max", limits.menu_depth_max)
                    .with("by", limits.by.menu_depth.as_str()),
            );
        }
        self.check_names(catalog, &mut out);
        check_messages(&self.messages, limits, &mut out);
        if out.is_empty() {
            Ok(Layout::from_checked(self))
        } else {
            Err(out)
        }
    }

    /// Sets natural ids (format 1), reports missing and doubled ids, zeroes the lines; returns the element count.
    fn fill_elems(&mut self, out: &mut Problems) -> usize {
        let format = self.format;
        let mut seen: HashSet<(ElemKind, ElemId)> = HashSet::new();
        let mut count = 0;
        walk(self, &mut |v| {
            count += 1;
            if v.elem.is_none() {
                if format >= 2 {
                    out.push(Coded::new(LayoutCode::ElemMissing).with("line", *v.line));
                } else {
                    let parts: Vec<&str> = v.parts.iter().map(String::as_str).collect();
                    *v.elem = Some(ElemId::natural(v.kind, &parts));
                }
            }
            if let Some(e) = v
                .elem
                .as_ref()
                .filter(|e| !seen.insert((v.kind, (*e).clone())))
            {
                out.push(
                    Coded::new(LayoutCode::ElemTwice)
                        .with("elem", e.as_str())
                        .with("kind", v.kind.as_str()),
                );
            }
            *v.line = 0;
        });
        count
    }

    /// Lists whose order is not part of the meaning get their canonical order: bindings by (slot, nth), messages by (key, lang),
    /// views by (view, owner). The table has no sequence column for them and the exchange file writes them this way.
    fn canonical_order(&mut self) {
        for p in &mut self.pages {
            for b in &mut p.v.blocks {
                b.v.bindings.sort_by(|x, y| {
                    (x.v.slot.as_bytes(), x.v.nth).cmp(&(y.v.slot.as_bytes(), y.v.nth))
                });
            }
        }
        self.messages.sort_by(|x, y| {
            (x.v.key.as_bytes(), x.v.lang.as_bytes())
                .cmp(&(y.v.key.as_bytes(), y.v.lang.as_bytes()))
        });
        self.views.sort_by(|x, y| {
            (
                x.v.view.as_bytes(),
                x.v.owner.as_ref().map(|o| o.0.as_bytes()),
            )
                .cmp(&(
                    y.v.view.as_bytes(),
                    y.v.owner.as_ref().map(|o| o.0.as_bytes()),
                ))
        });
    }

    fn check_names(&self, catalog: &dyn CatalogRules, out: &mut Problems) {
        for r in &self.requires {
            check_range(&r.v.range, out);
        }
        for s in &self.sources {
            check_range(&s.v.range, out);
        }
        let mut pages = HashSet::new();
        for p in &self.pages {
            if !pages.insert(p.v.page.as_str()) {
                out.push(
                    Coded::new(LayoutCode::KeyTwice)
                        .with("kind", "page")
                        .with("key", &p.v.page),
                );
            }
            let mut places = HashSet::new();
            for b in &p.v.blocks {
                let elem = b.elem.as_ref().map_or("", ElemId::as_str);
                if !places.insert(b.v.place.as_str()) {
                    out.push(
                        Coded::new(LayoutCode::KeyTwice)
                            .with("kind", "block")
                            .with("key", format!("{}/{}", p.v.page, b.v.place)),
                    );
                }
                if catalog.block_known(&b.v.block) {
                    check_bindings(&b.v.block, &b.v.bindings, catalog, out);
                } else {
                    out.push(
                        Coded::new(LayoutCode::BlockUnknown)
                            .with("elem", elem)
                            .with("block", &b.v.block),
                    );
                }
            }
        }
    }
}

fn check_bindings(
    block: &str,
    list: &[Item<BindingDoc>],
    catalog: &dyn CatalogRules,
    out: &mut Problems,
) {
    let mut used: HashSet<(&str, u32)> = HashSet::new();
    for b in list {
        let elem = b.elem.as_ref().map_or("", ElemId::as_str);
        let bad = |cause: &str| {
            Coded::new(LayoutCode::BindingInvalid)
                .with("elem", elem)
                .with("cause", cause)
        };
        if !catalog.slot_known(block, &b.v.slot) {
            out.push(bad("slot-unknown"));
        } else if b.v.nth < Technical::NTH_MAX && b.v.nth >= catalog.slot_repeat(block, &b.v.slot) {
            out.push(bad("bound-twice")); // more bindings than the slot's repeat
        }
        if b.v.nth >= Technical::NTH_MAX {
            out.push(
                Coded::new(LayoutCode::TooMany)
                    .with("what", "binding-nth")
                    .with("max", Technical::NTH_MAX)
                    .with("by", "technical")
                    .with("elem", elem),
            );
        }
        if !used.insert((b.v.slot.as_str(), b.v.nth)) {
            out.push(
                Coded::new(LayoutCode::KeyTwice)
                    .with("kind", "binding")
                    .with("key", format!("{}/{}", b.v.slot, b.v.nth)),
            );
        }
        if let Some(s) = &b.v.schemes {
            let names: Vec<String> = s.iter().map(|x| x.v.clone()).collect();
            out.extend(scheme_names_rule(elem, &names));
        }
    }
}
