//! `Bind::build`: layout + catalog + providers -> `Plan`. Pure, synchronous, no I/O, no clock, no panic, no `Result`.
//!
//! Pattern: Builder. Every problem lands as a `PartLock` on the smallest part it concerns, or as a window `Notice`.

use crate::build_menu::build_menu;
use crate::build_place::{Env, build_place};
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

    /// The problems of one element. `provider-unchecked` is only a window notice (bind checks the lists itself).
    pub(crate) fn take(&mut self, elem: &str) -> Vec<Coded> {
        let mut out = Vec::new();
        for (i, p) in self.items.iter().enumerate() {
            if self.used[i] || p.elem.as_deref() != Some(elem) {
                continue;
            }
            self.used[i] = true;
            let c = coded(p);
            if p.code == "layout-error-provider-unchecked" {
                self.notices.push(Notice::new(c));
            } else {
                out.push(c);
            }
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
            let mut order: Vec<_> = page.places.iter().collect();
            order.sort_by_key(|p| p.seq);
            let env = Env {
                catalog,
                host: env_host,
                sources: &sources,
            };
            let places = order
                .iter()
                .map(|p| {
                    build_place(
                        &env,
                        page,
                        p,
                        page.places.iter().filter(|q| q.seq == p.seq).count() > 1,
                        &mut pool,
                    )
                })
                .collect();
            pages.push(PagePlan {
                page: page.page.clone(),
                elem: page.elem.clone(),
                seq: page.seq,
                frame: page.frame.clone(),
                locks,
                places,
            });
        }
        let menu = build_menu(&layout.menu, &sources, env_host, &mut pool);
        let mut notices = std::mem::take(&mut pool.notices);
        notices.extend(layout.conflicts.iter().map(|p| Notice::new(coded(p))));
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

fn build_sources(layout: &Resolved, providers: &Providers<'_>) -> Vec<SourcePlan> {
    let search = providers.settings.search.unwrap_or_default();
    let mut out = Vec::new();
    for (i, s) in layout.sources.iter().enumerate() {
        let idx = u32::try_from(i).unwrap_or(u32::MAX);
        let lock = |c: Coded| PartLock::locked(Part::Source(idx), c);
        let missing = |have: &str| {
            let range = format!("{} {}", s.capability, s.range.text());
            Coded::new("accepts-error-contract-missing")
                .with("capability", s.capability.clone())
                .with("range", range)
                .with("have", have)
        };
        let status = match providers.records.offer(&s.capability) {
            Offer::NotInstalled => Err(lock(missing(""))),
            Offer::Unreachable { cause } => Err(lock(cause)),
            Offer::Reached(list) => match list
                .iter()
                .map(|p| p.version)
                .filter(|v| s.range.contains(*v))
                .max()
            {
                Some(version) => Ok(SourceReady {
                    capability: s.capability.clone(),
                    version,
                    search,
                }),
                None => {
                    let have = list
                        .iter()
                        .map(|p| p.version)
                        .max()
                        .map_or(String::new(), |v| format!("{}.{}", v.0, v.1));
                    Err(lock(missing(&have)))
                }
            },
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
    for s in sources.iter_mut() {
        let own = pool.take(&s.elem);
        if s.status.is_ok()
            && let Some(c) = own.into_iter().next()
        {
            s.status = Err(PartLock::locked(Part::Source(s.idx), c));
        }
    }
}

fn state_of(plan: &Plan) -> PlanState {
    let drawable = plan.pages.iter().any(|p| {
        p.places
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
