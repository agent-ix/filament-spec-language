---
id: SR-2026111612172400000040
title: "Code review of QSL-2430 PR #2928"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-analyze/src/analyze/document.rs, fsl-analyze/src/analyze/request.rs, fsl-analyze/src/protocol_analysis/item.rs, fsl-replay/src/protocol_analysis/artifact.rs, fsl-analyze/tests/it/protocol_analysis_differential.rs, fsl-analyze/tests/it/protocol_item_bounds.rs; QSL-2430"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-636
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-570
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-571
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-574
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-575
    type: reviews
---

## Summary

Ticket: QSL-2430. Reviewed PR #2928 at `7f3faa7008df36ef25d51539011f16800ed4a9d1` against the request route, assessed-root identity, protocol subject/role/bounds propagation, and the FR-636/FR-570/FR-571/FR-574/FR-575 acceptance surface. The CLI differential and W45 bound tests pass, but the upper bound is not handled consistently and an unknown role can settle as unsupported instead of invalid input.

## Verdict

CHANGES REQUESTED. The new CLI route carries the checked-v2 package, protocol binding, role, premises and assessed root into the projection/realizability engines; the targeted tests exercise this route. The implementation does not consistently enforce FR-636's bound range or invalid-member settlement, and the missing-role lookup falls through to an unadapted capability.

## Coverage

Examined FR-636-AC-2 and AC-5..7; FR-570, FR-571, FR-574 and FR-575; TC-1596 and TC-2988; request parsing, CheckedPackageV2 admission, protocol subject construction, role resolution, assessed-root parsing, engine request construction, settlement mapping, and the changed CLI tests. The `fsl-analyze` Rust lane was reviewed as part of this code review. The targeted tests passed: `cargo test -p fsl-analyze --test it protocol_item_bounds -- --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart --skip a_recovery_predicate_over_the_recovery_view_is_evaluated --test-threads=1` (3 passed), and `cargo test -p fsl-analyze --test it protocol_deadlock_agrees_with_projection -- --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart --skip a_recovery_predicate_over_the_recovery_view_is_evaluated --test-threads=1` (1 passed). No edits were made to the PR worktree.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | `temporal.max_history` is only checked for zero, so values above 2^53 - 1 are admitted and passed to the realizability engine despite FR-636 requiring an in-range bound. | fsl-analyze/src/analyze/document.rs:642,1505-1509; fsl-analyze/src/analyze/request.rs:690; FR-636 | implementation-bug-despite-evidence |
| FND-002 | medium | `temporal.max_depth` above 2^53 - 1 fails `read_max_depth` during whole-document deserialization, instead of settling only that item as `invalid-request`/`invalid-value` while other items run, as FR-636 specifies. | fsl-analyze/src/analyze/document.rs:509-555,1492-1496; FR-636 | implementation-bug-despite-evidence |
| FND-003 | medium | A supplied role name absent from the checked package returns `AnalyzeItem::unadapted`, which settles `unsupported-requested-capability`; the request path does not preserve the projection adapter's invalid-role settlement for this case. | fsl-analyze/src/analyze/request.rs:627-640; FR-570, FR-575 | implementation-bug-despite-evidence |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-004 | medium | The shared `temporal.max_depth` reader now accepts values above the prior safe-count/usize ceiling for every item kind, while only protocol-analysis items validate the bound; ordinary temporal claims can reach the engine with a horizon of `usize::MAX`. | fsl-analyze/src/analyze/document.rs:553-557,2064-2068; FR-620, FR-625 | implementation-bug-despite-evidence |
| FND-005 | medium | Numeric values greater than `u64::MAX` still fail `u64::deserialize` at whole-request parsing, so out-of-range protocol bounds of that size cannot settle only their item as FR-636 requires. | fsl-analyze/src/analyze/document.rs:556-557,660-664; FR-636 | implementation-bug-despite-evidence |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | still-open | The shared reader no longer enforces the prior MAX_COUNT/usize ceiling for non-protocol items, and the non-protocol branch has no replacement validation. |
| FND-005 | still-open | `u64::deserialize` rejects integers above u64 before per-item validation; the current test covers only 2^53, which fits u64. |
