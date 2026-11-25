//! The port of a fixed data set and what it answers.
//!
//! Pattern: Object Mother – a named, fixed data set the builder places next to the generated ones. Departure: the mother of a module
//! is an adapter on the mother of the contract (it holds no data of its own).

use std::collections::BTreeMap;

use fs_forms_bind::ModuleForm;
use fs_interfaces::forms::Form;
use fs_objects::StoredObject;
use fs_types::Zone;

use crate::content::ContentCtx;
use crate::error::Failure;
use crate::spec::Span;
use crate::value::Moment;

/// The display names of what the records point at (calendars, people): IRI to name (AS2 `name`). The projection of the set puts the name
/// where a text channel holds an IRI; a missing name is the error `name-missing`, the IRI is never shown instead.
pub type Names = BTreeMap<String, String>;

/// The records of one form a fixed source gives for a window.
#[derive(Clone, Debug)]
pub struct FixedSlice {
    /// The records as AS2 objects with standard content, in the order of the set (first time, then key).
    pub objects: Vec<StoredObject>,
    /// What a module declares for them: the form, the type and the map channel -> field.
    pub declared: ModuleForm,
    /// The channels that belong together as the content needs them (zone, label, order); the projection reads the objects with it.
    pub ctx: ContentCtx,
    /// The display names of everything the records point at.
    pub names: Names,
}

/// Pattern: Object Mother – a named, fixed data set; the builder asks it for the records of a form inside a window.
pub trait FixedSource {
    /// The name `[[sample.set]] fixed = "<name>"` asks for, e.g. `events.sample`.
    fn name(&self) -> &str;

    /// The AS2 type of the records (`Event`, `Task`, `Person`, `Group`).
    fn data_type(&self) -> &str;

    /// The fixed "now" of the data as wall-clock time in `zone`; a view of these data never reads the clock.
    ///
    /// # Errors
    /// `zone-unknown` when the zone is not in the zone database.
    fn now(&self, zone: &Zone) -> Result<Moment, Failure>;

    /// The records of `form` inside `window` (days in `zone`), as AS2 objects with standard content, the map channel -> field,
    /// and the display names of everything the records point at. No random numbers and no clock: the same call gives the same answer.
    ///
    /// # Errors
    /// `unsupported` when the form has no channels this source can fill; `zone-unknown`.
    fn fill(&self, form: &Form, window: &Span, zone: &Zone) -> Result<FixedSlice, Failure>;
}
