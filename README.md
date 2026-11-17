# FreeSynergy Forms — ask, and decide together

**A poll where an anonymous answer really is anonymous, not just labelled that way.**

FreeSynergy Forms lets a person or a group ask questions, collect answers and read the result. Answers can carry a name, or be anonymous. A survey can start in a small group and later widen to the larger group around it, keeping the answers already given.

What is different here: anonymous answers do not go into the ordinary store at all. They go into a separate box that keeps no log and no time, with answers shuffled. Results appear only in batches of at least a minimum group size, so reading twice cannot reveal one person's answer. If answers are held back and could change the outcome, the program says it cannot name a winner yet. A decision can be linked to the tasks that follow from it.

## What it can do today

Latest tag: `v0.0.2`. The version in this tree is `0.0.2` (program with a window, server and frontend).

- **A window and a server (new in 0.0.2).** `fs-forms` makes sure the server runs and opens the frontend `simple`: the name, the icon, what works today and what comes. Everything that does not work yet says "coming soon"; a missing server or a server that does not fit is said in the window, never a grey window. The window shows no surveys yet: the core is not connected to it. Try `fs-forms --version`, `fs-forms --help`.

- Create and edit surveys with questions and conditions, through the stages draft, open, closed.
- Named answers: save a draft, resume, submit, replace while open.
- Anonymous answers in a separate box, with one-time tokens for "one answer per invited person".
- Results: summary, evaluation with counting methods such as plurality, approval, majority and supermajority, and a CSV export.
- Widen a survey over groups, tested against an in-memory group tree (the real Groups core cannot sit in the same tree until fs-groups is lifted to the same pins).
- Link a decision to task identifiers.
- Tests: 107, with real stores. Checked by an independent reviewer over three rounds; two rounds found serious errors and the last found none.

## What is not there yet

- The window shows no surveys and cannot make or answer one yet. The server answers only what it offers (nothing yet) and the contract version.
- Tokens have no real delivery until the messenger plugs in.
- Public links and embedding are not possible; DDI export is not supported.
- Anonymity has stated limits: someone running the machine can still read memory while a ballot passes. The design file lists them, and a window must show them.
- One survey store per process.
- Contract: draft (`surveys.capnp`, not locked). The basics are released with this draft contract. This frontend and server are its first consumers.
- Open: a public link without an account comes with the server part. Planned, not scheduled. Date polls belong to the program Decisions (Forms supplies the embedded input fields).

## Part of FreeSynergy

Forms belongs to the democracy and project-management suites. FreeSynergy is a family of programs that are free for everyone, forever, and that keep your data yours. The documentation, the manifesto and the overall plan live in [fs-documentation](https://github.com/FreeSynergy/fs-documentation); the reading version is at <https://freesynergy.github.io/fs-documentation/>. Start with the [manifesto](https://freesynergy.github.io/fs-documentation/manifesto/) if you want to know why we do this.


## For developers

- Workspace crates: `fs-forms-core` (core, no GUI) and `fs-forms-methods` (counting methods), `fs-forms-server` (the contract over a local socket), `fs-forms-client` (the contract as a client), `fs-forms-simple` (the frontend `simple`, a process of its own, depends only on the client and fs-components), `fs-forms-bin` (the command `fs-forms`: starter and `serve`).
- Engine and frontend are separate processes (ADR 0021, 0094 draft); the manifest of the frontend is `packaging/frontends/simple.metainfo.xml`.
- The contract is `surveys.capnp` in [fs-interfaces](https://github.com/FreeSynergy/fs-interfaces) (draft, not locked; shown as Forms). Forms owns it.
- Build and test: `cargo build --workspace` and `cargo test --workspace` (the window crates need the system libraries of Blinc; the tests do not open a window).
- Design and decisions: [DESIGN.md](DESIGN.md).
- Licence: MIT OR Apache-2.0.

## Auf Deutsch

**Eine Umfrage, bei der eine anonyme Antwort wirklich anonym ist, nicht nur so beschriftet.** FreeSynergy Forms lässt Menschen oder Gruppen Fragen stellen, Antworten sammeln und das Ergebnis lesen. Anonyme Antworten landen nicht im gewöhnlichen Speicher, sondern in einem eigenen Behälter ohne Protokoll und ohne Zeit; Ergebnisse erscheinen erst ab einer Mindestgruppengröße.

**Was es heute kann.** Letzter Tag `v0.0.2`. Der Stand in diesem Baum ist `0.0.2` (Programm mit Fenster, Server und Frontend). Der Kern (Umfragen mit Fragen und Bedingungen, namentliche und anonyme Antworten, Ergebnisse, Zählverfahren, CSV-Export, Ausweiten über Gruppen) liegt als Bibliothek vor. Neu ist ein Fenster mit Server: `fs-forms` startet den Server und das Frontend `simple` mit Name, Symbol, „Was es heute kann“ und „Was kommt“. Was noch nicht geht, sagt „kommt noch“; fehlt der Server oder passt er nicht, steht das im Fenster. Das Fenster zeigt noch keine Umfragen, der Kern ist noch nicht daran angeschlossen.

**Was noch nicht da ist.** Umfragen im Fenster anlegen, beantworten und auswerten, Einladen von Personen, öffentlicher Link ohne Konto (kommt mit dem Server-Teil, geplant, nicht terminiert). Terminumfragen gehören zum Programm Decisions. Die vollständige Liste steht in der Feature-Liste (`todo/features/forms.md` in fs-documentation, sobald sie dort liegt).

**Für Entwickler.** Frontend und Engine sind getrennte Prozesse; das Frontend hängt nur am Client und an fs-components. Bauen: `cargo build --workspace`, `cargo test --workspace`. Vertrag: Entwurf (`surveys.capnp`, nicht gesperrt). Die Grundfassung erscheint mit diesem Entwurf. Lizenz: MIT OR Apache-2.0.
