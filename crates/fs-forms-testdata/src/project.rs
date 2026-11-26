//! Object -> row: what `SampleModule` sends in `page`, and the table it keeps in memory. A value carries the FIELD the module declares for its channel.
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
    /// What the module declares for the form: channel to field names (`ModuleForm::map`). A value is shown, and a write is named, by FIELD.
    pub map: Vec<(String, Vec<String>)>,
}

impl Table {
    /// The path of a value of `channel`: the first field the map declares for it (only the first is shown; every declared field may be
    /// written). A channel without an entry keeps its name: `SampleModule::declare` can give a form of a newer minor than the set
    /// declared, whose channels the map does not know; the host reads only by the map, so it never sees such a value.
    #[must_use]
    pub fn field_of<'a>(&'a self, channel: &'a str) -> &'a str {
        self.map
            .iter()
            .find(|(c, _)| c == channel)
            .and_then(|(_, fields)| fields.first())
            .map_or(channel, String::as_str)
    }

    /// The channel a written path names: the channel whose declared fields hold `path`; a channel without an entry is named by itself.
    #[must_use]
    pub fn channel_of(&self, path: &str) -> Option<&str> {
        self.map
            .iter()
            .find(|(_, fields)| fields.iter().any(|f| f == path))
            .map(|(c, _)| c.as_str())
            .or_else(|| {
                let named = self.form.channel(path)?;
                (!self.map.iter().any(|(c, _)| *c == named.name)).then_some(named.name.as_str())
            })
    }
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
            let map = set
                .declared
                .iter()
                .find(|d| same_form(d, &slice.form))
                .map_or_else(Vec::new, |d| d.map.clone());
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
                map,
            })
        })
        .collect()
}

/// A record as the contract shows it; `may` are the commands offered on it.
#[must_use]
pub fn view(rec: &Rec, table: &Table, may: Vec<String>) -> RecordView {
    let mut out = RecordView::default();
    out.id.clone_from(&rec.id);
    out.may = may;
    out.values = rec
        .values
        .iter()
        .flat_map(|(channel, texts)| texts.iter().map(|t| Value::new(table.field_of(channel), t)))
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

/// Whether the key `key` (`name@x.y.z`; the patch does not count) is the form `name` of version `major.minor` (one rule for the table of a
/// key and the declaration of a form).
#[must_use]
pub fn is_form(key: &str, name: &str, major: u64, minor: u64) -> bool {
    fs_interfaces::forms::parse_key(key)
        .is_some_and(|(n, v)| n == name && v.major == major && v.minor == minor)
}

/// Whether the declaration `d` is for the form `key`.
fn same_form(d: &fs_forms_bind::ModuleForm, key: &str) -> bool {
    is_form(
        key,
        &d.form.name,
        u64::from(d.form.major),
        u64::from(d.form.minor),
    )
}
