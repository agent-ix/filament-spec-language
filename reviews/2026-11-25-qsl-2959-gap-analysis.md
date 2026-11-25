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
