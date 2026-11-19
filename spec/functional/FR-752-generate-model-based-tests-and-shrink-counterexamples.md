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
`max_cases`, `max_trace_length`, `max_value_bits` (1 to 64) when a
generation root or an operation result has no finite bound (FSpec FR-190 "Value domains"), and the
run's limits and cancellation. A `verify` plan step with method
`model_based_test` (FR-753) supplies the clause, domain and bound
(`max_cases`), and the plan request supplies `seed`, `max_trace_length` and
`max_value_bits` (FSpec FR-824).

## Outputs

FSpec FR-190's result record: its settlement, `cases`, `checks`, `partitions`,
`partition_coverage`, operation coverage, provenance and at most one
retained counterexample.

## Behavior

- The generator SHALL be a deterministic function of the package, clause,
  profile, seed and limits; it SHALL first emit every boundary partition of
  FSpec FR-190 "Generation and shrinking contract", then seeded cases.
- The generator SHALL classify each generation root and operation result as
  FSpec FR-190 "Value domains" states. The generator SHALL draw a root with no finite bound from
  the size-bounded domain of `max_value_bits` in size order, and every other
  root from its declared domain unchanged, so `max_value_bits` never narrows
  a declared finite domain. The provenance SHALL record `max_value_bits`,
  one `size_bounded` entry per such root and `value_bound_cuts`.
- For a state model the entry SHALL give FR-120's `ModelSystem` the
  size-bounded domain of each root with no finite bound (an operation result
  included), and the indexed declared domain of each declared-finite root
  FR-120 does not enumerate, as that root's bound for the requires-bound
  check (FR-120 "Bounds"); a clock-bearing root FR-120 lists as unbounded keeps
  its `RequiresBound` refusal. An enabled application whose postcondition has no
  candidate post-state or result inside a size-bounded domain SHALL be
  counted as a value-bound cut and yields no successor; it is never a
  deadlock.
- Partition cases SHALL be built as FSpec FR-190 "Partitions" states: in the
  domain through the shortest enabled prefix the coverage exploration finds,
  or as refusal cases through the native admission that guards the root.
  For a parameter partition value that no explored state within the bounds
  enables, the generator SHALL produce a generated case discarded
  `precondition-false`, never a skipped one. For a field partition value
  that no explored state holds, the generator SHALL record the partition
  `uncovered` with its reachability reason.
