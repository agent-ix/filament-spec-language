---
id: TC-2211
title: "The remainder walk is bounded by the checking work ceiling"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-495
    type: verifies
  - target: ix://agent-ix/filament-spec-language/NFR-011
    type: verifies
  - target: ix://agent-ix/filament-specification/TC-1660
    type: references
---
# TC-2211: The remainder walk is bounded by the checking work ceiling

## Description

Verify that the exact remainder interval of a large prime dividend over a
wide divisor range is found under the default checking work ceiling, and
that a ceiling below the walk's cost stops checking with the work stage
limit instead of widening the type. Scope: FR-495-AC-20.

## Test Procedure

1. Under `CheckMode::Linked` and `CheckingLimits::default()`, check `big` of
   FR-495-AC-20 into `Int[1, 500000003]` and into `Int[2, 500000003]`.
2. Check `big` into `Int[1, 500000003]` under
   `CheckingLimits::default().with_work_budget(10000)`.

Test: `fsl-semantics/tests/it/division_law_selection.rs`
`the_remainder_walk_is_bounded_by_the_checking_work_ceiling`, tagged
`#[trace("TC-2211", "FR-495-AC-20", "FSpec-TC-1660", "FSpec-FR-147-AC-14")]`.

## Expected Results

- Step 1: the first checks; the second refuses `unproved-range` at the `mod`.
- Step 2: one refusal, `stage_limit_exceeded` with kind `WorkBudget` and
  bound 10000, located at the `mod`; no `unproved-range` refusal.
