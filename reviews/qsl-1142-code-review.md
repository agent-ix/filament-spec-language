---
id: SR-20261120215957
title: "code and Rust review of PR 2984 FIFO-key-stable channel keys"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-package/src/emit/tests/protocol_graph.rs, fsl-semantics/src/check/check.rs, fsl-semantics/src/check/lowering/protocol.rs, fsl-semantics/src/check/protocol_controls.rs; PR 2984 head 84205e614d"
review_set: subset
---

## Summary

Ticket: QSL-1142. The PR deletes the S4 lowering guard (`fifo_key_unstable` / `equality_stable`) that refused bounded, quantity and model-owned FIFO keys. It replaces the S3 `holds_float` key check with `Scope::reaches_float`, which also walks record, tuple and union fields. It also un-ignores the bounded, quantity, composite and model-owned key tests. The S3 change matches FR-881: a float leaf anywhere in the key refuses, and `WithFloat` now refuses at the key span with the full `KeyEquality` detail. The test and trace side does not fully back FR-880-AC-10 and FR-881-AC-2.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-880-AC-10 key `Int[0, 99]` was removed from the stable-key test, and no test keys the model value type `PartId`; if the case was dropped because it fails, S3 admits a key the reader refuses (FR-881-AC-2) | fsl-package/src/emit/tests/protocol_graph.rs:1838 |
| FND-002 | medium | Key tests retagged to bare `FR-881`; FR-881-AC-2 now has zero tagged tests and FR-880-AC-10's key half has none (quoin matrix on the PR head) | fsl-package/src/emit/tests/protocol_graph.rs:824 |
| FND-003 | medium | `ProtocolConstruct::ChannelKeyNotEqualityStable` is no longer constructed anywhere; its doc still describes the deleted reader rule and its tracking arm still points at QSL-1142 | fsl-semantics/src/check/refusal.rs:952 |
| FND-004 | low | Stale `Quantity<m>` FIFO skip in `a_channel_carries_every_type_the_reader_admits` still hides the case this PR admits | fsl-package/src/emit/tests/protocol_graph.rs:749 |
| FND-005 | low | Deleted test's cases `Option<Quantity<m>>`, `Sequence<Int[0, 5]>` and `WithQuantity` keyed by `m` were not carried over | fsl-package/src/emit/tests/protocol_graph.rs:1838 |
| FND-006 | low | `reaches_float` copies `reaches_event`'s composite walker verbatim; two copies of one traversal will drift | fsl-semantics/src/check/check.rs:2098 |

## Scope and evidence

- FR-881-AC-1 (examined): float, optional float, float record field and event keys refuse at S3 with `ill_typed`/`operator-ineligible` at the key span, keeping `=`, the channel, the key text and the type. Clean for the diff: `an_unstable_fifo_key_refuses_at_the_key` now covers `WithFloat` with full detail. The message text is covered in fsl-replay spine.rs, and events are covered in model_state_tests.rs.
- FR-881-AC-2 (examined): S3 must admit a key exactly when the FR-726 reader does. See FND-001 and FND-002.
- FR-881-AC-3 (context_only): untagged before and after this PR, so it is not a regression of this diff.
- FR-880-AC-10 (examined): see FND-001 and FND-002.
- fsl-semantics/src/check/lowering/protocol.rs (examined): the guard removal is clean. The remaining imports are still used, and `unemitted` is correctly no longer `mut`.
- fsl-semantics/src/check/protocol_controls.rs (examined): clean.
- Gates: no cargo fmt, clippy or test was run, because the brief made this review read-only while the coder rebuilds. The PR body says verification is pending the QSL-2496 reader pin. `quoin matrix --json` (quoin 0.36.8) ran on a `git archive` of the PR head.
- quoin:gap-analysis: only the matrix step ran, without cargo. The reverse-gap and stub sweeps are folded into this diff-scoped review. This is not a full gap audit.

## Verdict

Not mergeable as reviewed. FND-001 (high) and FND-002 (medium) must be fixed in this PR, FND-003 should be fixed in it too, and the build must then pass on the final reader pin. The core S3 logic change is sound.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | The deleted test's `("WithDecimal", "m.d")` case (a bounded Decimal field key) was not carried into `a_fifo_key_stable_key_is_written`; every other field-key family (`m.b`, `m.q`, `m.r`, `m.t`) is there | fsl-package/src/emit/tests/protocol_graph.rs:1864 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | f6444f059b restores `("Int[0, 99]", "m")`; f6444f059b and c4029cf8ce add a bounded-integer model value type `M::PartId` keyed by `m` |
| FND-002 | fixed | both key tests now trace `"TC-2503", "TC-2504", "FR-880-AC-10", "FR-881-AC-2", "FSpec-FR-726-AC-2"` |
| FND-003 | fixed | the variant and its `tracking` arm are deleted from refusal.rs; there are no references left in `*.rs` |
| FND-004 | fixed | the `Quantity<m>` FIFO skip is deleted, so the FIFO case runs |
| FND-005 | fixed | `Option<Quantity<m>>`, `Sequence<Int[0, 5]>` and `("WithQuantity", "m")` are added |
| FND-006 | fixed | one `reaches_leaf(root, leaf, members)` walker now backs `reaches_event`, `reaches_float` and `holds_float` |
