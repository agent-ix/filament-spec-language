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

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | The symbolic field, parameter and result routes have no test. By reading the code, reverting any of the three `domain_with_clocks` calls back to `Domain::of` still passes every test, whereas reverting `domain_values` or `complete_open_fields` fails `clock_reference_routes_range_over_the_declared_identities`. The routes in CR FND-008 and FND-009 are untested as well. | fsl-eval/src/simulation/model/symbolic.rs:280, fsl-eval/src/simulation/model/symbolic.rs:404, fsl-eval/src/simulation/model/symbolic.rs:422 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `clock_reference_candidates_follow_the_selector_maxima_and_keep_seeds` asserts exact counts under maxima 2 and 3, selectors 0..2, the scalar oracles, and a q[7] seed placed last without repetition. `clock_reference_routes_range_over_the_declared_identities` asserts the exact x and y place identities on two routes. Both pass. |
| FND-002 | fixed | Five Source tests cover invalid selector, unbounded selector, scope mismatch at the nested locus, unlisted owner (complete and incomplete) and removed place at the nested locus. All pass on titan at 0b4a656a. |
| FND-003 | fixed | New tests fail on reverted code (mutation run): `a_protocol_event_record_ranges_a_clock_reference_over_the_declared_places` (values_of 0 events vs 2; enumerates), `a_model_based_parameter_holding_a_clock_reference_is_refused`, `symbolic_subjects_refuse_an_unbounded_clock_reference_domain_by_name` (parameter and result routes; the reverted code encodes an empty-domain script). The symbolic field route (symbolic.rs:280) has no entry that observes it: every caller (query, relation_query, refinement, hyper) runs `StateLayout::of` right after `symbolic()`, which refuses any Reference, Option, collection or record field with `field-sort` first. |
