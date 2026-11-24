//! X1: the export is compared with a FIXED reference text (not only with another export), so a writer that reverses or reorders
//! bindings, messages or tables cannot pass. The input is in the wrong order on purpose; the output is canonical.
use std::path::Path;

use fs_forms_exchange::{Format, read_layout, write_layout};
use fs_forms_layout::{Limits, OpenCatalog};

const UNSORTED: &str = r#"schema = 3
kind = "frontend"
id = "board"
extends = "net.example.Tasks"
exec = "fs-forms-run"
title = "board-title"

[[page]]
elem = "page:board"
page = "board"
frame = "workbench"
navigation = "single"

[[page.place]]
elem = "block:board/main"
at = "main"
block = "board"

[[page.place.bind]]
elem = "binding:board/main/value/0"
slot = "value"
nth = 0
field = "homepage"
role = "input"

[[page.place.bind.schemes]]
elem = "scheme:a"
scheme = "ssh"

[[page.place.bind.schemes]]
elem = "scheme:b"
scheme = "https"

[[page.place.bind]]
elem = "binding:board/main/title/1"
slot = "title"
nth = 1
field = "title"
role = "display"

[[page.place.bind]]
elem = "binding:board/main/title/0"
slot = "title"
nth = 0
field = "title"
role = "display"

[[message]]
elem = "message:b/de"
key = "b"
lang = "de"
text = "Zwei"

[[message]]
elem = "message:a/en"
key = "a"
lang = "en"
text = "One"

[[message]]
elem = "message:a/de"
key = "a"
lang = "de"
text = "Eins"
"#;

const GOLDEN: &str = r#"schema = 3
kind = "frontend"
id = "board"
extends = "net.example.Tasks"
exec = "fs-forms-run"
title = "board-title"

[[page]]
elem = "page:board"
page = "board"
frame = "workbench"
navigation = "single"

[[page.place]]
elem = "block:board/main"
at = "main"
block = "board"

[[page.place.bind]]
elem = "binding:board/main/title/0"
slot = "title"
nth = 0
field = "title"
role = "display"

[[page.place.bind]]
elem = "binding:board/main/title/1"
slot = "title"
nth = 1
field = "title"
role = "display"

[[page.place.bind]]
elem = "binding:board/main/value/0"
slot = "value"
nth = 0
field = "homepage"
role = "input"

[[page.place.bind.schemes]]
elem = "scheme:a"
scheme = "ssh"

[[page.place.bind.schemes]]
elem = "scheme:b"
scheme = "https"

[[message]]
elem = "message:a/de"
key = "a"
lang = "de"
text = "Eins"

[[message]]
elem = "message:a/en"
key = "a"
lang = "en"
text = "One"

[[message]]
elem = "message:b/de"
key = "b"
lang = "de"
text = "Zwei"
"#;

#[test]
fn x1_the_export_is_the_fixed_reference_text() {
    let doc = read_layout(&mut UNSORTED.as_bytes(), Path::new("g.toml"), Format::Toml).unwrap();
    let layout = doc.check(&OpenCatalog, &Limits::declared()).unwrap();
    let mut out = Vec::new();
    write_layout(&layout, Format::Toml, false, &mut out).unwrap();
    assert_eq!(String::from_utf8(out).unwrap(), GOLDEN);
}
