---
id: TC-537
title: "An undefined claim evaluation refutes in the explicit-state model checker"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-125
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-126
    type: verifies
---
# TC-537: An undefined claim evaluation refutes in the explicit-state model checker

## Description

Verify that the trace evaluator reads a claim's letter at each position the
claim reads and returns the claim undefined where an atom is undefined, and
that EN-1 settles such a claim `Violated` with an `UndefinedEvaluation`
counterexample, reported as the first refuting evidence in canonical order.

Scope: FR-125-AC-6, FR-126-AC-9.

## Test Procedure

All steps use the `Counter` subject of FR-124-AC-1 (universe `{c}`, initial
value 0) with the function
`function q using v(x: Integer): Integer pure { 6 mod x }`, whose body is
checked under `Kernel` (FR-495-AC-3). Steps 1 and 2 read the clause's
letters from the subject's production letters along its behaviour
`0, 1, 2, 3`.

1. Evaluate the letters of `always holds(q(2 - c.value) >= 0)` under
   infinite-trace.
2. Evaluate `eventually[0,1] holds(q(2 - c.value) = 0 and c.value = 1)`
   under event-position false-extension, `on origin`.
3. With no `terminal` declaration, run `check_model` on
   `always holds(q(2 - c.value) >= 0)` under infinite-trace, then on
   `always eventually holds(q(2 - c.value) = 0 and c.value = 1)`, then on
   `always (holds(c.value <= 0) and holds(q(2 - c.value) >= 0))`, then on
   `eventually[0,1] holds(q(2 - c.value) = 0 and c.value = 1)` under
   event-position false-extension, `on origin`.
4. With a `terminal when q(3 - self.value) = 0` member, run `check_model`
   on the `DeadlockFreedom` item.

Tag the tests `#[trace("TC-537", "FR-125-AC-6")]` and
`#[trace("TC-537", "FR-126-AC-9")]`.

## Expected Results

- Step 1: positions 0 and 1 are defined; position 2 is undefined with cause
  `division-by-zero`.
- Step 2: `true`, with only positions 0 and 1 read.
- Step 3: `Violated`, prefix `0 -inc-> 1 -inc-> 2`, `kind:
  UndefinedEvaluation{where: position 2, cause: division-by-zero}`; the same
  counterexample for the TP-4 claim, not the terminal stutter lasso;
  `Violated`, `kind: Formula`, prefix `0 -inc-> 1`; `Holds{Exhaustive}`.
- Step 4: `Violated`, prefix `0 -inc-> 1 -inc-> 2 -inc-> 3`, `kind:
  UndefinedEvaluation{where: position 3, cause: division-by-zero}`.
