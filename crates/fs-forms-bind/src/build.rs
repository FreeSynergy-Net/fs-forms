//! `Bind::build`: layout + catalog + providers -> `Plan`. Pure, synchronous, no I/O, no clock, no panic, no `Result`.
//!
//! Pattern: Builder. Every problem lands as a `PartLock` on the smallest part it concerns, or as a window `Notice`.

use crate::build_menu::{MenuCtx, build_menu};
use crate::build_place::Env;
use crate::build_tree::build_nodes;
use crate::catalog::Catalog;
use crate::coded::Coded;
use crate::codes::{BindCode, Notice, Part, PartLock};
use crate::keys::KeyPlan;
use crate::model::{Problem, Resolved};
use crate::plan::{PagePlan, PlaceStatus, Plan, PlanState, SourcePlan, SourceReady};
use crate::providers::{Offer, Providers, SettingsHealth};

/// The builder; a unit type, so equal inputs give equal plans.
pub struct Bind;

/// The layout problems, each handed out once; what nobody takes is reported as orphan (never dropped).
pub(crate) struct Pool<'a> {
    items: &'a [Problem],
    used: Vec<bool>,
    pub notices: Vec<Notice>,
}

impl<'a> Pool<'a> {
    fn new(items: &'a [Problem]) -> Self {
        Self {
            items,
            used: vec![false; items.len()],
            notices: Vec::new(),
        }
    }

    /// The locking problems of one element (warnings never come here: they are notices).
    pub(crate) fn take(&mut self, elem: &str) -> Vec<Coded> {
        let mut out = Vec::new();
        for (i, p) in self.items.iter().enumerate() {
            if self.used[i] || p.elem.as_deref() != Some(elem) {
                continue;
            }
            self.used[i] = true;
            out.push(coded(p));
        }
        out
    }
}

fn coded(p: &Problem) -> Coded {
    Coded {
        code: p.code.clone(),
        params: p.params.clone(),
    }
}

impl Bind {
    /// Builds the plan. Order: sources, pages and placements (by `seq`), menu, problems nobody took, style, state.
    #[must_use]
    pub fn build(layout: &Resolved, catalog: &dyn Catalog, providers: &Providers<'_>) -> Plan {
        let mut pool = Pool::new(&layout.problems);
        let mut sources = build_sources(layout, providers);
        lock_sources_with_problems(&mut sources, &mut pool);
        let env_host = providers.host;
        let mut pages = Vec::new();
        let mut page_order: Vec<_> = layout.pages.iter().collect();
        page_order.sort_by_key(|p| p.seq);
        for page in page_order {
            let mut locks: Vec<PartLock> = pool
                .take(&page.elem)
                .into_iter()
                .map(|c| PartLock::locked(Part::Page(page.page.clone()), c))
                .collect();
            if page.places.is_empty() {
                locks.push(PartLock::locked(
                    Part::Page(page.page.clone()),
                    BindCode::PageEmpty.coded().with("page", page.page.clone()),
                ));
            }
            let env = Env {
                catalog,
                host: env_host,
                sources: &sources,
                facts: providers.records,
            };
            let places = build_nodes(&env, page, &page.places, &mut pool);
            pages.push(PagePlan {
                page: page.page.clone(),
                elem: page.elem.clone(),
                seq: page.seq,
                frame: page.frame.clone(),
                locks,
                places,
            });
        }
        let ctx = MenuCtx {
            sources: &sources,
            host: env_host,
            platform: providers.platform,
            standard: providers.standard,
        };
        let menu = build_menu(&layout.menu, &ctx, &mut pool);
        let notices = collect_notices(layout, providers, &mut pool);
        let style = providers
            .settings
            .style
            .clone()
            .or_else(|| layout.style.clone());
        let mut plan = Plan {
            key: layout.key.clone(),
            stamp: layout.stamp,
            rev: layout.rev,
            style,
            sources,
            pages,
            menu,
            keys: KeyPlan::default(),
            notices,
            state: PlanState::Ready,
        };
        plan.state = state_of(&plan);
        plan
    }
}

