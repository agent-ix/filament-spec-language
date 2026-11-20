---
id: SR-20261120073951
title: "gap-analysis of filament-spec-language PR 3157 against FR-531, FR-322, FR-264, FR-740, FR-185"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR 3157 head e21589a2f285fef2f3db2c688c3094b86f7ccb92; FR-531-AC-36..39, FR-264-AC-1/6, FR-740-AC-8, FR-185-AC-12, FR-094-AC-17; tests"
review_set: subset
---

## Summary

Ticket: QSL-2625 (also QSL-2648, QSL-2652). Planless. Plan completion: not assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-531-AC-39 requires rows for the precondition clause function; the producer emits none and the AC-39 test asserts zero rows | tests/it/dispatch_emission.rs:711-734 |
| FND-002 | medium | FR-264-AC-6 and QSL-1985 test now raises the limit for a different package (2,000 not 20,000 levels), and the linear-bytes helper strips body_locations | fsl-package/src/emit/tests/output_limit.rs:82-90 |
| FND-003 | medium | FR-185-AC-12 random-parameter refusal case removed though random parameters can still be refused; enum and Boolean tests tagged FR-740-AC-8, which describes the rover example | fsl-package/src/emit/tests/probabilistic.rs:761 |
| FND-004 | medium | Body row name carries an ix://source/ check-time alias; the test blanks body_locations to keep passing | fsl-package/src/emit/tests/source_objects.rs:1114-1117 |
| FND-005 | medium | No test reads every emitted package body shape back through the strict reader; only the full suite covers it, and it was not run | fsl-package/src/emit/body_locations.rs:47 |
| FND-006 | low | dispatch_emission dropped the graph-equality assertion for the carrier-less read | tests/it/dispatch_emission.rs:452-465 |
| FND-007 | low | Stale doc comment and test name say the sqrt test is ignored | fsl-eval/tests/it/ieee_ordering_route_parity.rs:205-212 |
| FND-008 | low | IR-1569 ignore reasons are stale now that Boolean supports emit | fsl-eval/src/simulation/model/probabilistic/tests.rs:667 |

## Verdict

Blocking on FND-001. AC-36, AC-37, AC-38 have tests (two un-ignored, three added). FR-094-AC-17 test is guarded by a non-zero declaration count.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | low | Boolean and enum random-parameter tests retagged FR-740-AC-1, which states the `Service` fixture (d and outcome on Server::attempt); neither test checks it | fsl-package/src/emit/tests/probabilistic.rs:761 |
| FND-010 | low | AC-39 test does not assert that the rows' `function` is a node with a `clause` binding `precondition` | tests/it/dispatch_emission.rs:719-753 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
| FND-007 | fixed | - |
| FND-008 | fixed | - |
