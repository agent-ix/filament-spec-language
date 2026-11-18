---
id: FR-752
title: "Generate model-based tests from a checked model and shrink failing cases to a 1-minimal counterexample"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-190
    type: depends_on
---
# FR-752: Generate model-based tests from a checked model and shrink failing cases to a 1-minimal counterexample

## Description

`fsl-eval` SHALL implement FSpec FR-190's model-based testing: generate
cases from the checked domains and test traces from a state model's
operations, evaluate the named clause natively as the oracle, and shrink a
failing case by FSpec FR-190's pinned order to a 1-minimal counterexample
that native replay reproduces.

## Inputs

A checked package, one clause identity, a generator profile, a seed,
`max_cases`, `max_trace_length`, `max_value_bits` where a domain has no
finite bound, and the run's limits and cancellation. A `verify` plan step
with method `model_based_test` (FR-753) supplies the clause, domain and
bound (`max_cases`).

## Outputs

FSpec FR-190's result record: counts, partitions covered, provenance and at
most one retained counterexample.

## Behavior

- The generator SHALL be a deterministic function of the package, clause,
  profile, seed and limits; it SHALL first emit every boundary partition of
  FSpec FR-190 "Generation and shrinking contract", then seeded cases.
- Test traces SHALL be built by FR-120's `ModelSystem`: each invocation is
  chosen among those enabled at the current state, so no generated trace
  applies a disabled invocation, and every state is the system's successor.
- The oracle SHALL be the native evaluator: FSpec FR-181 execution for state clauses and
  contracts, the temporal evaluator under the clause's profile for temporal
  clauses, and value evaluation for value clauses.
- The shrinker SHALL follow FSpec FR-190 "Shrinking guarantee".
- The shrinker SHALL replay every accepted candidate natively.
- A retained counterexample settles the clause `refuted` only after native
  replay; a run with no failure records `tested` evidence and settles no
  verdict.
- For a clause over a state model, the result record SHALL carry FSpec
  FR-190 "Operation coverage": one `operations` entry per operation the
  model declares, `covered` with its first case or `uncovered` with its
  reason, and the `coverage` summary. The reachability behind `unreachable`,
  `unreachable-within-bound` and `not-generated` SHALL come from exploring
  FR-120's `ModelSystem` from the generation's initial-state domain under
  the run's limits, never from the generated cases alone, so an operation
  no case happened to apply is not reported unreachable.
- Coverage SHALL NOT change the item's verdict or its FR-285 category: an
  uncovered operation leaves a passing run `tested` with the exit of a
  completed run, and a refuted run exits as a violation, with the uncovered
  entries beside the counterexample.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-752-AC-1 | Over `test/counters` with `inc` on `Int[0, 5]` and `value <= 3`, every generated invocation is enabled, the first cases include `value` 0 and 5 and the just-outside refusals, and the retained counterexample is four `inc` from 0, 1-minimal and replay-reproduced. | Test (TC-2102) |
| FR-752-AC-2 | For every seed in a fixed set, every shrink step the shrinker accepts is strictly smaller in the pinned order, domain-valid, enabled and fails the same clause; no single removal or one-step decrease of the retained case still fails. | Test (TC-2102) |
| FR-752-AC-3 | A `verify` step `check t claim Bounded method model_based_test domain M::Counter bound 50;` runs at most 50 cases and its record names the step; a passing run settles no verdict and records `tested`. | Test (TC-2102) |
| FR-752-AC-4 | Over a counter model with `inc` (enabled while `value < 5`, `value` in `Int[0, 5]`), `reset` (enabled at `value = 5`) and `jam` (enabled at `value > 5`), seed 7, `max_cases` 100 and `max_trace_length` 8, `operations` lists `inc` and `reset` `covered` with their first case and `jam` `uncovered`, reason `unreachable`, and `coverage` is `partial`; with `max_trace_length` 3, `reset` is `uncovered`, reason `unreachable-within-bound`. Two equal runs give equal `operations`, and the passing run stays `tested` with no verdict. | Test (TC-2952) |

## Dependencies

- FR-120 (model transition system), FSpec FR-181 (execution), FR-753 plan
  steps; FSpec FR-190.
