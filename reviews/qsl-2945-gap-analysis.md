---
id: SR-20261123010002
title: "Gap analysis of filament-spec-language PR #3575: nested reference lifting"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@af2554ebd81b6632cec71c046914f1ef2c2a102e; PR #3575 diff: FR-135-AC-6, FR-140-AC-16, FR-630-AC-4, TC-540 step 6, TC-1086 step 10, TC-1590 step 4 and their tagged tests"
review_set: subset
---
# Gap analysis of filament-spec-language PR #3575

## Summary

Ticket: QSL-2945. Plan completion: not assessed. The matrix check was done by
hand: AC to `#[trace]` tags in the PR diff. `quire trace --id` did not finish
in 120 s on this repo and was stopped.

Bindings checked:

- `a_row_nesting_a_reference_is_lifted_element_by_element` (TC-540,
  FR-135-AC-6): sequence, option, population and plain rows check, and the
  `Slot` row refuses `ill_typed`/`type-mismatch` at `self.cargo`. Correct.
- `a_sequence_of_references_maps_element_by_element_in_order` (TC-1086,
  FR-140-AC-16): the order `[b, a]` is kept, and `SubRing` gives `Undetermined`
  with `UnmappedType(SubRing)` at the `chain` row of object `a`. Correct.
- `a_nested_reference_row_checks_and_proves` (TC-1590, FR-630-AC-4 and
  FR-135-AC-6): it is no longer ignored, the aggregate is complete, and the item
  is `proved` and `certified`. The fixture is Counter to Counter, not Ring to
  Queue as the AC words it.

The composite refusal tests (box, tuple, union) were deleted. Measured state of
the code: the S3 `lift()` leaves `Composite` unchanged, and `conforms` compares
composites with `composite_conforms`, so a composite row that nests a reference
now refuses `ill_typed`/`type-mismatch` (found by reading the code, not by a
test run). FR-135 (lines 175-180, 249-253) and FR-630 (lines 56-61) still say
that such a row is lifted. No test pins either behaviour, and no follow-up
ticket for composite lifting exists in Linear. Searching QSL for "nested
reference" and "composite" titles found only QSL-2945 and closed tickets.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The composite nested-reference rows (box, tuple, union) lost their only tests. Today the code refuses them with type-mismatch, while FR-135 and FR-630 specify them as lifted. That gap is silent: no ignored test cites a follow-up, and there is no follow-up ticket. File the follow-up, and add an `#[ignore = "QSL-<follow-up>: composite nested-reference lifting"]` test that asserts the specified lifted outcome for the box, tuple and union rows. | fsl-semantics/tests/it/refinement_declarations.rs:1146 |
| FND-002 | medium | The FR-135-AC-6 clause "a nested option or collection at any depth" has no test. The S3 test only covers one level, and so does the replay test (see SR-20261123010001 FND-002). | fsl-semantics/tests/it/refinement_declarations.rs:1146 |
| FND-003 | medium | The Option and Map arms of replay `lift_nested` have no test: `some`/`none` payload lifting, map value lifting with keys and order kept, and an unmapped element inside an option or map giving `UnmappedType`. The same gap as SR-20261123010001 FND-001: one fix closes both. | fsl-replay/tests/it/refinement_unmapped.rs:118 |
| FND-004 | low | FR-630-AC-4 and TC-1590 step 4 name `Sequence<Reference<Ring>>` mapped to `Sequence<Reference<Queue>>`, but the tagged test uses a Counter to Counter `chain`. The shape matches, but the names in the AC do not. | fsl-analyze/tests/it/analyze_family_cli.rs:397 |

## Verdict

The new ACs FR-140-AC-16 and the rewritten FR-135-AC-6 and FR-630-AC-4 each
have a tagged test that asserts exact codes and values. The gaps are FND-001
(composite tests deleted with no pending test and no follow-up), FND-002
(depth) and FND-003 (the Option and Map arms).
