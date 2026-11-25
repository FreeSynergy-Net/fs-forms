//! Object -> row: what `SampleModule` sends in `page`, and the table it keeps in memory.
//!
//! Pattern: Projection (the stored AS2 object, read the way a real module must: RFC 3339 with the offset of the zone of the
//! anchor, whole days as `full-date`; the reading is `SampleSet::rows`) + Adapter (row -> `RecordView` of the contract).

use fs_interfaces::forms::{Art, Form};
use fs_interfaces::module::{RecordView, Value};

use crate::error::Failure;
use crate::set::SampleSet;

/// The values of a record as text by channel (a channel left empty is missing).
pub type Values = Vec<(String, Vec<String>)>;

/// One record in memory. `alive` is false after the creation was taken back; the record stays so its history stays.
#[derive(Clone, Debug)]
pub struct Rec {
    /// The key (`urn:fs-sample:<form>:<n>`).
    pub id: String,
    /// The revision; seeds start at 1, every change adds one.
    pub rev: u64,
    /// The values.
    pub values: Values,
    /// Whether the record exists.
    pub alive: bool,
}

/// The records of one form with the declaration the module keeps for it.
#[derive(Clone, Debug)]
pub struct Table {
    /// The key `name@x.y.z`.
    pub key: String,
    /// The declaration the commands run on (the catalog's, or the one given with `SampleModule::declare`).
    pub form: Form,
    /// The records, in the order of the set.
    pub recs: Vec<Rec>,
}

/// The tables of a set: one per form, the rows read from the stored objects.
///
/// # Errors
/// `form-unknown` when the catalog has no form of the key, or the key is not in the set.
pub fn tables(set: &SampleSet) -> Result<Vec<Table>, Failure> {
    set.slices
        .iter()
        .map(|slice| {
            let form = fs_interfaces::forms::find_key(&slice.form).ok_or_else(|| {
                crate::error::fail(crate::SampleCode::FormUnknown, "form", &slice.form)
            })?;
            let recs = set
                .rows(&slice.form)?
                .into_iter()
                .map(|row| Rec {
                    id: row.key,
                    rev: 1,
                    values: row.values,
                    alive: true,
                })
                .collect();
            Ok(Table {
                key: slice.form.clone(),
                form: form.clone(),
                recs,
            })
        })
        .collect()
}

/// A record as the contract shows it; `may` are the commands offered on it.
#[must_use]
pub fn view(rec: &Rec, may: Vec<String>) -> RecordView {
    let mut out = RecordView::default();
    out.id.clone_from(&rec.id);
    out.may = may;
    out.values = rec
        .values
        .iter()
        .flat_map(|(channel, texts)| texts.iter().map(|t| Value::new(channel, t)))
        .collect();
    out
}

/// The channels that carry a value (everything but `records` and `command`), by name.
#[must_use]
pub fn data_channels(form: &Form) -> Vec<&str> {
    form.channels
        .iter()
        .filter(|c| {
            c.arts
                .first()
                .is_some_and(|a| !matches!(a, Art::Records | Art::Command))
        })
        .map(|c| c.name.as_str())
        .collect()
}
