# fs-forms-bind

The pure build plan of FreeSynergy Forms: layout + block catalog + provider facts in, a `Plan` out. Every problem lands as a
visible `PartLock` on the smallest part it concerns (source, page, placement, slot, menu entry, command) or as a window
`Notice`; nothing is dropped silently. No I/O, no clock, no toml or JSON, no panics, no `Result` from `Bind::build`.

- `Bind::build` is the builder; the `Plan` is a value object (`Eq`), its SHA-256 (`digest_without_style`) uses a fixed
  encoding (labels, lengths, UTF-8), independent of `Debug` and of the compiler.
- Commands: one decision (`command_state`) for menu, context menu, button and key; the right comes first.
- Keys: the real `fs_components_commands::Keymap`; reserved keys cannot be bound; `delete` runs only in a list.
- Features: `testing` (test doubles, fixtures), `conformance` (the `PlanSuite`: 12 rules, 12 bench mutants).
- Input types (`Resolved`, `Catalog`, `Offer` ...) are narrow mirrors of the F1/F1b types until their crates are pinned.

Written with the help of Claude (Anthropic); see the commit trailers. Licence: MIT OR Apache-2.0.
