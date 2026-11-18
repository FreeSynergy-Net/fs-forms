# Changelog

Version = tag; all crates of the workspace carry the same version.

## Unreleased

### English

- **Probe F1b (layouts), crates `fs-forms-layout`, `fs-forms-exchange`, `fs-forms-store`** (first cut, written with Claude). Open points, stated plainly: overlay ops `Insert` and `Move` are stored but NOT applied (they give conflict `field-unknown`); the import side of the live provider check (K67) is not built, only `resolve` runs it (without a provider `resolve` now says `provider-unchecked`); a refused write of an AI acting on behalf is not logged (K78, the log has no word for it); the update with a failing hook in the middle (K93) has no test, the memory store is atomic by copy; the database adapter (`DbLayoutStore`), the contract `layouts.capnp`, `serve`, emergency mode and `repair` are not written (they wait for fs-objects v5, which is not pinned; `spec.rs` holds stand-ins with its shape); the exchange does not write filters of views or overlay files yet.
- Tables `forms_*` run on the real engine (turso 0.8.3, as dev-dependency of `fs-forms-store`): `forms_binding.from` is renamed `bind_from` (`from` is an SQL keyword and the fs-objects v5 renderer does not quote), and the two checks of the form `CHECK ((x IN (...)) = (...))` are written out as `A AND B OR NOT A AND NOT B`, because turso 0.8.3 refuses exactly this form ("cannot determine type of expression in CHECK"). The plain forms `(A) = (B)` and sums of booleans are accepted by the engine (measured by the reviewer: correction of an earlier, too wide statement). All the other checks were rewritten as well and are equivalent in three-valued logic.
- Overlays (round 4): after every op the result passes the same `check` as any description (local rules, catalog, limits); a person's overlay can never widen a scheme allowlist (`allowlist-widened`; only the instance or the package itself may); a source can only be removed when it is the last one (a removal in the middle would renumber the sources and point blocks at another source). Security allowlists in this crate: the scheme list of a field use (restrict only for a person), the fixed never-list (`javascript`, `vbscript`: nobody may add them), the block catalog and `repeat` per slot (checked again after every op), the technical `nth < 16`, the limits, and the instance locks. Command and workflow names are only names here: whether a person may use them is decided by the host.
- Open points found by the review of round 4 (stated plainly): (K1) conflicts are computed against the original alone when an overlay is written and against the whole stack in `resolve`; example: the instance removes source 3, a person removes source 2 -> the stored conflict says `result-invalid`, yet the op takes effect in `resolve`. It is safe (`resolve` checks everything) but the stored conflict can mislead; to be fixed by computing conflicts against the stack or by saying so in the design. (K2) A person can set the `path` of a field binding; a binding WITHOUT a scheme list means "nothing is allowed" (fail closed): the host must treat a missing list that way (the test belongs to the host order, not to this crate). (K3) `text_bytes_max` is applied to messages only. The signature of `apply` changed in round 4 (catalog and limits are passed in): F2 and H1 must follow.
- `LayoutCode` has 45 codes (new at the END of the list: `menu-invalid`, `source-unknown`), with German and English texts for all of them in `fs-forms-layout/src/texts/`.
- `LayoutDoc::check` now runs the same local rules as the tables (menu pairs, source numbers, lower-case schemes, ranges with both bounds) and `apply` runs them after every overlay op (an overlay can no longer set `javascript`).
- Windows-CI (`windows.yml`) taken over from H1 (`unreviewed/probe-forms-h1-bind`), byte for byte, so that merging the two branches has no conflict.
- `windows.yml` adapted to the crates of this branch (`-p fs-forms-layout -p fs-forms-exchange -p fs-forms-store`); when merging with H1, unite the `-p` list. Transitional: replaced by a caller of `_windows-check.yml` after fs-release v0.0.11.
- The crate `fs-forms-methods` is gone: the ten counting methods are the library `fs-tally` (own repo, tests moved with it). The box logic (`Lane`, token hash, OS randomness) is the library `fs-ballot`; `fs-forms-core` keeps its two lanes as `fs_ballot::Lane<Ballot, Signals>` (`Signals` implements `fs_ballot::Meta`). Behaviour and stored JSON field names are unchanged; the legacy single lane of box 0.1.0 now also keeps its pending pattern counts.
- **Pins:** `fs-tally` and `fs-ballot` are pinned to their tag `v0.0.1`. `.fs-lock-sources.toml` stays strict.
- Still open: the box is one JSON document in `fs-objects` (typed columns are the separate step D2b).

### Deutsch

- Das Crate `fs-forms-methods` entfällt: die zehn Zählverfahren sind die Bibliothek `fs-tally` (eigenes Repo, Tests mitgezogen). Die Urnen-Logik (`Lane`, Token-Hash, Zufall des Systems) ist die Bibliothek `fs-ballot`; `fs-forms-core` hält seine zwei Bahnen als `fs_ballot::Lane<Ballot, Signals>` (`Signals` setzt `fs_ballot::Meta` um). Verhalten und gespeicherte JSON-Feldnamen unverändert; die alte Einzelbahn der Urne 0.1.0 behält jetzt auch ihre wartenden Muster-Zählwerte.
- **Pins:** `fs-tally` und `fs-ballot` hängen an ihrem Tag `v0.0.1`. `.fs-lock-sources.toml` bleibt streng.
- Noch offen: die Urne ist ein JSON-Dokument in `fs-objects` (typisierte Spalten sind der eigene Schritt D2b).

