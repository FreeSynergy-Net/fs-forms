//! The only source of randomness: the operating system. It lives in `fs-ballot`; this module keeps the
//! names the program uses and maps the error.
//!
//! Pattern: Facade (insertion positions, Fisher-Yates shuffles and tokens all come from `fs_ballot::random`).

use crate::error::Result;

/// Fisher-Yates shuffle.
pub fn shuffle<T>(items: &mut [T]) -> Result<()> {
    Ok(fs_ballot::random::shuffle(items)?)
}

/// 256 random bits as 64 hex characters.
pub fn token() -> Result<String> {
    Ok(fs_ballot::random::token()?)
}
