//! `SampleModule` and its `ModuleCore` implementation.
//!
//! Pattern: Fake Object (Test Double) behind the port `ModuleCore`; see the parent module for the departure from the usual fake.

#![allow(clippy::unused_async_trait_impl, clippy::unnecessary_literal_bound)] // nothing to await; the trait is async

use std::cell::RefCell;

use fs_interfaces::forms::{Form, parse_key};
use fs_interfaces::module::{
    Applied, Blob, ChangeFeed, ChangePage, ChangesQuery, Context, Invocation, ModuleCore, Origin,
    Page, PageQuery, Refusal, RightsRules, StepRef,
};
use fs_types::BasicValue;

use super::apply::refuse;
use super::declared;
use super::rules::SampleRules;
use super::store::{Entry, Store};
use crate::error::{Failure, SampleCode};
use crate::project::{self, Table};
use crate::provider::SAMPLE_PROVIDER;
use crate::set::SampleSet;

/// The level the module speaks (`FormSource` level 2: change, take back, watch).
const LEVEL: u32 = 2;
/// How many records or changes a page has when the caller says 0.
const DEFAULT_LIMIT: usize = 50;

/// A `SampleSet` as a module.
pub struct SampleModule {
    store: RefCell<Store>,
    rules: SampleRules,
    feed: RefCell<Vec<ChangeFeed>>,
}

impl SampleModule {
    /// The module of `set`; its rights are the `forbid` wishes of the spec.
    ///
    /// # Errors
    /// `form-unknown` when a form of the set is not in the catalog.
    pub fn new(set: &SampleSet) -> Result<Self, Failure> {
        Ok(Self {
            store: RefCell::new(Store::new(project::tables(set)?)),
            rules: SampleRules::new(set.forbid.clone()),
            feed: RefCell::new(Vec::new()),
        })
    }

    /// Runs the commands of the table with the key of `form` from `form` instead of the catalog's declaration (for a declaration
    /// the pinned catalog does not carry yet, e.g. the `writes` of a newer minor). The data stay as they are.
    #[must_use]
    pub fn declare(self, form: &Form) -> Self {
        let key = form.key();
        for t in &mut self.store.borrow_mut().tables {
            if t.key == key {
                t.form = form.clone();
            }
        }
        self
    }

    /// The table the key (`name@x.y.z`, or the short `name@x.y`; the patch does not count) names.
    fn table_of(tables: &[Table], key: &str) -> Result<usize, Refusal> {
        let (name, want) = parse_key(key).ok_or_else(unknown_form)?;
        tables
            .iter()
            .position(|t| {
                parse_key(&t.key).is_some_and(|(n, v)| {
                    n == name && v.major == want.major && v.minor == want.minor
                })
            })
            .ok_or_else(unknown_form)
    }

    fn origin(cx: &Context) -> Origin {
        Origin::new(
            &cx.person.id.canonical(),
            &cx.claimed.program,
            &cx.claimed.instance,
            &cx.claimed.device,
        )
    }

    /// Tells the sinks of the feed; no borrow of the store is held (a sink must not call back, see `ChangeFeed`).
    fn tell(&self, entry: Option<&Entry>) {
        if let Some(e) = entry {
            let change = e.change_ref();
            self.feed.borrow().iter().for_each(|sink| sink(&change));
        }
    }
}

fn unknown_form() -> Refusal {
    refuse("unknown-form", 404, SampleCode::FormUnknown)
}

fn applied(e: &Entry) -> Applied {
    let mut a = Applied::default();
    if e.after.is_some() {
        a.record.clone_from(&e.object);
    }
    "ok".clone_into(&mut a.text);
    a.change = Some(e.change_ref());
    a
}

fn cursor(text: &str, len: usize) -> Result<usize, Refusal> {
    if text.is_empty() {
        return Ok(0);
    }
    text.parse::<usize>()
        .ok()
        .filter(|n| *n <= len)
        .ok_or_else(|| refuse("bad-arguments", 400, SampleCode::CursorBad))
}

fn window(limit: u32) -> usize {
    match usize::try_from(limit) {
        Ok(0) | Err(_) => DEFAULT_LIMIT,
        Ok(n) => n.min(500),
    }
}

impl ModuleCore for SampleModule {
    fn level(&self) -> u32 {
        LEVEL
    }

    fn provider(&self) -> &str {
        SAMPLE_PROVIDER
    }

    fn rules(&self) -> &dyn RightsRules {
        &self.rules
    }

    async fn page(&self, _cx: &Context, q: PageQuery) -> Result<Page, Refusal> {
        let store = self.store.borrow();
        let table = &store.tables[Self::table_of(&store.tables, &q.form)?];
        let (on_record, on_set) = declared::offered(&table.form);
        let alive: Vec<_> = table.recs.iter().filter(|r| r.alive).collect();
        let from = cursor(&q.cursor, alive.len())?;
        let end = from.saturating_add(window(q.limit)).min(alive.len());
        let mut page = Page::default();
        page.records = alive[from..end]
            .iter()
            .map(|r| project::view(r, on_record.clone()))
            .collect();
        page.total = Some(alive.len() as u64);
        page.may = on_set;
        if end < alive.len() {
            page.next = end.to_string();
        }
        Ok(page)
    }

    async fn blob(&self, _cx: &Context, _record: &str, field: &str) -> Result<Blob, Refusal> {
        let mut r = refuse("unsupported", 501, SampleCode::BlobUnsupported);
        r.detail = format!("{}: {field}", r.detail);
        Err(r)
    }

    async fn apply(&self, cx: &Context, command: Invocation) -> Result<Applied, Refusal> {
        let (done, told) = {
            let mut store = self.store.borrow_mut();
            let had = store.log.len();
            let at = store.invoke(&Self::origin(cx), &command)?;
            (applied(&store.log[at]), store.log.get(had).cloned())
        };
        self.tell(told.as_ref());
        Ok(done)
    }

    async fn revert(&self, cx: &Context, change: &str, step: StepRef) -> Result<Applied, Refusal> {
        let (done, told) = {
            let mut store = self.store.borrow_mut();
            let had = store.log.len();
            let at = store.revert(&Self::origin(cx), change, &step)?;
            (applied(&store.log[at]), store.log.get(had).cloned())
        };
        self.tell(told.as_ref());
        Ok(done)
    }

    async fn changes(&self, cx: &Context, q: ChangesQuery) -> Result<ChangePage, Refusal> {
        let store = self.store.borrow();
        let all: Vec<_> = store
            .log
            .iter()
            .rev()
            .filter(|e| {
                if q.run.is_empty() {
                    e.origin.program == cx.claimed.program
                } else {
                    e.run == q.run
                }
            })
            .collect();
        let from = cursor(&q.cursor, all.len())?;
        let end = from.saturating_add(window(q.limit)).min(all.len());
        let mut page = ChangePage::default();
        page.changes = all[from..end].iter().map(|e| e.change_ref()).collect();
        if end < all.len() {
            page.next = end.to_string();
        }
        Ok(page)
    }

    fn feed(&self, sink: ChangeFeed) {
        self.feed.borrow_mut().push(sink);
    }
}
