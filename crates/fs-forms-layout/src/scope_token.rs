//! The free scope word of a source: one `scope-token` of OAuth 2.0 (RFC 6749 section 3.3), at most 256 bytes.
//!
//! Pattern: Value Object - an opaque word checked once at construction (parse, don't validate); what it means belongs to the module
//! that serves the source, this crate only guarantees the grammar. No crate: the grammar is one byte predicate, `oauth2::Scope` checks nothing.

/// The longest scope word, in bytes (a limit of this format, not of the RFC).
pub const SCOPE_TOKEN_MAX: usize = 256;

/// A scope word: `1*( %x21 / %x23-5B / %x5D-7E )`, so printable ASCII without space, `"` and `\`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopeToken(String);

impl ScopeToken {
    /// The token of `text`, or `None` when it is empty, longer than [`SCOPE_TOKEN_MAX`] bytes or has a byte outside the grammar.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let ok = !text.is_empty()
            && text.len() <= SCOPE_TOKEN_MAX
            && text
                .bytes()
                .all(|b| b == 0x21 || (0x23..=0x5B).contains(&b) || (0x5D..=0x7E).contains(&b));
        ok.then(|| Self(text.to_owned()))
    }
    /// The word as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
