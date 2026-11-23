---
id: SR-2897002
title: "QSL-2897 part 1 gap analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2897; PR #3555 diff vs merge base 7ca2bd11ab9216e66dd595cf66878569d4032dfd; FR-106-AC-72, AC-73, AC-76, AC-78, AC-80, AC-81, FR-1040-AC-8"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-106", type: references }
  - { target: "ix://agent-ix/filament-spec-language/FR-1040", type: references }
---

## Summary

Ticket: QSL-2897. Planless gap analysis of the PR diff. Plan completion: not assessed. `quoin matrix --json` (quoin 0.36.8) refused with `native runtime matrix.build failed: CORE_REFUSED: request exceeds the accepted size`, so the computed matrix is unavailable for this repository. The criterion-to-test check below was done by hand from `#[trace]` tags and focused runs on titan. It is not a full matrix audit; the reverse-gap sweep was limited to the diff.

Focused runs on titan at head 56853164ff:

- `fsl-eval --test it clock_reference`: 4 passed, 6 ignored.
- The same filter with `--ignored`: all six fail, each at its Bare call, after its Source half passes.
- `fsl-eval --lib clock_reference_domains`: 2 passed.
- `fsl-analyze --test it clock_reference_selectors_use_the_timed_profile`: 1 passed. This is the AC-78 / FR-1040-AC-8 test, now un-ignored.
- `fsl-semantics --lib model::observation`: 38 passed, 1 failed. The failure is `collection_elements::native_collection_decimal_retains_ordered_spelling_and_checks_value_membership`, which fails the same way on origin/main 27044cf5 (it expects `Decimal[0, 10000; 2, 2]` and gets `...; exact]`), so this PR did not cause it. The field-locus change broke no other observation test in this filter.

No stub or placeholder code was found in the diff. The six ignores cite QSL-2776, and none claims a Bare-route criterion as met.

## Verdict

**CONDITIONAL.** No running test covers the new candidate-generation paths beyond scalar places, so add tests for FND-001 and FND-002.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No running test exercises bounded-collection selector enumeration (`selector_ranges` and `product`), timed-derived selector maxima in candidates, or the declared-then-seed ordering with a seed present. The domain unit tests pass hand-supplied identities, and `clock_reference_enumeration_on_the_source_route` has only scalar places `x` and `y`, although its doc says "a bounded declared clock slot is a finite domain". It asserts a count and distinct paths, not the exact identities equal to `ref(self.x)` and `ref(self.y)`. | fsl-eval/tests/it/timed_subject.rs:1405, fsl-eval/src/simulation/model/clock_domain.rs:144 |
| FND-002 | medium | Source-side refusal branches of `resolve_clock` are untested by any running test: selector invalid for the template (AC-80), scope mismatch on raw admission, owner absent from a complete population, an unlisted owner in an incomplete population, and the nested `maybe` / `refs[0]` locus that AC-73 names. Only scalar `r` with a declared, undeclared-name and `place/v1` target runs. | fsl-eval/tests/it/clock_reference_v2.rs:1284, fsl-semantics/src/model/observation/document.rs:1439 |
