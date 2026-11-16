---
id: SR-2026111605245700000002
title: "Code review — QSL-2151/2152/2153/2157 source checks"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-semantics/, fsl-package/, fsl-replay/, fsl-analyze/; FR-186, FR-195"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-186", type: reviews }
  - { target: "ix://agent-ix/filament-spec-language/FR-195", type: reviews }
---

## Summary

Reviewed PR #2740 at frozen head `7e9d318e56b65eea7f25e38031f135eabb7611f7`, including the QSL-2151/2152/2153/2157 acceptance criteria and their in-repository FR/TC coverage. One test-suite finding remains: two source-analysis tests are enabled although source routing is deferred to QSL-2158 and both tests fail before reaching the changed checker.

## Verdict

**CONDITIONAL** — the in-repository checker changes match the scoped criteria, but the newly active analyzer tests fail on this head.

## Analysis

Rust review covered the changed checker, package assertions, replay diagnostic mapping, analyzer mapping, and integration tests. The changed public `ClaimActivation` type has a cross-crate consumer in `fsl-package`; the timed data is retained in checked forms and included in normal-form encoding. No CI workflow, vendored material, unsafe code, or changed error wire contract was found. `git diff --check` passed. The dispatching lead will run the aggregate gate; I ran only the two specifically named analyzer tests to classify their reported status.

## Examined scope

- `FR-186-AC-8` (`spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md`): “Over the service model, `on each (m: Service::Server) { probability >= 0.9 [ holds(P) ] }` checks and records activation `Each` with parameter type `Service::Server` and no guard; with `when (m.phase = Service::Phase::Idle)` it records that guard; the two checked claims have unequal normal forms. Neither is refused `not-yet-implemented`.”
- `FR-186-AC-9` (same file): “`on each (k: Integer)` is refused `over-not-object-type` at `k`, and `on each (m: Service::Server) when (1)` is refused `ill_typed`/`type-mismatch` at `1`.”
- `FR-186-AC-10` (same file), examined to classify the new test failure; source behavior is outside the acceptance boundary: “Through `fsl analyze` over the `Link` model, `probabilistic E using inf on M under Even on each (k: M::Msg) when (not k.delivered) { probability >= rational(9, 10) [ eventually holds(k.delivered) ] }` settles `proved`, never `unsupported`, with one instance per `Msg` key of the start snapshot.”
- `FR-186-AC-11` (same file): “Over the untimed service model, `expected elapsed until holds(P) <= 3 ms`, the measure `elapsed from holds(A) until holds(B) within 5` and the window `steps from holds(A) until holds(B) within 5 ms` are each refused `ill_typed`/`type-mismatch` at `expected elapsed`'s quantity, at the measure and at `5 ms`; none is refused `not-yet-implemented`.”
- `FR-186-AC-12` (same file): “Over the timed `Retx` subject (`time dense unit ms`) under the timed profile, `expected elapsed until holds(c.late) <= 3 ms`, the measure `elapsed from holds(not c.late) until holds(c.late) within 5 ms >= 1 ms` and the window `steps from holds(not c.late) until holds(c.late) within 5 ms >= 1` each check, the `expected elapsed` threshold recorded as 3 and each horizon as 5, in milliseconds.”
- `FR-186-AC-13` (same file): “A claim under the timed profile over the untimed service model is refused `invalid_timed_model`/`model-time-on-untimed-model` naming `Service`, at the `using` alias, and not `timed-event-profile`; under the timed profile over `Retx`, `probability >= 0.99 [ eventually[0 ms, 4 ms] holds(c.late) ]` checks.”
- `FR-186-AC-14` (same file): “For each quantity row of `Quantity positions` a quantity of the position's dimension checks and a quantity of length (`5 m`) is refused `ill_typed`/`unit-mismatch` naming `Length` at the literal; `fraction holds(P) over [0, 10] <= 5 ms` and `steps from holds(A) until holds(B) within 5 >= 5 ms` are refused `ill_typed`/`type-mismatch` at `5 ms`; no position refuses a quantity literal `not-yet-implemented`.”
- `FR-195-AC-7` (`spec/functional/FR-195-check-exact-only-forms-and-route-exact-evidence.md`): “§15.5's `MeanTime` (`time dense unit ms`) written `<= 3 ms` checks as `MeanTime` with `AtMost(3)`; written `<= 0.003 s` it records `3`; written `>= 2 ms` it checks with `AtLeast(2)`; written `<= 3 m` it is refused `ill_typed`/`unit-mismatch`; written with the bare `<= 3` it is refused `ill_typed`/`type-mismatch`. `Link`, whose model declares no time binding, with `expected elapsed until holds(m.delivered) <= 3 ms` is refused `ill_typed`/`type-mismatch` located at the `expected elapsed` quantity, and not `not-yet-implemented`. ”
- `TC-2166`, `TC-2168`, `TC-2169`, `TC-2170`, and `TC-2171`: test procedures and expected results were checked as context for the corresponding FR criteria.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The PR enables `probabilistic_on_each_settles_per_instance` and adds `probabilistic_on_each_empty_population`, but both fail with `invalid_capability`/`unknown-claim` before reaching the changed checker. Source request routing is not implemented in this diff and is tracked by QSL-2158, so these active tests make the analyzer integration target fail before that follow-up lands. Keep these source-path checks deferred or otherwise prevent them from failing this PR's test lane. | fsl-analyze/tests/it/probability_from_source.rs:162-201 | correct-requirement-no-evidence |

## Test evidence

- `CARGO_TARGET_DIR=/home/peter/.cache/fsl-qsl-lane-target cargo test --locked -p fsl-analyze --test it probability_from_source::probabilistic_on_each_settles_per_instance -- --exact --nocapture` exited 101. At line 173, the result was `declined` with `invalid_capability` / `unknown-claim`, not `proved`.
- The corresponding `probabilistic_on_each_empty_population` command exited 101. At line 201, the result was `declined` with the same `unknown-claim`, not `inconclusive`.
- These failures are source-routing behavior tracked separately by QSL-2158; they do not show a regression in the changed semantics checker.
- The aggregate gate and other tests were not run per the dispatch instruction.

