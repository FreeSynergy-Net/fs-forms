//! Hashing of tokens.
//!
//! Pattern: Facade. The one place that turns a token into what the box keeps is `fs_ballot::hash_token`
//! (`sha256(token)` in hex); this module only names it for the program.

pub use fs_ballot::hash_token;