/// Window notices: open conflicts, warnings, layout problems without an element (or whose element is not in the plan), settings.
fn collect_notices(
    layout: &Resolved,
    providers: &Providers<'_>,
    pool: &mut Pool<'_>,
) -> Vec<Notice> {
    let mut notices = std::mem::take(&mut pool.notices);
    notices.extend(
        layout
            .warnings
            .iter()
            .chain(&layout.conflicts)
            .map(|p| Notice::new(coded(p))),
    );
    for (i, p) in layout.problems.iter().enumerate() {
        match (&p.elem, pool.used[i]) {
            (None, _) => notices.push(Notice::new(coded(p))),
            (Some(e), false) => {
                notices.push(Notice::new(
                    BindCode::ElemOrphan
                        .coded()
                        .with("elem", e.clone())
                        .with("cause", p.code.clone()),
                ));
            }
            _ => {}
        }
    }
    if let SettingsHealth::Defaults { cause } = &providers.settings.health {
        notices.push(Notice::new(
            BindCode::SettingsDefaults
                .coded()
                .with("setting", cause.clone()),
        ));
    }
    notices
}

fn build_sources(layout: &Resolved, providers: &Providers<'_>) -> Vec<SourcePlan> {
    let search = providers.settings.search.unwrap_or_default();
    let mut out = Vec::new();
    for (i, s) in layout.sources.iter().enumerate() {
        let idx = u32::try_from(i).unwrap_or(u32::MAX);
        let lock = |c: Coded| PartLock::locked(Part::Source(idx), c);
        let missing = |have: &str| {
            let range = format!("{}@{}", s.capability, s.range.text());
            Coded::new("accepts-error-contract-missing")
                .with("capability", s.capability.clone())
                .with("range", range)
                .with("have", have)
        };
        let status = match providers.records.offer(&s.capability) {
            Offer::NotInstalled => Err(lock(missing(""))),
            Offer::Unreachable { cause } => Err(lock(cause)),
            Offer::Reached(list) => {
                let best = list
                    .iter()
                    .map(|p| p.version)
                    .filter(|v| s.range.contains(*v))
                    .max();
                if let Some(version) = best {
                    Ok(SourceReady {
                        capability: s.capability.clone(),
                        version,
                        search,
                    })
                } else {
                    let have = list
                        .iter()
                        .map(|p| p.version)
                        .max()
                        .map_or(String::new(), |v| format!("{}.{}.{}", v.0, v.1, v.2));
                    Err(lock(missing(&have)))
                }
            }
        };
        out.push(SourcePlan {
            idx,
            elem: s.elem.clone(),
            range: s.range,
            data_type: s.data_type.clone(),
            status,
        });
    }
    out
}

fn lock_sources_with_problems(sources: &mut [SourcePlan], pool: &mut Pool<'_>) {
    // A source holds ONE lock; every further problem of it (and all of them when the source is locked already) is a notice:
    // nothing is dropped (rule "nothing silent").
    for s in sources.iter_mut() {
        let mut own = pool.take(&s.elem).into_iter();
        if s.status.is_ok()
            && let Some(c) = own.next()
        {
            s.status = Err(PartLock::locked(Part::Source(s.idx), c));
        }
        pool.notices.extend(own.map(Notice::new));
    }
}

fn state_of(plan: &Plan) -> PlanState {
    let drawable = plan.pages.iter().any(|p| {
        p.leaves()
            .iter()
            .any(|pl| matches!(pl.status, PlaceStatus::Ready(_)))
    });
    if !drawable {
        PlanState::Refused
    } else if plan.locks().is_empty() && plan.notices.is_empty() {
        PlanState::Ready
    } else {
        PlanState::Partial
    }
}
