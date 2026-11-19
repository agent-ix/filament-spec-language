---
id: SR-2026111612172400000012
title: "Gap analysis — QSL-2328 affected evaluator paths"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; affected past-operator and integer-coercion criteria, changed Rust paths, and the three named tests"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-327", type: references }
  - { target: "ix://agent-ix/filament-spec-language/FR-932", type: references }
  - { target: "ix://agent-ix/filament-spec-language/FR-495", type: references }
  - { target: "ix://agent-ix/filament-spec-language/FR-533", type: references }
---

## Summary

Scoped gap analysis of QSL-2328's past-initialization and Integer-coercion paths found no missing requirement-to-test-to-code link in the directly applicable criteria examined. The required matrix command ran repository-wide; this artifact's verdict is limited to the affected paths and does not claim repository-wide assurance or aggregate test completion.

## Verdict

**PASS** — the relevant criteria examined have traceable tests and corresponding implementation paths; no scoped reverse gap or stub was found. The PR reports the three named tests passing 1/1; those runs were not repeated here.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No gaps found (placeholder) | - |

## Coverage

- Reconciliation: `quoin matrix` (quoin 0.29.4, quire 0.38.3; whole-repository matrix).
- Whole-repository matrix counts: tagged 3,853; untagged 1,161; tagged-by-ignored-test 390; method-without-symbol 103; retired 13. All 5,520 criteria show `no run evidence` in the quoin evidence axis; no run-evidence claim is made.
- Scoped criteria checked: FR-327-AC-14 and FR-932-AC-1..3 (past-origin behavior), FR-495-AC-21 (division narrowing and lowered type), FR-533-AC-18 (protocol checked-node behavior). The named removal-filter test is tagged FSpec-FR-146-AC-15; that external FSpec criterion is outside this repository's spec root.
- Reverse-gap inventory (scoped): three changed production behavior units (`Past::empty`, `Translator::initial`, `Typer::coerce`); untraced behaviors 0, source stubs 0; three named tests inspected, test stubs 0.
- Plan completion: not assessed
- Semantic review: ran over the affected past and coercion behavior using the stated contracts and source paths.
