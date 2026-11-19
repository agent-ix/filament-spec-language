---
id: TC-1980
title: "Temporal disposition rows match the FSpec FR-352 registry"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-267
    type: verifies
---
# TC-1980: Temporal disposition rows match the FSpec FR-352 registry

## Description

Verify that the rows FR-267 writes for the pending, finite-window and
progress verdicts carry exactly the members, category, basis and outcome
FSpec FR-352 registers for them, and that an unregistered kind refuses.

Scope: FR-267-AC-20.

## Test Procedure

1. Write the `pending`, `pending-window` (position 0, deadline 5) and
   `pending-next-event` (position 3) reports and compare each written
   `disposition` with its FSpec FR-352 row.
2. Write an `indeterminate` report, an `activation-incomplete` report at
   position 1, a `progress-limit-reached` report (`node count`, 4,
   `progress.covered_positions`) and FR-502-AC-1's watermark
   `progress-contradicted` report; compare each with its FSpec FR-352 row.
3. Change a written row's `kind` to `pending-forever` and read it.

Tag the tests `#[trace("TC-1980", "FR-267-AC-20")]`.

## Expected Results

- Steps 1 and 2: every row matches FSpec FR-352: category `pending` and basis
  `unsettled` for the three pending rows, `indeterminate`/`unavailable`,
  `incomplete`/`unavailable` with no `outcome` for `activation-incomplete`
  and `progress-limit-reached`, `refusal`/`unavailable` with the
  `invalid_runtime_input`/`invalid-value` outcome for
  `progress-contradicted`; no row has `truth`.
- Step 3: the reader refuses naming `disposition.temporal.kind`.
