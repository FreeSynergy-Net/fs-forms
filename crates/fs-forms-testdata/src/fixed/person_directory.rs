//! `person-directory.sample`: the fixed sample of the contract `person_directory_v1` (`fs_person_directory::sample()`) as `Profile` records (the type of the module `person`).
//!
//! Pattern: Object Mother behind a Registry (the Adapter holds no data: it asks the directory `sample()` who is a person, and takes the
//! contact fields from `sample_card`/`sample_address`, so a person with two e-mails and two phones is ONE record, as the rule "one person"
//! says). Departures: only entries that are a confirmed `Person` and resolve to themselves are records (a profile, a group, a deleted,
//! a hidden or a merged entry is not a person of its own); the people are in the order of the directory's sort key; the data have no
//! time of their own, so there is no fixed "now".

use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

use fs_forms_bind::ModuleForm;
use fs_interfaces::forms::Form;
use fs_objects::StoredObject;
use fs_person_directory::{
    EntryState, PersonArt, PersonDirectory, PersonView, Resolved, sample, sample_address,
    sample_card, sample_name, sample_ref,
};
use fs_types::{BasicValue, Zone};

use super::person_card::card;
use super::{FixedSlice, FixedSource, Names};
use crate::assemble::{label_of, module_form, object_id, wrapped};
use crate::content::{self, ContentCtx, Parts};
use crate::error::{Failure, SampleCode, fail};
use crate::spec::Span;
use crate::value::Moment;

/// The name of this source.
const NAME: &str = "person-directory.sample";
/// The numbers of the entries of the sample (`sample()` has twelve).
const ENTRIES: std::ops::RangeInclusive<u8> = 1..=12;

/// The adapter on the fixed sample of the contract `person_directory_v1`.
#[derive(Clone, Copy, Debug, Default)]
pub struct PersonDirectorySample;

fn unsupported(what: impl Into<String>) -> Failure {
    fail(SampleCode::Unsupported, "what", what)
}

/// The answer of a future that is ready at once (the in-memory directory never waits); anything else is a failure, never a hang.
fn ready<T>(future: impl Future<Output = T>) -> Result<T, Failure> {
    match pin!(future).poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(v) => Ok(v),
        Poll::Pending => Err(unsupported("the sample directory did not answer at once")),
    }
}

/// The people of the sample: `(number, view)` of every entry that is a confirmed `Person` and resolves to itself, in the order of the
/// sort key of the directory.
fn people() -> Result<Vec<(u8, PersonView)>, Failure> {
    let directory = sample();
    let mut out = Vec::new();
    for n in ENTRIES {
        let Some(asked) = sample_ref(n) else { continue };
        let Resolved::Found(view) = ready(directory.resolve(&asked))?
            .map_err(|e| unsupported(format!("entry {n}: {e:?}")))?
        else {
            continue;
        };
        let own = view.person.id == asked.id;
        if own && view.person.art == PersonArt::Person && view.state == EntryState::Confirmed {
            out.push((n, view));
        }
    }
    out.sort_by(|a, b| (&a.1.sort_key, a.0).cmp(&(&b.1.sort_key, b.0)));
    Ok(out)
}

/// IRI of entry `n` (the key of the record, the `uid` of the card).
fn iri(view: &PersonView) -> String {
    view.person.id.canonical()
}

/// The display names of everything the records may point at: every entry of the directory by its IRI.
fn names() -> Names {
    ENTRIES
        .filter_map(|n| Some((sample_ref(n)?.id.canonical(), sample_name(n)?.to_owned())))
        .collect()
}

fn objects(form: &Form, window: Span) -> Result<Vec<StoredObject>, Failure> {
    let content = content::for_type("Profile");
    let key = form.key();
    let mut out = Vec::new();
    for (i, (n, view)) in people()?.iter().enumerate() {
        let uid = iri(view);
        let values = sample_card(*n);
        let fallback = sample_address(*n);
        let json = card(&uid, &view.display, &values, fallback, view.avatar.as_ref());
        let parts = Parts {
            name: view.display.clone(),
            json,
            instants: None,
        };
        let id = object_id(&key, window, i)?;
        out.push(wrapped(
            (id, content.content_type()),
            parts,
            ("Profile", window),
            (&uid, None),
        ));
    }
    Ok(out)
}

impl FixedSource for PersonDirectorySample {
    fn name(&self) -> &str {
        NAME
    }

    #[allow(clippy::unnecessary_literal_bound)] // the port says `&str`: another source may build its type at run time
    fn data_type(&self) -> &str {
        "Profile"
    }

    fn now(&self, _zone: &Zone) -> Result<Option<Moment>, Failure> {
        Ok(None)
    }

    fn fill(&self, form: &Form, window: &Span, zone: &Zone) -> Result<FixedSlice, Failure> {
        let ctx = ContentCtx {
            zone: zone.id().to_owned(),
            label: label_of(form).map(|c| c.name.clone()),
            order: None,
        };
        let declared: ModuleForm =
            module_form(form, &*content::for_type("Profile"), &ctx, "Profile");
        Ok(FixedSlice {
            objects: objects(form, *window)?,
            declared,
            ctx,
            names: names(),
        })
    }
}
