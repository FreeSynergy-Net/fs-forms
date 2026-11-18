//! `LayoutRead` of the memory store.
//!
//! Pattern: Repository (read side).

use crate::apply::apply;
use crate::codes::LayoutCode;
use crate::ids::{FrontendKey, LayoutId, OverlayId, Stamp};
use crate::layout::Layout;
use crate::memory::{MemoryLayoutStore, State, Stored, ready};
use crate::overlay::{Conflict, Overlay, OverlayScope};
use crate::port::{LayoutRead, Pending};
use crate::providers::{check_providers, split_provider_codes};
use crate::resolved::{LayerInfo, Resolved};
use crate::shim::{Coded, ObjectId};
use crate::types::{Health, LayoutFilter, LogEntry, Page, Summary};

/// Biggest page a caller may ask for.
const PAGE_MAX: u32 = 1000;

fn summary(st: &State, s: &Stored) -> Summary {
    let mine = |o: &&Overlay| o.layout == s.id;
    let ids: Vec<&OverlayId> = st.overlays.iter().filter(mine).map(|o| &o.id).collect();
    Summary {
        id: s.id.clone(),
        key: s.key.clone(),
        origin: s.origin,
        package: s.package.clone(),
        owner: s.owner.clone(),
        rev: s.rev,
        updated: s.updated,
        deleted: s.deleted,
        overlays: u32::try_from(ids.len()).unwrap_or(u32::MAX),
        open_conflicts: u32::try_from(
            st.conflicts
                .iter()
                .filter(|c| c.open && ids.contains(&&c.overlay))
                .count(),
        )
        .unwrap_or(u32::MAX),
    }
}

impl MemoryLayoutStore {
    /// An overlay is visible when it is an instance overlay or the person's own.
    pub(crate) fn visible(&self, o: &Overlay) -> bool {
        match &o.scope {
            OverlayScope::Instance => true,
            OverlayScope::Person(p) => *p == self.actor,
        }
    }
}

impl MemoryLayoutStore {
    /// `resolve` without the live provider check.
    fn resolve_now(&self, key: &FrontendKey) -> Option<Resolved> {
        let st = self.st.borrow();
        let i = st.live_key(key)?;
        let s = &st.layouts[i];
        let orig = s.layout.as_ref()?;
        let overlays: Vec<Overlay> = st
            .overlays
            .iter()
            .filter(|o| o.layout == s.id && self.visible(o))
            .cloned()
            .collect();
        let locks = st.locks_of(&s.id);
        let version = s.package.as_ref().map_or("", |p| p.version.as_str());
        let applied = apply(orig, &overlays, &locks, version);
        let conflicts = st
            .conflicts
            .iter()
            .filter(|c| c.open && overlays.iter().any(|o| o.id == c.overlay))
            .cloned()
            .collect();
        let layers = overlays
            .iter()
            .map(|o| LayerInfo {
                overlay: o.id.clone(),
                scope: o.scope.clone(),
                state: o.state,
                rev: o.rev,
            })
            .collect();
        Some(Resolved {
            id: s.id.clone(),
            key: s.key.clone(),
            rev: s.rev,
            stamp: Stamp(st.stamp),
            effective_digest: applied.layout.digest(),
            layout: applied.layout,
            layers,
            conflicts,
            locks,
            provenance: applied.provenance,
            problems: Vec::new(),
            warnings: Vec::new(),
        })
    }
}

impl LayoutRead for MemoryLayoutStore {
    fn stamp(&self) -> Pending<'_, Stamp> {
        ready(Ok(Stamp(self.st.borrow().stamp)))
    }
    fn health(&self) -> Pending<'_, Health> {
        ready(Ok(Health::Ok))
    }
    fn list<'a>(
        &'a self,
        filter: &'a LayoutFilter,
        after: Option<&'a LayoutId>,
        size: u32,
    ) -> Pending<'a, Page<Summary>> {
        if size == 0 || size > PAGE_MAX {
            return ready(Err(Coded::new(LayoutCode::PageSize)
                .with("size", size)
                .with("max", PAGE_MAX)));
        }
        let st = self.st.borrow();
        let hit = |s: &&Stored| {
            (filter.with_deleted || s.deleted.is_none())
                && filter.extends.as_ref().is_none_or(|e| *e == s.key.extends)
                && filter.origin.is_none_or(|o| o == s.origin)
                && filter
                    .package
                    .as_ref()
                    .is_none_or(|p| s.package.as_ref().is_some_and(|x| &x.package == p))
        };
        let all: Vec<&Stored> = st.layouts.iter().filter(hit).collect();
        let start = match after {
            None => 0,
            Some(a) => match all.iter().position(|s| &s.id == a) {
                Some(i) => i + 1,
                None => return ready(Err(Coded::new(LayoutCode::CursorInvalid))),
            },
        };
        let rest = &all[start.min(all.len())..];
        let items: Vec<Summary> = rest
            .iter()
            .take(size as usize)
            .map(|s| summary(&st, s))
            .collect();
        let next = (rest.len() > size as usize)
            .then(|| items.last().map(|s| s.id.clone()))
            .flatten();
        ready(Ok(Page { items, next }))
    }
    fn original<'a>(&'a self, id: &'a LayoutId) -> Pending<'a, Option<(Layout, Summary)>> {
        let st = self.st.borrow();
        let found = st.live(id).and_then(|i| {
            st.layouts[i]
                .layout
                .clone()
                .map(|l| (l, summary(&st, &st.layouts[i])))
        });
        ready(Ok(found))
    }
    fn resolve<'a>(&'a self, key: &'a FrontendKey) -> Pending<'a, Option<Resolved>> {
        Box::pin(async move {
            let Some(mut r) = self.resolve_now(key) else {
                return Ok(None);
            };
            if let Some(p) = &self.providers {
                let codes = check_providers(&r.layout, &**p).await?;
                (r.problems, r.warnings) = split_provider_codes(codes);
            }
            Ok(Some(r))
        })
    }
    fn overlays<'a>(&'a self, layout: &'a LayoutId) -> Pending<'a, Vec<Overlay>> {
        let st = self.st.borrow();
        ready(Ok(st
            .overlays
            .iter()
            .filter(|o| &o.layout == layout && self.visible(o))
            .cloned()
            .collect()))
    }
    fn overlay<'a>(&'a self, id: &'a OverlayId) -> Pending<'a, Option<Overlay>> {
        let st = self.st.borrow();
        ready(Ok(st
            .overlays
            .iter()
            .find(|o| &o.id == id && self.visible(o))
            .cloned()))
    }
    fn conflicts<'a>(
        &'a self,
        layout: Option<&'a LayoutId>,
        open_only: bool,
    ) -> Pending<'a, Vec<Conflict>> {
        let st = self.st.borrow();
        let of_layout = |c: &&Conflict| {
            layout.is_none_or(|l| {
                st.overlays
                    .iter()
                    .any(|o| o.id == c.overlay && &o.layout == l && self.visible(o))
            })
        };
        ready(Ok(st
            .conflicts
            .iter()
            .filter(of_layout)
            .filter(|c| c.open || !open_only)
            .cloned()
            .collect()))
    }
    fn log<'a>(&'a self, after: Option<&'a ObjectId>, size: u32) -> Pending<'a, Vec<LogEntry>> {
        let st = self.st.borrow();
        let start = after
            .and_then(|a| st.log.iter().position(|l| &l.id == a))
            .map_or(0, |i| i + 1);
        ready(Ok(st
            .log
            .iter()
            .skip(start)
            .take(size as usize)
            .cloned()
            .collect()))
    }
}
