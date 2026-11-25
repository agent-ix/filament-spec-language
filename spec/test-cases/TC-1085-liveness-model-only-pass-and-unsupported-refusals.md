---
id: TC-1085
title: "The liveness engine reads positions past a dead automaton and refuses unbuilt clauses"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-126
    type: verifies
---
# TC-1085: The liveness engine reads positions past a dead automaton and refuses unbuilt clauses

## Description

Verify that a TP-4 item whose negation automaton has no move still has the
letters and contracts of the positions past it read, that the read is charged
to the run's budgets and joins no product total, and that the
unbuilt-certificate case refuses.

Scope: FR-126-AC-12 to FR-126-AC-15.

## Test Procedure

Fixtures: ADR-018 §6's ConfigVersion example unit, universe `{a, b}`, both
versions 0, the subject's `max_candidates` 1,000,000.

1. `eventually (holds(c.versionNumber = 0) and
   holds(mod_defined(c.versionNumber)))`, with
   `function mod_defined using v(n: Int[0, 2]): Boolean pure
   { 6 mod (2 - n) >= 0 }`, its body checked under `Kernel` (FR-495-AC-3),
   and the default limits.
2. The same claim with `max_states` 1.
3. AC-1's weak `each` claim, with and without a built FR-339 certificate.
4. Step 1's claim with initial versions `a` 0 and `b` 1.
5. Step 4's request with `max_states` 6, with `max_states` 1,000, and with
   `max_states` 1; step 1's request with `max_states` 1,000.

## Expected Results

- Step 1: `Violated` bound to `a`, `kind: UndefinedEvaluation` with cause
  division-by-zero at position 2, the prefix `(0,0) -upd(a)-> (1,0) -upd(a)->
  (2,0)`; no product state is reported for the dead positions.
- Step 2: `Stopped{ResourceExhausted, {MaxStates, 1}}`.
- Step 3: `Holds{Exhaustive}` with a component certificate that FR-339's
  checker accepts.
- Step 4: `Violated` bound to `a` with the prefix
  `(0,1) -upd(a)-> (1,1) -upd(a)-> (2,1)` at position 2, even though binding
  `b` has a shallower finding.
- Step 5: the AC-14 finding for `a` at both `max_states` 6 and 1,000;
  `Stopped{ResourceExhausted, {MaxStates, 1}}` with no counterexample when
  the first binding is incomplete; the step 1 finding for `a`.
