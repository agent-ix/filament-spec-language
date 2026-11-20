---
id: SR-2026112005160200000003
title: "QSL-2595 batch spec-review integrity"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; QSL-2595; spec/functional/FR-763-name-the-construct-in-every-forms-stage-refusal.md, spec/test-cases/TC-1543-hyper-only-forms-are-refused-in-a-single-trace-clause.md, spec/test-cases/TC-2159-every-construct-message-begins-with-the-construct.md, spec/test-cases/TC-2160-the-parse-document-renders-the-construct-cause.md, spec/test-cases/TC-396-s2-refuses-unrepresented-and-foreign-constructs.md"
review_set: subset
---

## Summary

Integrity pass over the spec edits in PR filament-spec-language#3142 head 0b3edd5afe56895cd96eed79a7446c1d67276d16, implementing the QSL-2595 arch ruling (FR-612 wins over FR-763's hyper-condition row). Ticket: QSL-2595.

## Verdict

**PASS** with one medium finding. The FR-763 row, bullet, AC-5 removal, AC-6 edit, TC-2159 deletion, TC-396 step 4 and TC-2160 note removal and TC-1543 extension are mutually consistent. `git grep 'FR-763-AC-5\|TC-2159\|hyper-condition' HEAD` finds only historical review artifacts, nothing live.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-763 "The payload" still requires `FormsCause::UnrepresentedConstruct` to carry the construct payload and requires 120-byte `text` truncation, but no producer of that cause remains and, with AC-5 gone, no AC verifies the payload, the message prefix or the truncation; the only remaining row (`declaration`) is stated unreachable from source. Drop UnrepresentedConstruct from the statement and add a unit-level AC for the declaration-row payload and truncation, or state those clauses are verified by inspection. | spec/functional/FR-763-name-the-construct-in-every-forms-stage-refusal.md:51-60 |

## Coverage

FR-763 statement, construct table, AC-1..AC-6; TC-1543, TC-2160, TC-396 scope and tag lines.
