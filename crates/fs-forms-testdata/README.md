# fs-forms-testdata

## English

Generic, deterministic test data made from the declaration of a form. A `SampleSpec` (seed, anchor = period + IANA zone, one
`SetWish` per form) and the form as `fs_interfaces::forms` declares it give a `SampleSet` of ActivityStreams 2 objects
(`fs_objects::StoredObject`). The content follows the standard of the AS2 type: JSCalendar for `Event`, plain AS2 otherwise.

- Pattern: Test Data Builder + Strategy. One `ValueMaker` per kind of channel (`Art`); hand-made edge cases (`Case`) per kind of
  relation (`Relation::Order`, `Relation::Parent`), never per channel name or program.
- Deterministic: ChaCha8, one stream per form; the words come from the crate `fake` (pinned exactly, no word lists of our own).
  `SampleSet::render` is the text the golden tests freeze (renew on purpose with `FS_UPDATE_GOLDEN=1`).
- Never silent: a wish that cannot be met is a `SampleCode` (`sample-error-*`).
- `SampleModule` (package 3): the set served as a `ModuleCore` (level 2, provider text `fake`, in memory). `page` projects the objects to rows (RFC 3339 with the offset of the zone, whole days as `full-date`); `apply` and `revert` run every command from the `writes` column of the declaration (no name is matched) and keep a log (compare-and-set, one effect per step, undo, `changes`, `feed`); rights = everything except the `Forbid` wishes of the spec. It passes the 22+ rules of `module_v1` (fs-module suite behind `fs_rpc::Guarded`). A command whose declaration has no `writes`, or an empty one (`open` and `remove` look the same), is refused with a code, never guessed.
- Not here yet: the host switch `--sample-data` (package 4), the migration of the old fixtures (package 5).

Written with the help of Claude (Anthropic); see the commit trailers. Licence: MIT OR Apache-2.0.

## Deutsch

Allgemeine, deterministische Testdaten, erzeugt aus der Deklaration einer Form. Ein `SampleSpec` (Saat, Anker = Zeitraum +
IANA-Zone, ein `SetWish` je Form) und die Form, wie `fs_interfaces::forms` sie deklariert, ergeben ein `SampleSet` aus
ActivityStreams-2-Objekten (`fs_objects::StoredObject`). Der Inhalt folgt dem Standard des AS2-Typs: JSCalendar für `Event`,
sonst reines AS2.

- Pattern: Test Data Builder + Strategy. Ein `ValueMaker` je Kanal-Art (`Art`); Randfälle (`Case`) je Art von Beziehung
  (`Relation::Order`, `Relation::Parent`), nie je Kanalname oder Programm.
- Deterministisch: ChaCha8, ein Strom je Form; die Wörter kommen aus dem Crate `fake` (exakt gepinnt, keine eigenen Wortlisten).
  `SampleSet::render` ist der Text, den die Goldtests einfrieren (bewusst erneuern mit `FS_UPDATE_GOLDEN=1`).
- Nie still: ein Wunsch, der sich nicht erfüllen lässt, ist ein `SampleCode` (`sample-error-*`).
- `SampleModule` (Paket 3): die Menge als `ModuleCore` (Stufe 2, Provider-Text `fake`, im Speicher). `page` projiziert die Objekte auf Zeilen (RFC 3339 mit Offset der Zone, ganztägig als `full-date`); `apply` und `revert` führen jeden Befehl aus der Spalte `writes` der Deklaration aus (kein Name wird verglichen) und führen ein Protokoll (Compare-and-Set, eine Wirkung je Schritt, Rückgängig, `changes`, `feed`); Rechte = alles erlaubt außer den `Forbid`-Wünschen der Spec. Besteht die 22+ Regeln von `module_v1` (Suite von fs-module hinter `fs_rpc::Guarded`). Ein Befehl, dessen Deklaration kein `writes` hat oder ein leeres (`open` und `remove` sehen gleich aus), wird mit einem Code abgelehnt, nie geraten.
- Noch nicht hier: der Host-Schalter `--sample-data` (Paket 4), der Umzug der alten Fixtures (Paket 5).

Mit Hilfe von Claude (Anthropic) geschrieben; siehe die Commit-Trailer. Lizenz: MIT OR Apache-2.0.
