//! The builder: wishes + declaration of the form + relations + strategies -> a `SampleSet`.
//!
//! Pattern: Test Data Builder – pure and deterministic (no clock, no I/O); the values come from the strategies per `Art`.

use fs_forms_bind::{FormRef, ModuleForm};
use fs_interfaces::forms::{self, Art, Form};
use rand::SeedableRng;
use rand::rngs::ChaCha8Rng;

use crate::assemble::{
    Pool, empty_channels, first_time, fnv, has, label_of, map_records_channels, pools, stored,
};
use crate::content::{self, ContentCtx, Order};
use crate::error::{Failure, SampleCode, fail};
use crate::interval::{self, Pair};
use crate::makers;
use crate::plan::{Plan, plan};
use crate::set::{SampleSet, Slice};
use crate::spec::{Relation, SampleSpec, SetWish, Span};
use crate::value::{MakeCx, Record, Value, ValueMaker};

/// Collects the spec, the relations and the strategies, then builds.
pub struct SampleBuilder {
    spec: SampleSpec,
    makers: Vec<Box<dyn ValueMaker>>,
    relations: Vec<Relation>,
}

/// One record on its way: its place in the set, its plan, the ids before it and the pools of the set.
struct Draft<'a> {
    n: usize,
    plan: &'a Plan,
    ids: &'a [String],
    pools: &'a [Pool],
}

/// The highest version of the form that the requirement (`name@^1.0.0`, or just `name`) allows, found with `forms::find`.
pub(crate) fn resolve(wish: &str) -> Result<&'static Form, Failure> {
    let unknown = || fail(SampleCode::FormUnknown, "form", wish);
    let (name, req) = wish.split_once('@').map_or((wish, "*"), |(n, r)| (n, r));
    let req = semver::VersionReq::parse(req).map_err(|_| unknown())?;
    let version = forms::catalog()
        .iter()
        .filter(|f| f.name == name && req.matches(&f.version))
        .map(|f| f.version.clone())
        .max()
        .ok_or_else(unknown)?;
    forms::find(name, &version).ok_or_else(unknown)
}

impl SampleBuilder {
    /// A builder with the standard strategies.
    #[must_use]
    pub fn new(spec: SampleSpec) -> Self {
        Self {
            spec,
            makers: makers::standard(),
            relations: Vec::new(),
        }
    }

    /// A relation between channels; a set uses it when its form has the channels (every relation must be used by some set).
    #[must_use]
    pub fn relation(mut self, relation: Relation) -> Self {
        self.relations.push(relation);
        self
    }

    /// Replaces the strategy of an `Art` (or adds one).
    #[must_use]
    pub fn maker(mut self, maker: Box<dyn ValueMaker>) -> Self {
        self.makers.retain(|m| m.art() != maker.art());
        self.makers.push(maker);
        self
    }

    /// Builds the set.
    ///
    /// # Errors
    /// A [`Failure`] for the first wish that cannot be met; nothing is dropped silently.
    pub fn build(self) -> Result<SampleSet, Failure> {
        let span = self.spec.anchor.period.span()?;
        let mut used = vec![false; self.relations.len()];
        let mut set = SampleSet::empty(self.spec.forbid.clone());
        for wish in &self.spec.sets {
            self.build_set(wish, span, &mut used, &mut set)?;
        }
        if let Some(at) = used.iter().position(|u| !u) {
            return Err(fail(
                SampleCode::RelationUnknown,
                "what",
                format!("{:?} fits no form of the spec", self.relations[at]),
            ));
        }
        Ok(set)
    }

    fn build_set(
        &self,
        wish: &SetWish,
        span: Span,
        used: &mut [bool],
        set: &mut SampleSet,
    ) -> Result<(), Failure> {
        let form = resolve(&wish.form)?;
        let key = form.key();
        let mut rng = ChaCha8Rng::seed_from_u64(self.spec.seed ^ fnv(&key));
        let (order, parent) = self.relations_of(form, used);
        let ctx = ContentCtx {
            zone: self.spec.anchor.zone.id().to_owned(),
            label: label_of(form).map(|c| c.name.clone()),
            order: order.clone(),
        };
        let plans = plan(
            wish,
            span,
            &mut rng,
            order.is_some(),
            order.as_ref().is_some_and(|o| o.all_day.is_some()),
            parent.is_some(),
        )?;
        let pools = pools(wish, form, &mut rng)?;
        let mut records = Vec::new();
        let mut ids: Vec<String> = Vec::new();
        for (n, one) in plans.iter().enumerate() {
            let id = format!("urn:fs-sample:{key}:{}", n + 1);
            let record = self.record(
                form,
                &ctx,
                parent.as_deref(),
                (&mut rng, span),
                &Draft {
                    n,
                    plan: one,
                    ids: &ids,
                    pools: &pools,
                },
            )?;
            ids.push(id);
            records.push((n, record));
        }
        empty_channels(wish, form, &mut records, &mut rng)?;
        let sort = |r: &(usize, Record)| (first_time(&ctx, &r.1), r.0);
        records.sort_by_key(sort);
        let content = content::for_type(&wish.data_type);
        let first = set.objects.len();
        for (n, record) in &records {
            let object = stored(
                &*content,
                &ctx,
                (&key, &wish.data_type),
                span,
                parent.as_deref(),
                (*n, &ids[*n]),
                record,
            )?;
            set.objects.push(object);
        }
        let (mut map, fields) = content.map(&ctx, form);
        map_records_channels(&mut map, form, &wish.data_type);
        set.declared.push(ModuleForm {
            form: FormRef {
                name: form.name.clone(),
                major: u32::try_from(form.version.major).unwrap_or(0),
                minor: u32::try_from(form.version.minor).unwrap_or(0),
            },
            data_type: wish.data_type.clone(),
            map,
            fields,
        });
        set.slices.push(Slice {
            form: key,
            data_type: wish.data_type.clone(),
            first,
            len: records.len(),
            ctx,
        });
        Ok(())
    }

