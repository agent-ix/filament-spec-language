---
id: TC-2862
title: "Plans keep policies, exact costs and the canonical order"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1012
    type: verifies
---
# TC-2862: Plans keep policies, exact costs and the canonical order

## Description

Verify non-safety policies, deterministic outcomes, exact decimal costs, timed and probabilistic plans and the pinned exploration counts. Scope: FR-1012-AC-1 to AC-7.

## Test Procedure

1. Plan under `eventually holds(c.versionNumber = 3)`.
2. Run one request twice.
3. Minimize a decimal per-step cost.
4. Plan over FSpec's `timed-tick` and `timed-go` fixtures, the latter with guard `x >= 2` and with `x > 2`, twice.
5. Plan over `probabilistic-try` for `Reach.final.done` and for `maximize Reach.probability`.
6. Plan over `counter` with `max_states` 2 and with `max_work` 1, each twice.
7. Plan over a counter whose invariant is undetermined from value 4 up, for goal 8 and for goal 2, and through a substituted plan crossing that state.

## Expected Results

- Step 1: not refused; every plan visits version 3.
- Step 2: byte-identical documents.
- Step 3: the exact run value, `potential` certificate.
- Step 4: `elapsed` `30 min`, delays 0; `optimal` delay `2/1` with an accepted `closure`; `feasible`, `bound` lower `2/1` unattained, delay `5/2`, byte-identical.
- Step 5: `optimal` `retry`; `optimal` `try, try` at `3/4`.
- Step 6: `explored` 2 states, 6 work and 1 state, 1 work, plan `null`, byte-identical.
- Step 7: `incomplete` naming the invariant with no plan and no certificate; `incomplete` carrying the plan found before the state; no plan through the state is reported.
