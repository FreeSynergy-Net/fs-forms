# Changelog

Version = tag; all crates of the workspace carry the same version.

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
