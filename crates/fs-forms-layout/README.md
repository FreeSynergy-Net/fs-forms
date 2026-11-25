# fs-forms-layout

Part of FreeSynergy Forms (probe F1b, first cut): the typed model of a forms description, overlays and the storage port (with an in-memory store).

Written with the help of Claude (Anthropic); every change is reviewed by a second Claude instance and checked by the gates of the repository. Open points are listed in the CHANGELOG of the repository.

Licence: MIT OR Apache-2.0.

## Rule for the host

A field binding without a scheme list means "no scheme is allowed" (fail closed). A person's overlay can never widen a scheme list; the instance can (its scheme list, default `https`, `http`, `ssh`; a closed positive list). The schemes `javascript`, `vbscript`, `data`, `file`, `blob` are never allowed, whatever the instance says (`scheme-not-allowed`).
