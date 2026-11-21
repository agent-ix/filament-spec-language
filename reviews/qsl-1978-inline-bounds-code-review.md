---
id: SR-2026112117451800000000001
title: "code-review of PR 3355 inline-bound Shop inventory correction (QSL-1978)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #3355 at a6a5eb2da723f1478bfbec1490f0f802f1fc11d7 over main 70427772a1be1cc4816788f1688a2db1f02cdb33; spec/functional/FR-710-author-a-domain-package-in-an-fsl-model-unit.md (AC-1), spec/test-cases/TC-1942-compile-a-model-unit-to-an-admitted-domain-package.md (step 1, expected results)"
review_set: subset
relationships:
  - { target: ix://agent-ix/filament-spec-language/FR-710, type: references }
  - { target: ix://agent-ix/filament-spec-language/TC-1942, type: references }
---

## Summary

Ticket: QSL-1978. PR filament-spec-language#3355 is a Markdown-only diff of two files,
18 insertions and 2 deletions. It has no Rust, so the rust-review lane does not apply. It has
no production code, so gap-analysis does not apply. The diff was read as authored test intent
against main's producer in `fsl-semantics/src/model/intake/model_unit.rs`, read-only. That
producer sets `has_value_type` for named value types (lines 345 and 871). The ticket says its
inline field path falls back to plain native Integer. Nothing was built or run.

## Verdict

PASS. TC-1942 step 1 checks each value FR-710-AC-1 pins and fails on the producer defect
the ticket names. A plain `ix://filament/native/Integer` `typeRef` fails "Each field names its
own exact definition". A two-entry inventory fails "exactly three used kinds". Swapped
references fail the per-field `OrderId`/`OrderTotal` check. Missing or wrong `min`/`max`
operands fail the exact `"0"`/`"999"` and `"0"`/`"100"` check. The admission, invariant and
repeat-compile byte-equality checks are kept. The TC's operand strings and `min`/`max`
keywords match FSpec FR-154's scalar-and-constraints bounded Integer form and FCD's
constraint operand encoding. The spec-consistency gap, that FR-710's statement does not own
the generated identity, is recorded once in the integrity review (FND-001 there) and not
repeated here.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