- The generator SHALL make every seeded draw with FSpec's
  `filament.simulation.sampler/v1` draw function at the coordinates FSpec
  FR-190 "Seeded draws" binds (`trace` the case index, `step` the invocation
  index, `choice` the root's position), with its uniform choice extended to
  any number of items, so a run reproduces from its seed on every platform.
- The generator SHALL draw a `Text[min, max; profile]` root (a declared
  Text value type, FR-056) as FSpec FR-190 "Text domains" states: `unrank` of
  an index drawn uniformly over the whole declared domain, by exact integer
  arithmetic over the pinned Unicode 17.0.0 tables, never narrowed to an
  alphabet subset and never refused because the domain exceeds one draw
  value; and it SHALL emit the root's `normalization` partitions after its
  length partitions. Under `unicode-scalars` and `binary-utf8` canonically
  equivalent sequences are distinct values and are never merged, as FSpec
  FR-141 compares them; under `nfc`, `nfd`, `nfkc` and `nfkd` the domain
  holds normal forms only, so the generator never produces a value the type
  cannot retain.
- Filtering disabled argument vectors out of a `ModelSystem` expansion SHALL
  remain the construction of the enabled set: it produces no case and no
  count.
- The oracle SHALL evaluate the clause at the check points and under the
  premises FSpec FR-190 "Case accounting" gives its kind. A skipped check
  point is counted in `checks.skipped` and never passes a case.
- The result record SHALL carry `cases` (with `discarded_by`), `checks`,
  `partitions` and `partition_coverage`, and `max_cases` SHALL bound
  `cases.generated`, discarded cases included.
- A completed run with no failing and no passing case SHALL settle
  `inconclusive` with cause `no-evaluated-case`, never `tested`. A refusal
  case that native admission accepts SHALL settle the run `failed` with
  `tool-failure{defect: oracle-disagreement}`, and a shrink step that leaves
  the domain or changes the failing clause `failed` with
  `tool-failure{defect: invalid-shrink}`.
- With any size-bounded root or value-bound cut, operation and partition
  coverage SHALL NOT report `unreachable`, and every
  `unreachable-within-bound` entry SHALL carry `bounds` (FSpec FR-190
  "Operation coverage").
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
| FR-752-AC-3 | A `verify` step `check t claim Bounded method model_based_test domain M::Counter bound 50;` runs at most 50 cases (`cases.generated` at most 50, discarded cases included) and its record names the step; a passing run settles no verdict and records `tested`. | Test (TC-2102) |
| FR-752-AC-4 | Over a counter model with `inc` (enabled while `value < 5`, `value` in `Int[0, 5]`), `reset` (enabled at `value = 5`) and `jam` (enabled at `value > 5`), seed 7, `max_cases` 100 and `max_trace_length` 8, `operations` lists `inc` and `reset` `covered` with their first case and `jam` `uncovered`, reason `unreachable`, and `coverage` is `partial`; with `max_trace_length` 3, `reset` is `uncovered`, reason `unreachable-within-bound` with `bounds` `["max_trace_length"]`. Two equal runs give equal `operations`, and the passing run stays `tested` with no verdict. | Test (TC-2956) |
| FR-752-AC-5 | Over the predicate `Square` with parameter `x: Integer` and body `x * x >= 0`, seed 7, `max_cases` 20 and `max_value_bits` 8: the first five cases are `x` = 0, -1, 1, -255 and 255 (`size-least`, `size-unit` twice, `size-extreme-low`, `size-extreme-high`); no case has `x` outside `[-255, 255]` and no partition kind ends in `-refusal`; `provenance.max_value_bits` is 8 and `provenance.size_bounded` has one entry with `values` `"511"`; the run is `tested` with `cases.generated` 20. Over the body `x < 200` the retained counterexample is `x` = 200, which native replay fails. `max_value_bits` 0 and 65 each refuse `invalid_runtime_input`/`invalid-value`, and `max_value_bits` 8 over the predicate `value <= 3` with parameter `value: Int[0, 5]` refuses `invalid_runtime_input`/`unknown-member`; each refusal names `max_value_bits` and runs no case. | Test (TC-2102) |
| FR-752-AC-6 | Over a counter with `value: Integer` (initial 0), `add(by: Int[1, 100])` (`value' = value + by`) and `big()` (enabled when `value >= 300`), seed 7, `max_cases` 100, `max_trace_length` 8 and `max_value_bits` 8: `big` is `uncovered` with reason `unreachable-within-bound` and `bounds` `["max_value_bits"]`, never `unreachable`; `provenance.value_bound_cuts.count` is at least 1; the run is `tested`. With `max_value_bits` 10, `big` is `covered`, or `not-generated` with an `enabled_at` prefix of `add` invocations whose arguments sum to at least 300. | Test (TC-2956) |
| FR-752-AC-7 | Over a counter with `value` in `Int[0, 5]` (initial 0), `inc(by: Int[1, 6])` (enabled when `value + by <= 5`) and `jam()` (enabled when `value > 5`), the postcondition clause `Inc` of `inc`, seed 7, `max_cases` 100 and `max_trace_length` 8: cases 0 to 7 are the partitions of `value` (-1, 0, 5, 6) followed by those of `by` (0, 1, 6, 7); the cases for `value` -1 and 6 and `by` 0 and 7 are refusal cases discarded `domain-refusal` with their partitions `refused`; the `by` 6 case is discarded `precondition-false` and its partition is `uncovered` with reason `precondition-false` and `bounds` `[]`; `cases.generated` is 100 and equals `passed + failed + discarded`; the run is `tested`. The precondition clause of `inc` over the same model has `discarded_by.precondition-false.count` 0, because a false precondition is a defined, passing point. The postcondition clause of `jam` discards all 100 cases, with `discarded_by` `domain-refusal` `{count: 4, first_case: 0}`, `precondition-false` `{count: 1, first_case: 6}` (the `by` 6 partition case), `invariant-false` `{count: 0, first_case: null}` and `no-check-point` `{count: 95, first_case: 1}`, has `cases.passed` 0, and settles `inconclusive` with cause `no-evaluated-case`, never `tested`; as step `j` of a plan whose step `k` has `depends [j]`, `k` is `blocked` by `j` and the exit is `22`. | Test (TC-2102) |
| FR-752-AC-8 | Over AC-7's model with a test-double argument admission that accepts `by` 7, the run settles `failed` with `{"type": "tool-failure", "defect": "oracle-disagreement"}`, exit `30`, and writes no `tested` record. Two runs of AC-7 with equal inputs give byte-equal `cases`, `checks`, `partitions` and `partition_coverage`. | Test (TC-2102) |
| FR-752-AC-9 | Over a value clause `Label` with parameter `t: Text[0, 4; nfc]` and body `size(t) <= 4`, seed 7 and `max_cases` 30, the first cases are `t`'s partitions in FSpec FR-190-AC-19's order: the empty text, U+0000, U+0000 repeated 4 times, the `upper-refusal` refusal case of length 5 (discarded `domain-refusal`), then U+00E9, U+1E0D U+0307, U+AC00, U+00C5 and U+FB01; every seeded case is an `nfc`-normalized member of length at most 4; the run is `tested`. The same clause over `Text[0, 4; unicode-scalars]` has the eight witness spellings as eight distinct cases, U+00E9 and U+0065 U+0301 among them. Over `t: Text[0, 13; unicode-scalars]` the run draws 30 cases and is never `unsupported`. The generator's `unrank` gives FSpec FR-190-AC-18's values. With seed 7 the first seeded `x` of `Square` (`max_value_bits` 8) is 53 and the first seeded `t` over `Text[0, 2; unicode-scalars]` is U+D51A9 U+87AC9, the values FSpec FR-190-AC-21 computes. | Test (TC-2102) |

## Dependencies

- FR-120 (model transition system), FSpec FR-181 (execution), FR-753 plan
  steps; FSpec FR-190.
