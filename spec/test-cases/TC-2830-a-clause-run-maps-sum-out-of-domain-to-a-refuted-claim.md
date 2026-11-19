---
id: TC-2830
title: "A clause run maps sum-out-of-domain to a refuted claim and refuses an unproved sum at check"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-109
    type: verifies
---
# TC-2830: A clause run maps sum-out-of-domain to a refuted claim and refuses an unproved sum at check

## Description

Verify that `run_clause`'s outcome mapping is total over
`sum-out-of-domain`, that the cause is never null, and that a clause run
never reaches the outcome because `Linked` checking refuses the `sum`.
Scope: FR-109-AC-14.

Catches a mapping that reports category `undefined` or a null cause for a
claim, and a clause run that evaluates an unproved `sum`.

## Test Procedure

1. Build `filament_eval::Evaluation` with the outcome
   `Outcome::Undefined(Undefined::SumOutOfDomain)`, map it through FR-100's
   `convert_outcome` and wrap it as the clause disposition, as TC-468 step 5
   does for the outcomes FR-100-AC-9 constructs.
2. Run the step 1 unit with FR-109-AC-14's clause `S` (`sum<Int[1, 9]>`)
   added, selecting `S`, over healthy-parent.
3. Run the same unit with `S` written over `sum<Int[0, 9]>`.

Tag the tests `#[trace("TC-2830", "FR-109-AC-14")]`.

## Expected Results

- Step 1: stage `evaluate`, category `violation`, `truth` false,
  `UndefinedEvaluation{cause: sum-out-of-domain}`, exit 10; the cause is
  present.
- Step 2: stage `compile`, `undefined_expression`/`unproved-range` at the
  `sum`, exit 20, no admission and no evaluation.
- Step 3: stage `evaluate`, `success`, `truth` true, exit 0.
