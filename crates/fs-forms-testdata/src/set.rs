//! The result: AS2 objects, what the module declares for them, and the way back to rows of text.
//!
//! Pattern: Value object (`SampleSet`) + Projection (`rows`: object -> the row a module sends in `page`).

use std::fmt::Write as _;

use fs_forms_bind::ModuleForm;
use fs_objects::StoredObject;

use crate::content::{self, ContentCtx};
use crate::error::{Failure, SampleCode, fail};
use crate::spec::Forbid;

/// The records of one form in `SampleSet::objects`: a slice, in the order of the set.
#[derive(Clone, Debug)]
pub struct Slice {
    /// The key `name@x.y.z` of the form.
    pub form: String,
    /// The AS2 type of the records.
    pub data_type: String,
    /// Index of the first object.
    pub first: usize,
    /// Number of objects.
    pub len: usize,
    pub(crate) ctx: ContentCtx,
}

/// One row as a module sends it: the key of the record and the text of its channels (a channel left empty is missing).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SampleRow {
    /// The key of the record (`urn:fs-sample:<form>:<n>`).
    pub key: String,
    /// The values by channel name.
    pub values: Vec<(String, Vec<String>)>,
}

impl SampleRow {
    /// The first value of `channel`.
    #[must_use]
    pub fn get(&self, channel: &str) -> Option<&str> {
        let (_, values) = self.values.iter().find(|(c, _)| c == channel)?;
        values.first().map(String::as_str)
    }
}

/// What the builder made. Order and ids are stable: the same spec gives the same set.
#[derive(Clone, Debug)]
pub struct SampleSet {
    /// The objects, form after form, each form sorted by its first time and then by id.
    pub objects: Vec<StoredObject>,
    /// What a module declares for each form: the form, the type and the map channel -> field.
    pub declared: Vec<ModuleForm>,
    /// Which objects belong to which form.
    pub slices: Vec<Slice>,
    /// What `SampleModule` must refuse (from the spec).
    pub forbid: Vec<Forbid>,
}

impl SampleSet {
    /// An empty set that carries the wishes of the rights.
    pub(crate) fn empty(forbid: Vec<Forbid>) -> Self {
        Self {
            objects: Vec::new(),
            declared: Vec::new(),
            slices: Vec::new(),
            forbid,
        }
    }

    /// The rows of one form (`name@x.y.z`), read from the stored objects the way a module would send them.
    ///
    /// # Errors
    /// `form-unknown` when the set has no such form.
    pub fn rows(&self, form: &str) -> Result<Vec<SampleRow>, Failure> {
        let slice = self
            .slices
            .iter()
            .find(|s| s.form == form)
            .ok_or_else(|| fail(SampleCode::FormUnknown, "form", form))?;
        let reader = content::for_type(&slice.data_type);
        Ok(self.objects[slice.first..slice.first + slice.len]
            .iter()
            .map(|o| SampleRow {
                key: o.envelope.iri.clone().unwrap_or_default(),
                values: reader.read(&slice.ctx, &o.content),
            })
            .collect())
    }

    /// The set as text: one header per form, then one line per object (JSON with sorted keys). The golden test compares it.
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = String::new();
        for slice in &self.slices {
            let _ = writeln!(out, "# {} {} {}", slice.form, slice.data_type, slice.len);
            for object in &self.objects[slice.first..slice.first + slice.len] {
                let line = serde_json::to_value(object)
                    .map(|v| v.to_string())
                    .unwrap_or_default();
                out.push_str(&line);
                out.push('\n');
            }
        }
        out
    }
}
