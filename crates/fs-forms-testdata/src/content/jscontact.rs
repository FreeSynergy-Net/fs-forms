//! JSContact (RFC 9553) content for the AS2 type `Person`: one `Card` per person, as the standard has it (`name.full`, `emails`,
//! `phones`, `addresses`, `media`). A card with several e-mails, phones or addresses is one object; the way back to the channels
//! gives every item its own value on the path of its field (the form marks that channel `repeats`).
//!
//! Pattern: Strategy (`Content`, the standard of `Person`) + Adapter (card properties <-> field paths of the module).

use fs_forms_bind::Basic;
use fs_interfaces::forms::{Art, Form};
use fs_jscalendar::model::VENDOR;
use serde_json::{Map, Value as Json, json};

use super::text::{scalar_text, value_json};
use super::{Content, ContentCtx, FieldMap, Parts};
use crate::error::Failure;
use crate::value::{Record, Value};

/// The media type of a card (RFC 9553 section 7).
const MEDIA_TYPE: &str = "application/jscontact+json";

/// A person as a JSContact `Card`.
pub struct JsContact;

/// The names of the fields (the module's own paths: the form maps its channels onto them).
const NAME: &str = "name";
const PHOTO: &str = "photo";
/// The fields that carry values: path, JSContact member, basic type.
const FIELDS: [(&str, Basic); 6] = [
    ("emails", Basic::Email),
    ("phones", Basic::Text),
    ("address", Basic::Text),
    ("surname", Basic::Text),
    ("organizations", Basic::Text),
    ("titles", Basic::Text),
];

/// The items of a map member (`emails`: id -> `{address}`) as text, in the order of their ids; `key` names the text inside an item.
fn items(card: &Json, member: &str, key: &str) -> Vec<String> {
    card.get(member)
        .and_then(Json::as_object)
        .map(|m| {
            m.values()
                .filter_map(|v| v.get(key).and_then(Json::as_str).map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// The `value` of the component of `kind` in a name (`surname`).
fn component(card: &Json, kind: &str) -> Option<String> {
    card.pointer("/name/components")?
        .as_array()?
        .iter()
        .find(|c| c.get("kind").and_then(Json::as_str) == Some(kind))
        .and_then(|c| c.get("value")?.as_str().map(str::to_owned))
}

/// The photo of a card: the `uri` of its first medium of kind `photo`.
fn photo(card: &Json) -> Option<String> {
    card.get("media")?
        .as_object()?
        .values()
        .find(|m| m.get("kind").and_then(Json::as_str) == Some("photo"))
        .and_then(|m| m.get("uri")?.as_str().map(str::to_owned))
}

impl Content for JsContact {
    fn content_type(&self) -> &'static str {
        MEDIA_TYPE
    }

    fn store(&self, ctx: &ContentCtx, uid: &str, record: &Record) -> Result<Parts, Failure> {
        let name = match ctx.label.as_ref().and_then(|l| record.get(l)) {
            Some(Value::Text(t)) if !t.trim().is_empty() => t.clone(),
            _ => "(untitled)".to_owned(),
        };
        let mut card = Map::new();
        card.insert("@type".to_owned(), "Card".into());
        card.insert("version".to_owned(), "1.0".into());
        card.insert("uid".to_owned(), uid.into());
        card.insert("kind".to_owned(), "individual".into());
        card.insert("name".to_owned(), json!({ "full": name }));
        for (channel, value) in record.iter().filter(|(c, _)| ctx.label.as_ref() != Some(c)) {
            card.insert(format!("{VENDOR}{channel}"), value_json(value, &ctx.zone)?);
        }
        Ok(Parts {
            name,
            json: card.into(),
            instants: None,
        })
    }

    fn read(&self, _ctx: &ContentCtx, json: &Json) -> Vec<(String, Vec<String>)> {
        let mut out: Vec<(String, Vec<String>)> = Vec::new();
        let mut add = |path: &str, values: Vec<String>| {
            if !values.is_empty() {
                out.push((path.to_owned(), values));
            }
        };
        add(NAME, items_of(json.get("name"), "full"));
        add(PHOTO, photo(json).into_iter().collect());
        add("surname", component(json, "surname").into_iter().collect());
        add("emails", items(json, "emails", "address"));
        add("phones", items(json, "phones", "number"));
        add("address", items(json, "addresses", "full"));
        add("organizations", items(json, "organizations", "name"));
        add("titles", items(json, "titles", "name"));
        if let Some(object) = json.as_object() {
            for (key, value) in object {
                if let Some(channel) = key.strip_prefix(VENDOR) {
                    add(channel, vec![scalar_text(value)]);
                }
            }
        }
        out
    }

    fn map(&self, ctx: &ContentCtx, form: &Form) -> FieldMap {
        let mut map: Vec<(String, Vec<String>)> = Vec::new();
        let mut fields: Vec<(String, Basic)> = Vec::new();
        for channel in &form.channels {
            let Some(art) = channel.arts.first().copied() else {
                continue;
            };
            let name = channel.name.as_str();
            // The channel that names a record is the full name; a channel that takes many fields takes the contact fields of the card.
            if ctx.label.as_deref() == Some(name) {
                map.push((name.to_owned(), vec![NAME.to_owned()]));
                fields.push((NAME.to_owned(), Basic::Text));
            } else if art == Art::Reference && name == "preview" {
                map.push((name.to_owned(), vec![PHOTO.to_owned()]));
                fields.push((PHOTO.to_owned(), Basic::Image));
            } else if channel.count.max.is_none_or(|m| m > 1) && channel.arts.contains(&Art::Text) {
                // As many fields as the channel takes (`facts` 0..4, `fields` 0..n), in the order of FIELDS.
                let take = channel
                    .count
                    .max
                    .map_or(FIELDS.len(), |m| FIELDS.len().min(m as usize));
                map.push((
                    name.to_owned(),
                    FIELDS
                        .iter()
                        .take(take)
                        .map(|(p, _)| (*p).to_owned())
                        .collect(),
                ));
                for (path, basic) in FIELDS.into_iter().take(take) {
                    if !fields.iter().any(|(p, _)| p == path) {
                        fields.push((path.to_owned(), basic));
                    }
                }
            }
        }
        (map, fields)
    }
}

/// `key` of an object member as a one-item list.
fn items_of(member: Option<&Json>, key: &str) -> Vec<String> {
    member
        .and_then(|m| m.get(key)?.as_str())
        .map(|s| vec![s.to_owned()])
        .unwrap_or_default()
}
