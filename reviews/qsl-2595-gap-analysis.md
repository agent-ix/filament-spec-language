---
id: SR-2026112005160200000002
title: "QSL-2595 batch gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2595, QSL-2594, QSL-2658, QSL-2609; fsl-forms/src/probabilistic.rs, tests/it/compile_command.rs, fsl-semantics/tests/it/temporal_clock_names.rs, tests/it/refinement_source_routes.rs"
review_set: subset
---

## Summary

Planless gap check of head 0b3edd5afe56895cd96eed79a7446c1d67276d16 (PR filament-spec-language#3142) against FR-612-AC-1/2, FR-027-AC-7, FR-135 and FR-171/FR-488 stubs. Ticket: QSL-2595 with QSL-2594, QSL-2658, QSL-2609. Plan completion: not assessed. Matrix not recomputed (no builds).

## Verdict

**PASS** with one low finding. FR-027-AC-7 unknown_language now has a tagged public-compile test (code, stage, exit 20, empty stdout, span); FR-612-AC-1 is asserted with cause; the QSL-2594 and QSL-2609 stubs are un-ignored with assertions unchanged.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | A top-level hyper condition in a probability event (`event_form`) moved from an S2 refusal to the S3 predicate typer with no test; add one probabilistic case asserting unsupported_construct/expression-form. | fsl-forms/src/probabilistic.rs:419 |

## Coverage

FR-612-AC-1 (temporal_clock_names.rs:199-238), FR-027-AC-7 (compile_command.rs:1285), FR-135 (refinement_source_routes.rs:150), FR-171/FR-488 (refinement_remaining_source_routes.rs:69,129).
