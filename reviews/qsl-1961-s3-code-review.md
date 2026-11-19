---
id: SR-2026111414542500000007
title: "Code and Rust review of QSL-1961 S3 protocol step view and refinement witness (PR #2721)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-1961; PR #2721 head 32f4ab2fb468a3675dab618845fe4dba9b72d36e; fsl-replay/src/refinement/{step_view.rs,protocol_witness.rs,protocol_system.rs}; fsl-replay/src/refinement.rs; fsl-replay/src/lib.rs; fsl-replay/src/protocol_analysis/arena.rs; fsl-analyze/tests/it/{protocol_step_view.rs,protocol_realizability.rs,main.rs}"
review_set: subset
---
# Code and Rust review: QSL-1961 S3

## Summary

Ticket: QSL-1961. PR #2721, scoped to `gh pr diff 2721`. Methods: code-review
with the rust-review lane (rust-style baseline). No build or test run was
needed; findings were traced by reading the head and its `fsl-eval` callees.

Examined: `protocol_step_view` (dynamic-construct listing order, refusal
mapping of `AbstractProtocolSystem::new`), `explore` (state indexing by
canonical key, state limit, cut and undetermined refusals, terminal
classification), `step` (node, channel and record labels), the new
`AbstractProtocolSystem::successors_cut_and_terminal` and
`binder_record_bytes` (both delegate to `fsl-eval`'s `successors_bounded`,
`classify` and `binder_record_bytes`), `ArenaNames::channel_node`,
`RefinementWitness` (reuses `ProtocolWitnessWire::of`/`validate`, the PA-24
writer and reader; `deny_unknown_fields`; canonical round-trip check), and the
test-fixture visibility changes.

Hard constraints: no decision procedure is added (the view decides no hiding,
refinement or composition, and FR-147/FR-530 are untouched); no step rule is
restated (every step and terminal class comes from `fsl-eval`'s
`ProtocolSystem`); no legacy or compatibility path.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `explore` detects an undetermined finding through `finding_value`, which returns `Lose` as soon as it meets an `InvariantViolated`; a reached state that has an invariant violation and also an `InvariantUndetermined`, `ContractUndetermined` or `ProtocolExpressionUndetermined` finding is therefore not refused, so the view is served although FR-578 Behavior says a reached state with an undetermined finding refuses it | fsl-replay/src/refinement/step_view.rs:264-266; fsl-replay/src/protocol_analysis/arena.rs:234-249 |
| FND-002 | low | `successors_cut_and_terminal` repeats the three-arm Sc/Tso/Ra candidate re-wrapping of `successors_and_terminal` instead of the old method delegating to the new one and dropping the cut, leaving two copies of the same wrapper to keep in step | fsl-replay/src/refinement/protocol_system.rs:257-345 |

## Verdict

Mergeable after FND-001. The view and the witness meet the hard
constraints: one `fsl-eval` system, no step rules of FSL's own, no decision
procedure, the witness written and read by the existing PA-24 wire type.
FND-002 is a cleanup.
