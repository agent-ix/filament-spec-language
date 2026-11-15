---
id: SR-2026111414542500000005
title: "Code and Rust review of QSL-1961 S2 handoff compile-back and protocol subject (PR #2717)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-1961; PR #2717 head 3fda08450d402709fd8ec9ae462e3f74b8845861; src/checked_handoff.rs; src/checked_handoff/{selection.rs,tree.rs,corpus.rs}; src/emit_checked_v2.rs; src/population_fixtures.rs; fsl-analyze/src/protocol_analysis/{mod.rs,subject.rs}; Cargo.toml; tests/it/{checked_handoff.rs,checked_handoff_realizability.rs,bare_equality.rs,main.rs}"
review_set: subset
---
# Code and Rust review: QSL-1961 S2

## Summary

Ticket: QSL-1961. PR #2717, scoped to `gh pr diff 2717`. Methods: code-review
with the rust-review lane (rust-style baseline). The full gate was not re-run
(the author reported `make ci` exit 0 on 3fda08450); no focused checks were
needed.

Examined: the `compile`/`compile_inputs`/`read_inputs` split of
`emit_checked_v2::compile` (behaviour of `write` unchanged: same reads, same
order, `supplied_library` builds the same `SuppliedLibrary`); `admit`
extracted from `write_with` unchanged; `checked_handoff::compile` (sidecar read,
package read, input reconstruction, byte-equality check before Contract IR's
read, `Selection::check`); `Selection::inputs` label handling and clock
selection recovery from `clauses` and `compensations` binding rows; the new
sidecar members in `Selection::derive` and `PackageRows` (`deny_unknown_fields`
kept); `tree::read` ceiling (take(limit+1), refuse rather than truncate);
`protocol_subject` (unknown protocol, FR-219 memory resolution, no proof
bounds); `population_fixtures` `operation_bodies` fix; span shifts in tests.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `tree::read` checks the sidecar path only lexically and then `File::open`s it, following symlinks and opening non-regular files; a handoff directory whose `sources/...` or `models/...` entry is a symlink reads a file outside the directory (FR-517 says such a path is `Error::Label`), and a FIFO there blocks `compile` forever | src/checked_handoff/tree.rs:49-66 |

## Verdict

Mergeable after FND-001 or with it deferred by decision. The compile-back path
reuses the spine entry `write` uses (no second evaluator or link path), the
byte-equality check makes the sidecar self-consistent, and the subject builder
is a thin, correct adapter. Error codes `Read` and `Sidecar` are mapped in
`Error::code`. Rust style: no `unwrap` in library code, errors are typed,
public items documented; `SubjectPackage` lacks `Debug` only because
`CheckedPackage` has none.