    /// The first `Order` and the first `Parent` relation that fit the form; they are marked as used.
    fn relations_of(&self, form: &Form, used: &mut [bool]) -> (Option<Order>, Option<String>) {
        let (mut order, mut parent) = (None, None);
        for (at, relation) in self.relations.iter().enumerate() {
            match relation {
                Relation::Order { from, to, all_day }
                    if order.is_none()
                        && has(form, from, Art::Time)
                        && has(form, to, Art::Time)
                        && all_day.as_deref().is_none_or(|f| has(form, f, Art::Flag)) =>
                {
                    order = Some(Order {
                        from: from.clone(),
                        to: to.clone(),
                        all_day: all_day.clone(),
                    });
                    used[at] = true;
                }
                Relation::Parent { child }
                    if parent.is_none() && has(form, child, Art::Reference) =>
                {
                    parent = Some(child.clone());
                    used[at] = true;
                }
                _ => {}
            }
        }
        (order, parent)
    }

    fn record(
        &self,
        form: &Form,
        ctx: &ContentCtx,
        parent: Option<&str>,
        (rng, span): (&mut ChaCha8Rng, Span),
        draft: &Draft<'_>,
    ) -> Result<Record, Failure> {
        let Draft {
            n,
            plan: one,
            ids,
            pools,
        } = *draft;
        let mut record = Record::new();
        if let Some(order) = &ctx.order {
            let Pair {
                from,
                to,
                whole_days,
            } = match one.slot {
                Some((case, day, nth)) => interval::pair(case, day, nth, span),
                None => interval::ordinary(span, rng),
            };
            record.insert(order.from.clone(), Value::Time(from));
            let end = if one.invalid {
                Value::Raw("not-a-time".to_owned())
            } else {
                Value::Time(to)
            };
            record.insert(order.to.clone(), end);
            if let Some(flag) = &order.all_day {
                record.insert(flag.clone(), Value::Flag(whole_days));
            }
        } else if one.invalid {
            return Err(fail(
                SampleCode::CaseImpossible,
                "what",
                "invalid rows need an Order relation",
            ));
        }
        let skip = |name: &str| {
            ctx.order.as_ref().is_some_and(|o| {
                o.from == name || o.to == name || o.all_day.as_deref() == Some(name)
            }) || parent == Some(name)
        };
        for channel in form.channels.iter().filter(|c| !skip(&c.name)) {
            let Some(&art) = channel.arts.first() else {
                continue;
            };
            if matches!(art, Art::Records | Art::Command) {
                continue;
            }
            let pool = pools
                .iter()
                .find(|(c, _)| *c == channel.name)
                .map_or(&[][..], |(_, p)| p.as_slice());
            if !(channel.required || !pool.is_empty() || n % 2 == 1) {
                continue;
            }
            let maker = self.makers.iter().find(|m| m.art() == art).ok_or_else(|| {
                fail(
                    SampleCode::Unsupported,
                    "what",
                    format!("no strategy for the art {}", art.name()),
                )
            })?;
            let mut cx = MakeCx {
                rng: &mut *rng,
                index: u32::try_from(n).unwrap_or(u32::MAX),
                span,
                pool,
                earlier: &ids[..n.min(ids.len())],
                flavor: one.flavor,
            };
            if let Some(value) = maker.make(&mut cx, channel) {
                record.insert(channel.name.clone(), value);
            }
        }
        if let (Some(child), Some(at)) = (parent, one.parent) {
            record.insert(child.to_owned(), Value::Reference(ids[at].clone()));
        }
        Ok(record)
    }
}
