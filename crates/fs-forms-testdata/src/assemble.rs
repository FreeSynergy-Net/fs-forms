//! The steps after the plan: pools of different values, emptied channels, the order of records and the AS2 object of a record.
//!
//! Pattern: Builder steps (pure functions used by `SampleBuilder`).

use fs_interfaces::forms::{Art, Form};
use fs_objects::{Envelope, ObjectId, StoredObject};
use rand::RngExt;
use rand::rngs::ChaCha8Rng;

use crate::content::{self, ContentCtx};
use crate::error::{Failure, SampleCode, fail};
use crate::makers;
use crate::spec::{SetWish, Span};
use crate::value::{Moment, Record, Value};

/// The different values of a channel (`distinct`).
pub type Pool = (String, Vec<String>);

/// FNV-1a: a hash that is the same on every platform and release (unlike `DefaultHasher`).
pub fn fnv(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

pub fn has(form: &Form, name: &str, art: Art) -> bool {
    form.channel(name)
        .is_some_and(|c| c.arts.first() == Some(&art))
}

/// The pools of `distinct`: n different words per channel.
pub fn pools(wish: &SetWish, form: &Form, rng: &mut ChaCha8Rng) -> Result<Vec<Pool>, Failure> {
    let mut out = Vec::new();
    for (channel, n) in &wish.distinct {
        if !has(form, channel, Art::Text) || *n == 0 {
            return Err(fail(
                SampleCode::CaseImpossible,
                "what",
                format!("distinct needs a text channel: {channel}"),
            ));
        }
        let mut pool: Vec<String> = Vec::new();
        for _ in 0..(*n * 100) {
            if pool.len() == *n as usize {
                break;
            }
            let word = makers::word(rng);
            if !pool.contains(&word) {
                pool.push(word);
            }
        }
        if pool.len() != *n as usize {
            return Err(fail(
                SampleCode::CaseImpossible,
                "what",
                format!("no {n} different words for {channel}"),
            ));
        }
        out.push((channel.clone(), pool));
    }
    Ok(out)
}

/// Takes the value of an optional channel away from `n` records (not the broken ones).
pub fn empty_channels(
    wish: &SetWish,
    form: &Form,
    records: &mut [(usize, Record)],
    rng: &mut ChaCha8Rng,
) -> Result<(), Failure> {
    for (channel, n) in &wish.empty {
        let optional = form.channel(channel).is_some_and(|c| !c.required);
        let mut present: Vec<usize> = (0..records.len())
            .filter(|&i| records[i].1.contains_key(channel))
            .collect();
        let broken = |i: usize| records[i].1.values().any(|v| matches!(v, Value::Raw(_)));
        present.retain(|&i| !broken(i));
        if !optional || (*n as usize) > present.len() {
            return Err(fail(
                SampleCode::CaseImpossible,
                "what",
                format!("cannot empty {channel} in {n} records"),
            ));
        }
        for _ in 0..*n {
            let pick = present.swap_remove(rng.random_range(0..present.len()));
            records[pick].1.remove(channel);
        }
    }
    Ok(())
}

pub fn first_time(ctx: &ContentCtx, record: &Record) -> Option<chrono::NaiveDateTime> {
    let from = ctx.order.as_ref().map(|o| o.from.as_str());
    let time = |v: &Value| {
        if let Value::Time(m) = v {
            Some(Moment::wall(*m))
        } else {
            None
        }
    };
    from.and_then(|f| record.get(f))
        .and_then(time)
        .or_else(|| record.values().find_map(time))
}

/// The AS2 object of one record: deterministic id, dates of the anchor, `inReplyTo` for a parent.
pub fn stored(
    content: &dyn content::Content,
    ctx: &ContentCtx,
    (form_key, data_type): (&str, &str),
    span: Span,
    parent: Option<&str>,
    (n, id): (usize, &str),
    record: &Record,
) -> Result<StoredObject, Failure> {
    let day0 = span
        .first
        .and_hms_opt(0, 0, 0)
        .unwrap_or_default()
        .and_utc();
    let mut bytes = [0u8; 10];
    bytes[..8].copy_from_slice(&fnv(form_key).to_be_bytes());
    bytes[8..].copy_from_slice(&u16::try_from(n).unwrap_or(u16::MAX).to_be_bytes());
    let millis = u64::try_from(day0.timestamp_millis()).unwrap_or(0) + n as u64;
    let uuid = uuid::Builder::from_unix_timestamp_millis(millis, &bytes).into_uuid();
    let object_id = ObjectId::from_uuid(uuid)
        .map_err(|_| fail(SampleCode::Unsupported, "what", "object id"))?;
    let parts = content.store(ctx, &format!("urn:uuid:{object_id}"), record)?;
    let mut envelope = Envelope::new(data_type, parts.name.clone());
    envelope.id = object_id;
    envelope.iri = Some(id.to_owned());
    envelope.published = day0;
    envelope.updated = day0;
    envelope.in_reply_to = parent.and_then(|p| match record.get(p) {
        Some(Value::Reference(r)) => Some(r.clone()),
        _ => None,
    });
    if let Some((start, end)) = parts.instants {
        envelope.start_time = Some(start);
        envelope.end_time = end;
    }
    Ok(StoredObject::new(
        envelope,
        content.content_type(),
        parts.json,
    ))
}
