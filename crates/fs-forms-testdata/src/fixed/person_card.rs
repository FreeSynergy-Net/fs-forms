//! One person of the sample directory as a JSContact (RFC 9553) `Card`.
//!
//! Pattern: Adapter (the typed values of `person_directory_v1`, `fs-types`: `Name`, `EmailAddress`, `PhoneNumber`, `Address`, `FileRef`,
//! -> the members of the standard). Nothing is invented: a member without a value is left out (the sample has no birthday and no
//! organisation, so the card has none), and no context (work, private) is written because the directory does not say it.

use fs_person_directory::ExcerptValue;
use fs_types::{BasicValue, FileRef};
use serde_json::{Map, Value as Json, json};

/// `{"<prefix><nn>": item}` for the items in the order given (ids sort as they were given).
fn numbered(prefix: &str, items: Vec<Json>) -> Option<Json> {
    (!items.is_empty()).then(|| {
        items
            .into_iter()
            .enumerate()
            .map(|(i, item)| (format!("{prefix}{:02}", i + 1), item))
            .collect::<Map<String, Json>>()
            .into()
    })
}

/// A list of `NameComponent`/`AddressComponent` from `(kind, value)` pairs.
fn components<'a>(kind: &str, pairs: impl Iterator<Item = (&'a str, &'a str)>) -> Vec<Json> {
    pairs
        .map(|(k, v)| json!({ "@type": kind, "kind": k, "value": v }))
        .collect()
}

/// The card of the person `uid` shown as `display`, from the contact `values` of the directory, a `fallback` e-mail for a person whose
/// card holds none (the preferred address of the directory entry) and the photo `avatar`.
pub(super) fn card(
    uid: &str,
    display: &str,
    values: &[ExcerptValue],
    fallback: Option<&str>,
    avatar: Option<&FileRef>,
) -> Json {
    let mut full = display.to_owned();
    let mut name_parts: Vec<(String, String)> = Vec::new();
    let (mut emails, mut phones, mut addresses) = (Vec::new(), Vec::new(), Vec::new());
    for value in values {
        match value {
            ExcerptValue::FullName(n) => {
                full = n.display();
                name_parts.extend(
                    n.components()
                        .iter()
                        .map(|(k, v)| (k.as_str().to_owned(), v.clone())),
                );
            }
            ExcerptValue::Surname(t) => name_parts.push(("surname".to_owned(), t.canonical())),
            ExcerptValue::GivenName(t) => name_parts.push(("given".to_owned(), t.canonical())),
            ExcerptValue::Email(e) => {
                emails.push(json!({ "@type": "EmailAddress", "address": e.canonical() }));
            }
            ExcerptValue::Phone(p) => {
                phones.push(json!({ "@type": "Phone", "number": p.canonical() }));
            }
            ExcerptValue::Postal(a) => {
                let parts = a.components().iter().map(|(k, v)| (k.as_str(), v.as_str()));
                addresses.push(json!({
                    "@type": "Address",
                    "components": components("AddressComponent", parts),
                    "countryCode": a.country_code(),
                    "full": a.display(),
                }));
            }
            _ => {}
        }
    }
    if emails.is_empty() {
        let bare = fallback.map(|a| a.strip_prefix("mailto:").unwrap_or(a));
        emails.extend(bare.map(|a| json!({ "@type": "EmailAddress", "address": a })));
    }
    let mut card = Map::new();
    card.insert("@type".to_owned(), "Card".into());
    card.insert("version".to_owned(), "1.0".into());
    card.insert("uid".to_owned(), uid.into());
    card.insert("kind".to_owned(), "individual".into());
    let parts = name_parts.iter().map(|(k, v)| (k.as_str(), v.as_str()));
    let mut name = json!({ "@type": "Name", "full": full });
    if !name_parts.is_empty() {
        name["components"] = components("NameComponent", parts).into();
    }
    card.insert("name".to_owned(), name);
    for (member, items) in [
        ("emails", emails),
        ("phones", phones),
        ("addresses", addresses),
    ] {
        if let Some(map) = numbered(&member[..1], items) {
            card.insert(member.to_owned(), map);
        }
    }
    let photo = avatar.map(|f| json!({ "@type": "Media", "kind": "photo", "uri": f.iri.canonical(), "mediaType": f.media_type.0 }));
    if let Some(map) = numbered("m", photo.into_iter().collect()) {
        card.insert("media".to_owned(), map);
    }
    card.into()
}
