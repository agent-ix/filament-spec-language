---
id: TC-3023
title: "A tl_lowering request cross-evaluates an infinite-trace clause over a lasso"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-931
    type: verifies
---
# TC-3023: A tl_lowering request cross-evaluates an infinite-trace clause over a lasso

## Description

Verify the `mltl.infinite-trace/v1` correspondence of `tl_lowering`, its
lasso cross-evaluation and the refusals of a trace form that does not match
the correspondence. Scope: FR-931-AC-9 and FR-931-AC-10.

## Test Procedure

1. Lower the infinite-trace clause `C` (`on each { once[1,1] not holds(c.level = 0) }`) with no correspondence and with `mltl.infinite-trace/v1`; then with `cross_evaluate` over the lasso with no prefix and loop `0`.
2. Lower `C` naming `mltl.origin-complete-history/v1`; lower the event-position clause `historically [0, 2] holds(c.level <= 2)` naming `mltl.infinite-trace/v1`; lower the infinite-trace clauses `once holds(c.level = 0)` and `eventually[0,2] holds(c.level = 0)`.
3. Send a lasso beside `mltl.closed-trace/v1`; a finite trace and the `initial` form beside `mltl.infinite-trace/v1`; a finite trace for `C` with no correspondence; a lasso with an empty loop.
4. Lower `C` with a weak fairness constraint over an observed lasso.
5. Through the `fsl-command` library entry, cross-evaluate `C` over step 1's lasso with the faithful atom table and with `p0` mapped to the negated leaf.

Tests: `tests/it/mltl_differential.rs` `an_infinite_trace_past_clause_lowers_to_the_infinite_trace_correspondence`, `a_trace_form_must_match_the_correspondence` and `a_mis_mapped_atom_over_a_lasso_is_a_correspondence_loss`.

## Expected Results

- Step 1: the two records are equal, `supported`, `correspondence` `mltl.infinite-trace/v1`, `classification` `{"correspondence": "mltl.infinite-trace/v1", "fragment": "bounded-past"}`, exit 0; the cross-evaluation has `traces` 1, `positions` FR-329's unrolled count, `agree` true, `faults` `[]`.
- Step 2: `unsupported`, `classified` the other target, `dimensions` `["profile"]`, exit 21 for the first two; `unsupported`, `dimensions` `["infinite-trace"]`, no `classified`, exit 21 for the last two.
- Step 3: `invalid-request` at stage `request`, exit 20, naming `tl_lowering.cross_evaluate.trace` (`tl_lowering.cross_evaluate.initial` for the `initial` form), no result document; the empty loop refuses `invalid_runtime_input`/`invalid-value`.
- Step 4: `MissingFairnessPremise`, exit 21, no `cross_evaluation`.
- Step 5: faithful: exit 0, `agree` true; mis-mapped: exactly one `correspondence-loss` fault naming `C`, trace 0, position 1, native `false`, `tl` `true`, exit 30.
