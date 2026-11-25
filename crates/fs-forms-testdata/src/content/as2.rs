//! Plain ActivityStreams 2 content: the standard for every type that has no richer one.
//!
//! Pattern: Strategy (`Content`): the values are properties of the object named like their channel.

use fs_interfaces::forms::Form;

use super::text::{scalar_text, value_json};
use super::{Content, ContentCtx, FieldMap, Parts, basic_of};
use crate::error::Failure;
use crate::value::{Record, Value};

/// Channel values as AS2 properties; times as RFC 3339 text.
pub struct Plain;

impl Content for Plain {
    fn content_type(&self) -> &'static str {
        "application/activity+json"
    }

    fn store(&self, ctx: &ContentCtx, _uid: &str, record: &Record) -> Result<Parts, Failure> {
        let mut object = serde_json::Map::new();
        for (channel, value) in record {
            object.insert(channel.clone(), value_json(value, &ctx.zone)?);
        }
        let name = match ctx.label.as_ref().and_then(|l| record.get(l)) {
            Some(Value::Text(t)) if !t.trim().is_empty() => t.clone(),
            _ => "(untitled)".to_owned(),
        };
        Ok(Parts {
            name,
            json: object.into(),
            instants: None,
        })
    }

    fn read(&self, _ctx: &ContentCtx, json: &serde_json::Value) -> Vec<(String, Vec<String>)> {
        json.as_object()
            .map(|o| {
                o.iter()
                    .map(|(k, v)| (k.clone(), vec![scalar_text(v)]))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn map(&self, _ctx: &ContentCtx, form: &Form) -> FieldMap {
        let valued = form
            .channels
            .iter()
            .filter_map(|c| Some((c, *c.arts.first()?)));
        let valued: Vec<_> = valued
            .filter(|(_, a)| {
                !matches!(
                    a,
                    fs_interfaces::forms::Art::Records | fs_interfaces::forms::Art::Command
                )
            })
            .collect();
        let map = valued
            .iter()
            .map(|(c, _)| (c.name.clone(), vec![c.name.clone()]))
            .collect();
        let fields = valued
            .iter()
            .map(|(c, a)| (c.name.clone(), basic_of(*a)))
            .collect();
        (map, fields)
    }
}
