---
id: SR-967175026302197768900474181941935084809
title: "Gap analysis — QSL-2959 composite nested-reference lift"
type: SpecReview
analysis: gap-analysis
scope: "Diff-scoped acceptance review for FR-106-AC-84, FR-135-AC-13..19, FR-140-AC-19..22, and FR-630-AC-7..10"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language", type: references }
---

## Summary

The computed FSL matrix and changed tests were checked for QSL-2959's composite rows. Three relevant criteria remain untagged: FR-106-AC-84 and FR-630-AC-8/9; the full-repository quoin evidence matrix refused its request-size limit, so this is a diff-scoped result and plan completion was not assessed.

## Verdict

**FAIL** — live acceptance criteria in the reviewed scope lack tagged source tests.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-106-AC-84 has no tagged test. The added refinement fixtures declare tuple and union types and construct model states, but do not exercise TC-464 step 11's exact snapshot `tuple` and payload `variant` carriers, preservation of supplied reference values, or malformed tuple/Has arities and unknown cases. Add a snapshot admission test with literal expected values and refusals tagged `FR-106-AC-84`. | FR-106-AC-84; TC-464 step 11; fsl-semantics/tests/it/refinement_declarations.rs:1343 | correct-requirement-no-evidence |
| FND-002 | medium | FR-630-AC-8 and AC-9 are untagged. The analyzer test at line 729 loops only over `Box` and `Parcel`; Pair and Holder are exercised by checker/map tests, not by the analyzer item and certificate that these criteria require. Add separate Pair and Holder item tests asserting `proved`, `certified`, the resulting values, and the owning AC tags. | FR-630-AC-8; FR-630-AC-9; fsl-analyze/tests/it/refinement_replay.rs:729-747 | correct-requirement-no-evidence |

## Coverage

- Reconciliation: `quire matrix` 0.52.1 (engine 0.62.2), full-repository TSV filtered to the reviewed criteria; `quoin matrix` 0.36.10 refused with `CORE_REFUSED: request exceeds the accepted size`; no run evidence read.
- Plan completion: not assessed
- Criteria: tagged 13 · untagged 3 · tagged-by-ignored-test 0 · method-without-symbol 0
- Evidence: no run evidence
- Untraced behaviors / stubs: no stub found in the changed lines; FND-001 in the code-review artifact identifies a conformance check bypass under an existing requirement.
- Semantic review: skipped

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | QSL-3263 (blocks QSL-2959). The snapshot reader has no tuple carrier: `fsl-semantics/src/model/observation/tree.rs` has no `tuple` arm on origin/main d041ae422a. It also has no payload-union carrier: `admit_union_case` in document.rs admits only enum (nullary) cases. So FR-106-AC-84 needs new admission code, not only a test. The FR-106 spelling conflict is already fixed on main by #3876 (`components`). Checked at head a2d980afb9a2f31f5e3a60c43986f0a1c1d43c56. |
| FND-002 | deferred | QSL-3263 (blocks QSL-2959). The FR-630-AC-8/9 analyzer certificates for Pair and Holder need the same snapshot tuple and payload-union carriers, which do not exist on origin/main d041ae422a. QSL-2959 stays open until then. The PR body defers these and claims neither criterion. |
