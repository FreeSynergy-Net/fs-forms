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
- `SampleModule` (package 3): the set served as a `ModuleCore` (level 2, provider text `fake`, in memory). `page` projects the objects to rows (RFC 3339 with the offset of the zone, whole days as `full-date`); `apply` and `revert` run every command from the `writes` column of the declaration (no name is matched) and keep a log (compare-and-set, one effect per step, undo, `changes`, `feed`); rights = everything except the `Forbid` wishes of the spec. The suite of `module_v1` (fs-module, behind `fs_rpc::Guarded`) checks it 119 times with no failure of the module itself; 7 rules are skipped (6, 11, 18 twice, 20, 21, 22, L2-9), and the failures that come from the wrapper (14 of three kinds, the same kinds as for the control) are listed in the CHANGELOG. A command whose declaration has no `writes`, or an empty one (`open` and `remove` look the same), is refused with a code, never guessed.
- From a description (package 4a): `SampleSpec::from_description` / `SampleModule::from_description` take the forms the blocks need (port `BlockForms`), the section `[sample]` (`read_sample`) or the defaults of the declaration. Not here yet: the host switch `--sample-data` over the sources (package 4b), the migration of the old fixtures (package 5).

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
- `SampleModule` (Paket 3): die Menge als `ModuleCore` (Stufe 2, Provider-Text `fake`, im Speicher). `page` projiziert die Objekte auf Zeilen (RFC 3339 mit Offset der Zone, ganztägig als `full-date`); `apply` und `revert` führen jeden Befehl aus der Spalte `writes` der Deklaration aus (kein Name wird verglichen) und führen ein Protokoll (Compare-and-Set, eine Wirkung je Schritt, Rückgängig, `changes`, `feed`); Rechte = alles erlaubt außer den `Forbid`-Wünschen der Spec. Die Suite von `module_v1` (fs-module, hinter `fs_rpc::Guarded`) prüft es 119-mal ohne Fehler des Moduls selbst; 7 Regeln werden übersprungen (6, 11, 18 twice, 20, 21, 22, L2-9), und die Fehler, die vom Wrapper kommen (14 von drei Arten, dieselben Arten wie bei der Kontrolle), stehen im CHANGELOG. Ein Befehl, dessen Deklaration kein `writes` hat oder ein leeres (`open` und `remove` sehen gleich aus), wird mit einem Code abgelehnt, nie geraten.
- Aus einer Beschreibung (Paket 4a): `SampleSpec::from_description` / `SampleModule::from_description` nehmen die Formen, die die Blöcke brauchen (Port `BlockForms`), den Abschnitt `[sample]` (`read_sample`) oder die Vorgabe aus der Deklaration. Noch nicht hier: der Host-Schalter `--sample-data` über die Sources (Paket 4b), der Umzug der alten Fixtures (Paket 5).

Mit Hilfe von Claude (Anthropic) geschrieben; siehe die Commit-Trailer. Lizenz: MIT OR Apache-2.0.
