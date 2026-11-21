---
id: SR-20261121152317
title: "code review of PR 3324 reserved slot-name refusals at the identifier"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 3324 head c50d5340c14486339e7d5cfa8931917de0855e6f; fsl-forms/src/spans.rs, fsl-forms/src/value.rs, fsl-semantics/src/check/**, fsl-semantics/tests/it/{checker_rulings,clock_places}.rs"
review_set: subset
---

## Summary

Ticket: QSL-2581. Code review with the rust-review lane. Binder name spans are recorded per node in declaration order and copied whole by subtree, cut_removing and joined, so an ordinal indexes the same token after a cut or join; fold uses ordinals 0/1 matching identifiers 0/1; case ordinals sum earlier arms' binder counts, matching the flattened span list. Model-operation parameters still bind with no site and resolve to no region (FR-103). Two medium/low test gaps and one low efficiency nit.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test exercises binder-span carriage through subtree, cut_removing, joined or binders_of_nodes; the end-to-end tests never route a binder through those paths | fsl-forms/src/spans.rs:157-160 |
| FND-002 | low | Collection count/sum (Tally), reduce and population sum/all/only binders have no identifier-region test | fsl-semantics/src/check/check/typing.rs:4221-4230 |
| FND-003 | low | Case-arm binder ordinal recomputes the prefix sum of earlier arms for every binder, quadratic in arms | fsl-semantics/src/check/check/typing.rs:3590-3602 |

## Verdict

Fix round needed for FND-001. Clean: no weakened assertion (TC-3082 quantifier test moved from contains to exact equality; clock-guard test from the guard span to the binder token, both strictly tighter); additive fsl-forms API with no inconsistent consumer (SpanNode private; ReservedSlotName built in one place; fsl-replay matches it with a wildcard); the import and line reflow in check.rs and node_key/mod.rs is what rustfmt edition 2021 emits (verified with rustfmt 1.9.0 against the file), so it repairs drift the Makefile fmt gate checks; no Python or logic-bearing shell; all new tests tagged TC-3082 / FR-488-AC-29. Clippy and builds not run (read-only review).

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | The fix commit overwrites the committed #3259 review artifacts SR-20261121044238 and SR-20261121044239 (with their dispositions) in reviews/qsl-2581-code-review.md and reviews/qsl-2581-gap-analysis.md, because this review used the same file names | reviews/qsl-2581-code-review.md:1-3 |

## Dispositions

Round 1, at head d6812eb95e611de91f01e667592488f4cc8b3d0c (fix commit d6812eb95e), read-only, no builds.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | spans::tests::binder_spans_survive_subtree_cut_and_join reads fold and case forms and asserts binder texts at the root, via binders_of_nodes, after spans.cut with the body as a leaf, at child 0 of negated, at children 0 and 1 of conjunction of two operands cut from one parenthesised body (ExpressionForm::cut, i.e. cut_removing, then joined), and at the root of subtree. |
| FND-002 | fixed | reduce accumulator/element and collection count/sum tests added in reserved_slot.rs, each asserting the exact identifier region. |
| FND-003 | fixed | CaseArms.binders_before is captured as first_binder and advanced by arm.binders.len() before the unresolved-arm continue; ordinal = first_binder + binders.len(). |
