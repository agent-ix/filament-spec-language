---
id: SR-7812
title: "code-review review of PR3626 cfail, ctimeout and cexpire protocol step sources"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-cst/src/grammar.rs, fsl-forms/src/refinement.rs and their two test files; QSL-2967; PR3626 3fda3d51b28439c5aec4b1fcf96002a8c5788e0f"
review_set: subset
---

## Summary

Ticket: QSL-2967. The three spellings match the standard grammar (shared-grammar.md protocol-step-source) and FR-432's step-class tables. The keyword enum, ALL, as_str and parse path are consistent at 11 entries. No other consumer matches StepClassKeyword exhaustively. A cfail, ctimeout or cexpire row reaches S3 with a keyword that no catalog entry carries and refuses as unresolved, which is the typed refusal FR-432 requires for an unmapped row. One low finding on the trace tag and one on the untested refusal.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-310-AC-2 tag on step_sources_and_node_targets_build_as_written claims the whole AC, but the test asserts only its protocol half; the seq/only half is in an untagged-for-AC test | fsl-forms/tests/it/refinement_forms.rs:268 |
| FND-002 | low | No test pins the S3 refusal the PR body states for a parsed cfail, ctimeout or cexpire row | fsl-semantics/src/check/refinement/protocol.rs:895 |

## Scope and evidence

Titan run of cargo test --locked -p fsl-cst -p fsl-forms: all green. Mutation checks recorded in the posted comment.

- FR-432 (spec/functional/protocol/FR-432-map-every-protocol-step-in-refinements.md): cfail, ctimeout or cexpire of one template takes the row left side step cfail <C>::<protocol>::<template>, step ctimeout, step cexpire.
- FR-310-AC-2 (spec/functional/FR-310-build-the-s2-form-of-a-refinement-declaration.md): RingIsQueue's items row builds its seq and only forms as expression forms, and put -> enq(self.ring, _) builds Apply with arguments [self.ring, Open]. FR-147's CasTwice builds abstract side Protocol(Spec, Twice) and concrete side Impl; a protocol step row builds a Named source of kind fork and a Node target; a bare name builds a Named source with no kind.
