---
id: TC-839
title: "S6a evaluates an infinite-trace clause exactly over a fair lasso"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-329
    type: verifies
---
# TC-839: S6a evaluates an infinite-trace clause exactly over a fair lasso

## Description

Verify ADR-018 SM-8 lasso evaluation with past operators reaching into the
prefix, malformed-lasso refusals, fairness on a model-step lasso, and
metering.

Scope: FR-329-AC-1 to FR-329-AC-5 and FR-329-AC-8 to FR-329-AC-11 (FR-329-AC-11 planned under QSL-2568).

## Test Procedure

1. On the observed lasso (empty prefix, loop 0, 1, 2) evaluate the three
   clauses of FR-329-AC-1.
2. On the observed lasso (prefix 5, loop 0, 1, 2) evaluate the two clauses
   of FR-329-AC-2.
3. Evaluate any clause over a lasso with an empty loop, and over a
   `Lasso::Model` whose last loop post-state is not its entry state.
4. Over ADR-018 §6's `upd(a)` lasso as `Lasso::Model`, evaluate
   `always eventually holds(b.versionNumber = 2)` under `fair weak
   attemptUpdate` and under `fair weak each attemptUpdate`.
5. Evaluate step 1's first clause with a meter one unit short of its visit
   count.
6. Under a bounded profile, build through FR-217's replay a fair
   zero-counted protocol lasso (a `repeat` around a `parallel`, loop of
   `fork` and `join` steps) and evaluate `eventually[1,1] holds(p)` with `p`
   false throughout, `eventually[0,1] holds(p)` with `p` true at position 0,
   and `eventually[0,1] holds(p)` with `p` true only at a loop position.
7. Under a bounded profile, evaluate a clause over a `Lasso::Observed`, over
   a `Lasso::Model` on a model subject, and over a protocol-subject lasso
   whose loop has one counted step; replay the last through FR-217.
8. Under a bounded profile, over FR-205-AC-10's `Echo` protocol with `p`
   (`c.v = 1`) false at every position, build through FR-217's replay the
   zero-counted lasso with prefix `send(Snd)` and loop `duplicate(ch,
   delivery 0)`, `lose(ch, delivery 0)`, whose last post-state's
   `ProtocolKey` differs from its entry's and whose cycle key (FR-205) equals
   it. Evaluate `eventually[1,1] holds(p)` over it against the default
   package (the payload carrying the default package's scheduler
   constraints) and against the package that declares `scheduling
   adversarial`. Repeat both with loop closure compared by `ProtocolKey`
   (an adverse build of the closure test).
9. Planned under QSL-2568 (FR-329-AC-11): evaluate step 8's lasso, against
   both packages, for a clause that reads the delivery index of `Rcv`'s
   receive steps.

Tag the tests `#[trace("TC-839", "FR-329-AC-n")]`.

## Expected Results

- Step 1: `true`; `true`; `false` at 0.
- Step 2: `true`; `false` at 0.
- Step 3: `invalid_runtime_input`/`invalid-value` for each.
- Step 4: `false` at 0; refusal `invalid_runtime_input`/`invalid-value`
  naming the `each` constraint.
- Step 5: `Incomplete` with no truth value.
- Step 6: `false` at 0; `true`; `true`.
- Step 7: `unsupported_construct`/`expression-form` for each evaluation, with
  no truth value (the replay is FR-217-AC-7's).
- Step 8: against the default package `invalid_runtime_input`/
  `invalid-value` naming the root's scheduler constraint (`Rcv` enabled at
  every loop position, never taken), no truth value; against the adversarial
  package `Completed(false)` at 0. The step fails if the closure test compares
  `ProtocolKey`s: both evaluations then refuse as a loop that does not
  close, naming the loop's last step, no constraint and giving no truth
  value, since closure is checked before fairness.
- Step 9 (planned, QSL-2568): both refuse `invalid_runtime_input`/
  `invalid-value` as a loop that does not close, naming the loop's last
  step and no constraint.
