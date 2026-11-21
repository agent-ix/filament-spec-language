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
