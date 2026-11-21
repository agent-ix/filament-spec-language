---
id: SR-202611210022442313144108508
title: "QSL-2734 PR3286 spec integrity analysis"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; PR3286 1d666e7daa977b75f02e1bdefbd89b361d0449fd; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md, spec/functional/FR-533-evaluate-a-protocol-slot-from-the-checked-package-s-own-nodes.md"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-093
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-533
    type: references
---

## Summary

Ticket: QSL-2734. Integrity review of the two edited FRs. `quire validate` (quire 0.52.0, engine 0.62.2) on both files returned exit 0 with no diagnostics. `quoin matrix` (quoin 0.36.8) lists FR-533 criteria AC-1 through AC-22 only. The bounded package-reconstruction limitation follows the arch ruling and is not reopened.

## Verdict

CONDITIONAL. The FR-093 paragraph and the FR-533 boundary section agree on the refusal code, the `what` value, the location, the absence of a value, and the preservation of nominal and affine conversions. Both name QSL-2734 reconstruction as unmet. Defects: the new AC row is detached from its table, the product and power obligations have no criterion, and AC-23 lacks a test-case reference.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A blank line between the FR-533-AC-22 and FR-533-AC-23 rows splits the AC table, so AC-23 is a headerless orphan row that quire does not extract (matrix FR-533 criteria end at AC-22). | spec/functional/FR-533-evaluate-a-protocol-slot-from-the-checked-package-s-own-nodes.md:497-498 |
| FND-002 | medium | The boundary prose requires the bare refusal for a quantity product, quotient or integer power, but FR-533-AC-23 verifies only a quotient; product and integer-power obligations have no criterion. | spec/functional/FR-533-evaluate-a-protocol-slot-from-the-checked-package-s-own-nodes.md:304-307 |
| FND-003 | low | FR-533-AC-23's verification cell is bare `Test`, while every sibling row names its test case (`Test (TC-1333)`), so AC-23 has no TC procedure. | spec/functional/FR-533-evaluate-a-protocol-slot-from-the-checked-package-s-own-nodes.md:498 |

## Scope

- FR-533-AC-23 (examined): the new bare-route boundary criterion.
- FR-533 "Dimension equivalence reconstruction boundary" prose (examined).
- FR-093 dimension-equivalence runtime-graph paragraph (examined).
- FR-093-AC-46 (context_only): unchanged source-route criterion that AC-23 cites.

## Dispositions

Round 1, fix head 4219c63abebbce1dd776d930e126e778c4bd5d96 (original head 1d666e7daa977b75f02e1bdefbd89b361d0449fd). The reason column records the evidence; after-excerpts are in the round-1 dispositions comment.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The blank line is removed. AC-23 directly follows AC-22 in one table, and the matrix extracts 23 FR-533 criteria. |
| FND-002 | fixed | The boundary now states the product, quotient and integer-power refusal and its no-unit-charge ordering. AC-23 and TC-1333 row 23 / fixture Q verify the product, the quotient and the noncanonical square, keeping same-dimension, affine and ordinary power as positives. |
| FND-003 | fixed | AC-23's verification cell is now Test (TC-1333). TC-1333 row 23 and fixture Q supply the procedure, and the TC scope reads AC-1 to AC-23. |

Round 2, final head 7eb36c927d92a4b7637c05c608d09db5e819f2b8 (delta from 4219c63abebbce1dd776d930e126e778c4bd5d96). Each round-1 fix was re-verified at this head; none of the fixed files or spec rows changed in the delta. 

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 2: still fixed at 7eb36c927d, unchanged by the delta. The blank line is removed. AC-23 directly follows AC-22 in one table, and the matrix extracts 23 FR-533 criteria. |
| FND-002 | fixed | Round 2: still fixed at 7eb36c927d, unchanged by the delta. The boundary now states the product, quotient and integer-power refusal and its no-unit-charge ordering. AC-23 and TC-1333 row 23 / fixture Q verify the product, the quotient and the noncanonical square, keeping same-dimension, affine and ordinary power as positives. |
| FND-003 | fixed | Round 2: still fixed at 7eb36c927d, unchanged by the delta. AC-23's verification cell is now Test (TC-1333). TC-1333 row 23 and fixture Q supply the procedure, and the TC scope reads AC-1 to AC-23. |
