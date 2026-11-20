---
id: SR-20261120073950
title: "code-review (rust) of filament-spec-language PR 3157 body_locations producer and reader"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 3157 head e21589a2f285fef2f3db2c688c3094b86f7ccb92; fsl-package/src/emit/body_locations.rs, emit/protocol_locations.rs, emit.rs, introduced_binders.rs, reader_protocol_projection.rs, bare.rs, emit/probabilistic.rs, tests"
review_set: subset
---

## Summary

Ticket: QSL-2625 (also QSL-2648, QSL-2652). Rust code review of the body_locations producer, the output-limit charge, the reader projection and the IEEE operand binder handling.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `#ieee_operand` scopes no longer hide their binders in any walk, so protocol slot and temporal clause terms with a non-trivial IEEE ordering report the checker's operand binders as free binders | fsl-package/src/introduced_binders.rs:263-268 |
| FND-002 | high | Gate red: cargo fmt --check fails on the PR's own line, and the string_edge marker moved off `parameter_name` onto a const while new string compares are unmarked | fsl-package/src/reader_protocol_projection.rs:3581-3605 |
| FND-003 | medium | Row charge (8 bytes per path index plus 64 per row) overcharges real JSON bytes, so a body deeper than about 200 is refused at a bound its canonical bytes fit | fsl-package/src/emit/body_locations.rs:27-32 |
| FND-004 | medium | Reader gives every let/if/case node its last child's location, a location the wire does not carry | fsl-package/src/reader_protocol_projection.rs:198-225 |
| FND-005 | medium | Identity file round trip drops SelectedSourceObjectOperation.parameters, which the emitter now writes | fsl-package/src/bare.rs:788-797 |
| FND-006 | low | `rekey_timed_model_nodes` doc comment now attaches to `replace_term_data_node` | fsl-package/src/emit.rs:5833-5836 |
| FND-007 | low | `.or(None)` is a no-op | fsl-package/src/reader_protocol_projection.rs:2959 |
| FND-008 | low | `#ieee_operand` binder choice is lowest-level-not-bound, a heuristic; the exact rule is level equal to the let's binder position | fsl-package/src/reader_protocol_projection.rs:2950-2960 |

## Verdict

Blocking on FND-001 and FND-002. Pins are clean: one rev per first-party crate, contract-ir 308116a0 and filament-specification 1c70322b are the main tips, verification-contracts cd0c8ef6 is transitive through contract-ir's own pin, no new patch section and no .cargo directory. Charge is applied before each path is built. QSL-2652 #1082 adaptations cover every construction site. merge-tree against origin/main 38dbd6b7 is clean.
