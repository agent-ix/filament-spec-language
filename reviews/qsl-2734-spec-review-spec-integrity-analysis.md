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
