---
id: SR-38367
title: "spec-review of QSL-3273 refinement and abstraction-relation identity (filament-spec-language PR 3883)"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language@700527b7c5aa06ca693562ec0d21573f787bcd9e; spec/decisions/ADR-013, ADR-015, ADR-017, ADR-020; spec/functional/FR-135, FR-137, FR-147, FR-306; spec/test-cases/TC-540, TC-542, TC-552, TC-804; spec/usecase/US-031"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-135
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-306
    type: references
---

# Spec review of QSL-3273 refinement and abstraction-relation identity (filament-spec-language PR 3883)

## Summary

Ticket: QSL-3273. Head 700527b7c5aa06ca693562ec0d21573f787bcd9e. This base spec-review checks each amended criterion for a literal oracle. Consistency and contradiction findings are in the spec-integrity-analysis artifact for the same ticket (SR-21023).

Examined, each with a literal oracle: FR-135-AC-4 (equal `package` and `path`, ordinal 0; the field row `value` reads `self.value` and `self.tmpA`; no hidden field); FR-137-AC-1 (`granularity` `whole` against `each`); FR-147-AC-9 (no `abstract_internal` against `["Twice::Done"]`); FR-147-AC-15 node part (`protocol` and `dependencies` compared); FR-306-AC-2 (`objects` entry `fields` with the old and new `RustField`); TC-540 step 4, TC-542 step 1, TC-552 steps 9 and 15, TC-804 step 2; US-031-EX-1. Each node-key half can be tested at the check level (`requirement_occurrence().node()`) and at the emission level (`node_id`).

The existing tagged tests will need the code PR the ruling names. Two tests assert the old reading: fsl-semantics/tests/it/refinement_declarations.rs:1300 (the red test) and fsl-package/src/emit/tests/admission_corpus.rs:2164-2168, which asserts `assert_ne!` on the refinement `node_id` across two packages of different identities (`test/root` against `pkg`, fsl-package/src/emit/tests.rs:3221). The second test passes vacuously under the new FR-147-AC-15 but asserts the opposite. fsl-package/src/emit/tests/abstraction.rs:304 (FR-306-AC-2) already compares bodies only.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | FR-135-AC-4 ends "The two checked packages have the same package identity (ADR-013 O-02)", but its premise already fixes "a package of the same identity and version", so the clause cannot fail. TC-540 step 4's "the same package identity" and FR-306-AC-2's "the package identity ... is the same" have the same problem. Either restate it as an observable (for example, the emitted `name` members are equal and the node-key `package` components are equal) or drop it. | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md:298; spec/functional/FR-306-carry-the-abstraction-relation-in-the-checked-package.md:79 | correct-requirement-no-evidence |

## Verdict

The amended criteria are testable with literal oracles. The one low finding is a tautological clause. Mergeability is decided by SR-21023 FND-001 (high).

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Head 9a70bf8295f43d3cec02a0d0837a492da3d9b7c5: the same-package-identity clause is removed from FR-135-AC-4, TC-540 step 4, FR-306-AC-2 and TC-804 step 2. |
