//! A1/A5 (fs-forms v0.0.3): the reader takes at most `LAYOUT_BYTES_MAX` bytes; more is `import-invalid` with cause `too-big`.
//! Mutants: no `take` at all (the endless source is read on); `take(MAX)` instead of `MAX + 1` (a file one byte over is not seen):
//! the two tests pin both edges.

use std::io::Read;
use std::path::Path;

use fs_forms_exchange::{Format, LAYOUT_BYTES_MAX, read_layout};
use fs_forms_layout::LayoutCode;

/// A source that never ends: the reader must stop by itself.
struct Endless(u64);

impl Read for Endless {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0 += buf.len() as u64;
        assert!(
            self.0 < 4 * LAYOUT_BYTES_MAX,
            "the reader keeps reading past the limit"
        );
        buf.fill(b'#');
        Ok(buf.len())
    }
}

fn cause(problems: &[fs_forms_layout::Coded<LayoutCode>]) -> Vec<String> {
    problems
        .iter()
        .flat_map(|p| p.params.iter())
        .filter(|(k, _)| k == "cause")
        .map(|(_, v)| v.clone())
        .collect()
}

#[test]
fn a_source_without_end_is_stopped_with_too_big() {
    let e = read_layout(&mut Endless(0), Path::new("big.toml"), Format::Toml).unwrap_err();
    assert_eq!(e.len(), 1);
    assert_eq!(e[0].code, LayoutCode::ImportInvalid);
    assert_eq!(cause(&e), ["too-big"]);
}

#[test]
fn exactly_the_limit_is_read_one_more_byte_is_too_big() {
    let max = usize::try_from(LAYOUT_BYTES_MAX).unwrap();
    // comments only: valid TOML, but not a description (a different problem than too-big)
    let at_limit = vec![b'#'; max];
    let e = read_layout(&mut at_limit.as_slice(), Path::new("a.toml"), Format::Toml).unwrap_err();
    assert!(!cause(&e).contains(&"too-big".to_owned()), "{e:?}");
    let over = vec![b'#'; max + 1];
    let e = read_layout(&mut over.as_slice(), Path::new("b.toml"), Format::Toml).unwrap_err();
    assert_eq!(cause(&e), ["too-big"]);
}
