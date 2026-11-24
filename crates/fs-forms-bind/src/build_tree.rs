//! Step of `Bind::build` for the tree of places of one page: every level in `seq` order, a frame keeps its children.
//!
//! Pattern: Composite walk (recursion over the tree; the leaves go to `build_place`).

use crate::build::Pool;
use crate::build_place::{Env, build_place};
use crate::codes::{Part, PartLock};
use crate::model::PageIn;
use crate::model_tree::{AttrIn, NodeIn};
use crate::plan_tree::{AttrPlan, FramePlan, NodePlan};

pub(crate) fn attrs(set: &[AttrIn]) -> Vec<AttrPlan> {
    set.iter()
        .map(|a| AttrPlan {
            name: a.name.clone(),
            value: a.value.clone(),
        })
        .collect()
}

pub(crate) fn build_nodes(
    env: &Env<'_>,
    page: &PageIn,
    nodes: &[NodeIn],
    pool: &mut Pool<'_>,
) -> Vec<NodePlan> {
    let mut order: Vec<&NodeIn> = nodes.iter().collect();
    order.sort_by_key(|n| n.seq());
    order
        .into_iter()
        .map(|n| match n {
            NodeIn::Block(p) => {
                let twice = nodes.iter().filter(|q| q.seq() == p.seq).count() > 1;
                NodePlan::Block(build_place(env, page, p, twice, pool))
            }
            NodeIn::Frame(f) => {
                let part = Part::Place {
                    page: page.page.clone(),
                    place: f.place.clone(),
                };
                let locks = pool
                    .take(&f.elem)
                    .into_iter()
                    .map(|c| PartLock::locked(part.clone(), c))
                    .collect();
                NodePlan::Frame(FramePlan {
                    place: f.place.clone(),
                    elem: f.elem.clone(),
                    seq: f.seq,
                    frame: f.frame.clone(),
                    set: attrs(&f.set),
                    locks,
                    children: build_nodes(env, page, &f.children, pool),
                })
            }
        })
        .collect()
}
