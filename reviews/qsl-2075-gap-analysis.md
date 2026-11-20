---
id: SR-1470
title: "Review value declaration acceptance coverage"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2075; codex/qsl-2075-value-near-misses at 2eee6ee41514465cf35481a71dd3eef8db56b46e; fsl-cst/tests/it/value_declarations.rs and its registration"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-002
    type: references
  - target: ix://agent-ix/filament-specification/FR-339
    type: references
---

## Summary

Diff-scoped acceptance and reverse-traceability review of QSL-2075's original value-declaration corpus. Each of the six assigned families has a valid control and at least four meaningful nonlexical refusal cases. No gap was found within this feature.

## Verdict

**PASS** for the assigned subset. No repository-wide matrix, completion, ignored-criterion coverage or final-gate claim is made.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Plan completion: not assessed

| Family | Active original cases | Coverage |
| --- | --- | --- |
| Dimension equivalence | 1 valid, 5 malformed | Wrong `by`, non-exact ratio, missing unit, missing `=`, missing terminator |
| Unit | 1 valid, 5 malformed | Plain ratio, missing dimension separator, numeric dimension, missing `=`, missing terminator |
| Enum | 1 valid, 4 malformed | Empty body, missing comma, misplaced `ordered`, unclosed body; admitted trailing comma remains valid |
| Record | 1 valid, 5 malformed | Empty body, missing type, wrong field separator, missing field terminator, unclosed body |
| Tuple | 1 valid, 4 malformed | Empty tuple, trailing comma, wrong element separator, missing terminator |
| Alias | 1 valid, 5 malformed | Missing `=`, missing type, missing name, second `=`, missing terminator |

All 34 cases invoke the real public parser. No lexical `#` mutation, mock, success-only result check, ignored case or copied upstream fixture is introduced. Negative helper assertions establish a real refusal, exact typed cause and source byte locus, source identity/path and lossless retention. Accepted helpers assert the intended production rather than accepting an unrelated admissible parse. Tags bind valid controls to FSpec FR-339-AC-1/2 and malformed controls to FSpec FR-339-AC-1 plus FSL FR-002-AC-6.

## Examined criteria

| Criterion | Role | Excerpt |
| --- | --- | --- |
| FSpec FR-339-AC-1 | examined | Every production in the base and complete-V1 facet grammar has positive, boundary and located-negative corpus vectors. |
| FSpec FR-339-AC-2 | examined | Parsing and lossless rendering reproduce accepted UTF-8 source bytes exactly, including comments and whitespace. |
| FSL FR-002-AC-6 | examined | The bounded malformed/truncation corpus returns a `ParsedSource` or a located diagnostic from `fsl_cst::parse` without panic; any admissible variant retains its exact submitted bytes. |

## Scope limits and evidence

The user's instructions restrict this analysis to the new feature diff and prohibit another matrix computation, inventory or remeasurement of accepted peer results. Accordingly, this is a manual criterion-to-test, reverse-ownership and hollow-evidence check of the assigned subset; broader repository assurance remains unassessed. No matrix or existing corpus audit was performed or claimed. The supplied coder results cover the 28 negative and six valid cases and scoped Clippy; no execution was repeated here. Final applicable component-gate custody remains with the lead and original coder.

The grammar authority is pinned FSpec `f32bca6c7937e0652742b2bbe27a52419bf5a0ae`, `proposals/filament-v1/shared-grammar.md:410` through line 424, incorporated by `spec/functional/source-package/FR-339-define-complete-native-grammar.md`. The language diagnostic/lossless criterion is `spec/functional/FR-002-parse-native-units.md`, FR-002-AC-6.