## 0.0.2

### English

- **Window and server (basics, workflow `grundfassung`).** New crates: `fs-forms-server` (the contract `surveys.capnp` draft 0.1 over a local socket through `fs-rpc` v0.0.2; answers `capabilities` with no capability ids and the contract version, and `supportReport`; everything else is `unimplemented`), `fs-forms-client` (connect with the version handshake, `Hello::fetch`, contract check: same main level, minor level of the server not lower), `fs-forms-simple` (the frontend `simple` as a process of its own: name, icon, what works today, what comes, every element locked with "coming soon"; depends only on the client and fs-components v0.0.25), `fs-forms-bin` (the command `fs-forms`: starter, `serve`, `--version`, `--help`).
- **Starter.** `fs-forms` starts or replaces the server (handshake of `fs-rpc`), asks the frontend to check the contract (`--check`, exit code 3 = does not fit) and falls back to the standard frontend `simple` with a visible message; a missing or unfitting server is said in the window.
- **Pins lifted so the tree holds one source per library:** `fs-objects` v0.0.1 to v0.0.6, `fs-interfaces` v0.0.1 to v0.0.27, new `fs-rpc` v0.0.2, `fs-types` v0.0.1 (through them); `.fs-lock-sources.toml` strict. Minimum Rust 1.89 for the workspace. The core and the methods crate are otherwise unchanged.
- **CI:** `check.yml` and `release.yml` call fs-release at `v0.0.10`; .
- The real icon (`packaging/net.freesynergy.Forms.png`, `.desktop`, embedded in the window); `fs-forms` depends on the package `fs-forms-simple` (deb `depends`, rpm `requires`).
- Contract: draft (`surveys.capnp`, not locked). The basics are released with this draft contract.
- Not done: the window shows no surveys (the core is not connected to the server), no way to make or answer a survey, no lease (the server keeps running after the window closes until it is stopped or replaced).
- Tests: `fs-groups-core` left the dev-dependencies (other pins); widening is tested against an in-memory `GroupLookup` until fs-groups has a tag on the same pins.

### Deutsch

- **Fenster und Server (Grundfassung, Workflow `grundfassung`).** Neue Crates: `fs-forms-server` (der Vertrag `surveys.capnp` Entwurf 0.1 über einen lokalen Socket mit `fs-rpc` v0.0.2; antwortet auf `capabilities` ohne Fähigkeits-Kennungen mit der Vertragsversion und auf `supportReport`; alles andere ist `unimplemented`), `fs-forms-client` (Verbinden mit Versions-Handschlag, `Hello::fetch`, Vertragsprüfung: gleiche Hauptstufe, Nebenstufe des Servers nicht niedriger), `fs-forms-simple` (das Frontend `simple` als eigener Prozess: Name, Symbol, was heute geht, was kommt, jedes Element mit „kommt noch“ gesperrt; hängt nur am Client und an fs-components v0.0.25), `fs-forms-bin` (der Befehl `fs-forms`: Starter, `serve`, `--version`, `--help`).
- **Starter.** `fs-forms` startet oder ersetzt den Server (Handschlag von `fs-rpc`), lässt das Frontend den Vertrag prüfen (`--check`, Rückgabecode 3 = passt nicht) und fällt mit sichtbarer Meldung auf das Standard-Frontend `simple` zurück; ein fehlender oder unpassender Server steht im Fenster.
- **Pins angehoben, damit der Baum je Bibliothek eine Quelle hat:** `fs-objects` v0.0.1 auf v0.0.6, `fs-interfaces` v0.0.1 auf v0.0.27, neu `fs-rpc` v0.0.2, `fs-types` v0.0.1 (durch diese); `.fs-lock-sources.toml` streng. Mindest-Rust 1.89 für den Arbeitsbereich. Kern und Methoden-Crate sind sonst unverändert.
- **CI:** `check.yml` und `release.yml` rufen fs-release bei `v0.0.10` auf; .
- Das echte Symbol (`packaging/net.freesynergy.Forms.png`, `.desktop`, im Fenster eingebettet); `fs-forms` hängt vom Paket `fs-forms-simple` ab (deb `depends`, rpm `requires`).
- Vertrag: Entwurf (`surveys.capnp`, nicht gesperrt). Die Grundfassung erscheint mit diesem Entwurf.
- Nicht getan: das Fenster zeigt keine Umfragen (der Kern ist nicht am Server angeschlossen), keine Möglichkeit, eine Umfrage anzulegen oder zu beantworten, keine Lease (der Server läuft nach dem Schließen des Fensters weiter, bis er beendet oder ersetzt wird).
- Tests: `fs-groups-core` ist keine Dev-Abhängigkeit mehr (andere Pins); Ausweiten wird gegen einen `GroupLookup` im Speicher geprüft, bis fs-groups einen Tag auf denselben Pins hat.
