---
id: TC-710
title: "An undefined timed claim evaluation refutes with a timed prefix and replays"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-235
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-237
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-239
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-243
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-245
    type: verifies
---
# TC-710: An undefined timed claim evaluation refutes with a timed prefix and replays

## Description

Verify that the zone search refutes a timed claim that evaluates undefined,
with a timed prefix of exact rational delays ending at the undefined
position on an admitted behaviour, that the counterexample replays, that
the item settles `refuted` with cause `UndefinedEvaluation`, and that the
same undefined value reached only through a time-lock does not refute.

Scope: FR-239-AC-5, FR-237-AC-5, FR-235-AC-5, FR-239-AC-11, FR-239-AC-12, FR-243-AC-9, FR-245-AC-13, FR-235-AC-9.

## Test Procedure

Fixtures: the `Ticker` model and claim of FR-239-AC-5, and its variant with
the unconditional invariant `x <= 1`.

1. Run the zone search.
2. Replay the counterexample, then the same counterexample with its last
   step removed.
3. Settle the outcome with step 2's first replay result.
4. Run the zone search over the unconditional-invariant variant.

Fixture for FR-239-AC-11, FR-239-AC-12, FR-243-AC-9, FR-245-AC-13 and
FR-235-AC-9: the `Call` model of FR-239-AC-11, whose `bad` precondition has
no value at the state `go` reaches.

5. Run the zone search, the digital-clocks route and the deadlock-freedom
   item over it, and the zone search under a weak fairness constraint on
   `go`.
6. Settle the zone refutation after replay.
7. Check a zone certificate built for the variant whose guard is defined
   against this subject.

Tag the tests `#[trace("TC-710", "<AC id>")]`.

## Expected Results

- Step 1: `Violated`, prefix `step` after delay 1 and `step` after delay 1,
  `kind: UndefinedEvaluation{where, cause: division-by-zero}`, `where`
  naming position 2, time stamp 2, the state with `n = 2`, `x = 0` and the
  expression's locus.
- Step 2: `reproduced-with-evaluated-witness`; then `inconclusive`,
  `ReplayParity`.
- Step 3: `refuted`, `decisive-counterexample`, category violation, cause
  `UndefinedEvaluation`.
- Step 4: `Undecided(NoAdmittedBehaviour)`, no counterexample.
- Step 5: `Violated` with `kind: UndefinedEvaluation` at the state with
  `phase = 1` on the zone and digital routes, and for deadlock freedom a
  kind that is never `Deadlock`; never `Holds`. Under the fairness
  constraint: `Undecided` or `Violated`, never `Holds`.
- Step 6: `refuted`, `decisive-counterexample`, cause
  `UndefinedEvaluation`.
- Step 7: rejected `BadState` at the `phase = 1` node.
