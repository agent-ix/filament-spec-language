---
id: SR-979
title: "code-review of PR 3134 per-quantifier hyper bindings and union payload widening"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@56f5af1d3674f74f735d034cf543884996d25d3c; fsl-command/src/checked_handoff/selection.rs, fsl-command/src/emit_checked_v2.rs, fsl-package/src/emit.rs, fsl-package/src/lib.rs, fsl-package/src/emit/tests/hyper_clause.rs, fsl-package/src/emit/tests/union_nodes.rs, fsl-semantics/src/check/check/typing.rs, fsl-semantics/tests/it/union_payload_widening.rs, tests/it/checked_handoff.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-488
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-517
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-317
    type: reviews
---
# SR-979: code-review of PR 3134 per-quantifier hyper bindings and union payload widening

## Summary

Ticket: QSL-2631 (also QSL-2632). PR agent-ix/filament-spec-language#3134, head 56f5af1d3674f74f735d034cf543884996d25d3c. This review includes the rust-review lane. The emitter now writes one `temporal_bindings` row per quantifier, keyed on the `hyperproperty` node at its `claim` occurrence. The handoff sidecar counts hyper wrappers, hybrid claims and `claim/hyperproperty` nodes. `Callee::Union` now reports the member payload type as the call parameter. No high findings. The open gaps are an untested half of FR-517-AC-25, the `fsl compile` handoff path dropping hyper selections, and a sidecar member that no spec defines.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-517-AC-25's hybrid-claim half (count 3, passes check) and its `native-compile/1` handoff exit-0 half are untested; the new `hybrid_claim_sites` chain in the count has no test | tests/it/checked_handoff.rs:2991-3014; fsl-command/src/checked_handoff/selection.rs:494-503 |
| FND-002 | medium | `handoff_clock_selections` maps the spine's derived selections back to names only for temporal clauses and compensations, so a clocked hyper clause's selection is dropped and `fsl compile` with a `handoff` over it cannot write a consistent handoff | fsl-command/src/command.rs:2779-2808 |
| FND-003 | medium | The `hypers` sidecar member (`node`, `name`, `traces`, `binding`, omitted when empty) is not in FR-517's sidecar definition, and FR-517 still says the emitter disagrees with FR-488 today | fsl-command/src/checked_handoff/selection.rs:155-176; spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md:99-115,138-147 |
| FND-004 | low | `check_hyper` matches rows on declaration and trace only, so two equal hyper clauses that share one `hyperproperty` node and get different request clocks are both compared with the occurrence-0 rows and falsely refused | fsl-command/src/checked_handoff/selection.rs:836-845 |
| FND-005 | low | On the request path a hyper clause whose quantifiers range over different models takes one caller clock that the emitter copies to every trace row; the compile path refuses the same clause `PerTraceClockConfigurations` | fsl-command/src/emit_checked_v2.rs:636-652; fsl-package/src/emit.rs:8887-8897 |

## Scope and evidence

- FR-488 "Clause names and clock bindings" (examined): the emitter writes one row per quantifier, in quantifier order, on the `hyperproperty` NodeId at the claim ordinal, with `trace` and with no wrapper row (emit.rs:8838-8898). The clock is the request selection, which the spine derives (fsl-replay/src/spine/clocks.rs:416-447, 504-528) and refuses `PerTraceClockConfigurations` when traces disagree.
- FR-517-AC-26 (examined): both tests check two rows `a`,`b` on the hyperproperty node, no wrapper row, strict-reader admission, a wrapper-keyed row refused at `/temporal_bindings/0/declaration`, and `Selection::check` with a negative on the sequence authority.
- FR-517-AC-25 (examined): the count of 2 and 1, compile-back, check, and the count-1 negative are tested. The hybrid and native-compile halves are not (FND-001).
- FR-317-AC-7 and FR-317-AC-8 (examined): `integer_widens` admits only containment, so a payload that does not fit gets `unproved-range` from the expect path, as the AC-8 test shows. For a nullary member `parameter` is `None` and `admit_shape` refuses arguments. For a multi-payload member each index is used. A composite or nested-union payload gives no conversion. The `lowers_as_integer` exception is kept. The negative test edits the package by hand and passes it to the strict `read_checked_package` with positive controls (emitted, re-derived identity, construction removed). The refusal is located at the `union_value` node.
- `clock_selections`: an unknown name still falls through to `UnknownClause`, and an unclocked hyper selection is refused by the emitter as unmatched. No Python, shims, new `#[ignore]`, weakened assertions or pin changes. `git merge-tree` against origin/main cc1c4d22 is clean.
- Exported `hyper_wrapper` and `hyper_clock_name` are used only by the sidecar derivation. This is the smallest surface that reads the wrapper key and the clock text, so there is no finding.

## Verdict

Mergeable: no high findings. The mediums should be fixed in this PR (FND-001, FND-002) or covered by a spec PR (FND-003).
