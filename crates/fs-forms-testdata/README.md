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
- Not here yet: `SampleModule` (`ModuleCore` double), the host switch `--sample-data`, the migration of the old fixtures.

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
- Noch nicht hier: `SampleModule` (Double von `ModuleCore`), der Host-Schalter `--sample-data`, der Umzug der alten Fixtures.

Mit Hilfe von Claude (Anthropic) geschrieben; siehe die Commit-Trailer. Lizenz: MIT OR Apache-2.0.
