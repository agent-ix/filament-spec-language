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

## Entry

`fsl-eval` SHALL expose model-based testing as one public entry in
`fsl_eval::simulation::model`:

```rust
pub fn run_model_based_test(
    package: &fsl_package::CheckedPackage,
    request: &ModelBasedTestRequest<'_>,
    admission: &dyn RootAdmission,
    cancel: &filament_exact::Cancel,
) -> Result<ModelBasedTestRecord, ModelBasedTestRefusal>
```

- `ModelBasedTestRequest` holds the clause identity, the generator profile,
  `seed`, `max_cases`, `max_trace_length` (`Option<u64>`), `max_value_bits`
  (`Option<u64>`), the model subject (`Option<ModelSubject>`) and the run's
  limits.
- `ModelSubject` is the subject a clause over a state model is tested
  against: FR-120's `ModelSimulationRequest` members other than the package
  and the limits, that is `packages` (FR-056), `snapshots` (the FR-106
  provision), `initial` (the initial snapshots) and `universes`. A value
  clause has none.
- The limits are the scalar accounting limits of every evaluation the run
  makes (FSpec FR-461 `accounting.*`), the observation and normalization
  limits the subject is admitted under (FR-106), the reachability
  exploration limits (`explore.states` and `explore.transitions`) and the
  position limit of one generated trace (`simulation.max_steps`), each a
  FR-255 setting.
- The entry SHALL build the run's model system itself. For a clause over a
  state model it builds FR-120's `ModelSystem` from the package and the
  subject, giving it the size-bounded domain of each root with no finite
  bound ("Behavior"), the request's accounting meter, its expansion limits
  and its observation and normalization limits, so every bound the request
  states reaches the system. For a value clause it builds no
  `ModelSystem`: the generator draws the clause's parameters and the oracle
  evaluates the clause by value evaluation.
- `ModelBasedTestRefusal` is a refusal of the whole request before any case
  runs, and nothing else: the request-member refusals of FSpec FR-190
  "Refusals" (no clause, an unknown clause identity, a `max_cases` or
  `max_trace_length` of 0, and the `max_value_bits` refusals of "Value
  domains"), each naming the member; a `max_trace_length` or `subject` that
  is `None` for a clause over a state model, `invalid_runtime_input`/
  `missing-member` naming the member, `max_trace_length` first, as FR-753
  refuses a missing plan input; a `subject` supplied for a value clause,
  `invalid_runtime_input`/`unknown-member` naming `subject`; a subject
  `ModelSystem::new` does not admit, with FR-120's admission refusal; the
  empty-domain refusal `invalid_model_binding`/`malformed-declaration`; and
  FR-120's `NotSimulated::RequiresBound` when a population has no finite
  universe for the initial-state domain. That refusal is represented by
  `member: "subject"`, `code: "requires_bound"`, `cause: "requires-bound"`
  and `details.domains`, the ordered FR-120 root list with each existing
  root's structural `key` and `kind`; it does not become a model-based-test
  record and no case runs.
  A `max_trace_length` supplied for a value clause is accepted and unused,
  as FR-753 hands it to every consuming step alike.
- `ModelBasedTestRecord` is FSpec FR-190's result record, and every FSpec
  FR-190 settlement lives in it, never in the refusal: `failed` with
  `tool-failure{defect: oracle-disagreement}` or
  `tool-failure{defect: invalid-shrink}`, `incomplete` with the
  `limit-reached` cause of the limit or the cancellation that stopped the
  run (FSpec FR-461), `unsupported` for a `frame` clause with no
  contract-level reading, `inconclusive` with `no-evaluated-case`, `tested`
  and `refuted`. A stop inside native evaluation, expansion, replay or
  shrinking is therefore a settlement of the record, so a caller never has
  to map an error to a settlement.
- `RootAdmission` is the native admission that guards a generation root
  (FSpec FR-190 "Partitions"), one method per guard: argument admission for
  an operation argument and parameter admission for a value-clause
  parameter, each given the whole argument vector; and value admission for
  a field or initial-state value, given the declared field type and the
  value. Each returns admitted or refused.
- `NativeAdmission` is the `RootAdmission` every production caller passes.
  Its argument and parameter admission is
  `fsl_eval::value::expression::admit_arguments`, called with the vector
  that holds the refusal value at the root's position and every other root
  at its least member, so the `Population<T>[N]` maximum check and object
  admission apply as they do to a supplied call (FR-089). Its field and
  initial-state admission is the kernel `ValueType::admits` on the declared
  field type, the per-value check FR-106 snapshot admission applies to each
  field.
- A test passes another `RootAdmission`, a test double, to drive the
  `oracle-disagreement` path; the generator, oracle and shrinker are the
  same code under either admission.
- The plan coordinator (FR-753) calls this entry for a `model_based_test`
  step with the checked package, the step's clause, the request's `seed`,
  `max_trace_length` and `max_value_bits`, for a clause over a state model the model subject
  whose `initial` is the step's admitted `initial` snapshot and whose
  `universes` are from the admitted `universe_snapshot` when present (FR-1038),
  otherwise from its initial snapshots, the accounting, observation and normalization limits from
  `AnalyzeLimits` and the exploration and position limits from
  `AnalyzeLimits.simulation` (FR-281),
  passes `NativeAdmission`, and maps the record's `result` and `cause` to
  the step's disposition (FSpec FR-824); a `ModelBasedTestRefusal` settles
  the step as the request refusal it is.

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
- When no initial snapshots are supplied for a state subject, the
  model-based-test generator SHALL draw each initial state from FR-120's
  product of complete states over the supplied finite population universes
  and the roots' FR-190 domains, and its reachability exploration SHALL use
  that same domain. Neither operation SHALL infer or create object keys or
  population universes. When a population root is class P without a finite
  universe, the model-based-test entry SHALL return FR-120's
  `NotSimulated::RequiresBound` refusal before any case runs; the entry SHALL
  NOT convert that refusal to a settlement or a value-bounded population.
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
- The generator SHALL charge a Text root's work to the run's
  `accounting` meter before it allocates, as FSpec FR-190 "Text domains"
  states: one `work_units` per cell of the root's count table, charged whole
  when the root's domain is prepared before the run's first case, and one
  `normalized_scalars` per scalar of every Text value it builds. A charge the
  budget cannot cover settles the run `incomplete` with
  `limit-reached{limit, value, setting: accounting.<counter>}`; the
  generator never panics, never narrows the declared domain and never
  settles `unsupported` because a Text root is large.
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
| FR-752-AC-8 | Over AC-7's model with a test-double `RootAdmission` passed to `run_model_based_test` that accepts `by` 7, the entry returns `Ok` with a record settled `failed` with `{"type": "tool-failure", "defect": "oracle-disagreement"}`, exit `30`, and writes no `tested` record. Two runs of AC-7 with equal inputs give byte-equal `cases`, `checks`, `partitions` and `partition_coverage`. | Test (TC-2102) |
| FR-752-AC-9 | Over a value clause `Label` with parameter `t: Text[0, 4; nfc]` and body `size(t) <= 4`, seed 7 and `max_cases` 30, run through `run_model_based_test` with no subject, the first cases are `t`'s partitions in FSpec FR-190-AC-19's order: the empty text, U+0000, U+0000 repeated 4 times, the `upper-refusal` refusal case of length 5 (discarded `domain-refusal`), then U+00E9, U+1E0D U+0307, U+AC00, U+00C5 and U+FB01; every seeded case is an `nfc`-normalized member of length at most 4; the run is `tested`. The same clause over `Text[0, 4; unicode-scalars]` has the eight witness spellings as eight distinct cases, U+00E9 and U+0065 U+0301 among them. Over `t: Text[0, 13; unicode-scalars]` the run draws 30 cases and is never `unsupported`. The generator's `unrank` gives FSpec FR-190-AC-18's values. With seed 7 the first seeded `x` of `Square` (`max_value_bits` 8) is 53 and the first seeded `t` over `Text[0, 2; unicode-scalars]` is U+D51A9 U+87AC9, the values FSpec FR-190-AC-21 computes. | Test (TC-2102) |
| FR-752-AC-10 | Over a value clause `Huge` with parameter `t: Text[0, 1099511627776; nfc]` (`max` 2^40) and body `size(t) >= 0`, seed 7, `max_cases` 30, no subject, `max_trace_length` `None` and `accounting.work_units` 1000000, `run_model_based_test` returns `Ok` with a record settled `incomplete`, cause `{"kind": "limit-reached", "limit": "work_units", "setting": "accounting.work_units", "value": 1000000}`, and `cases.generated` 0; it does not panic or abort, the record is not `unsupported`, and `provenance.size_bounded` is empty. The same clause over `Text[0, 1099511627776; unicode-scalars]` settles the same way. Over `Text[0, 4; nfc]` with the same limits the run is `tested` with `cases.generated` 30, and with `accounting.work_units` 2559, one below the 2560 FSpec FR-190 "Text domains" charges for its count table, it settles `incomplete` naming `accounting.work_units` with `cases.generated` 0. | Test (TC-3068) |
| FR-752-AC-11 | `run_model_based_test` returns `Err(ModelBasedTestRefusal)` naming the member, and runs no case, for `max_cases` 0, `max_trace_length` `Some(0)`, an unknown clause identity, and `invalid_model_binding`/`malformed-declaration` for a clause over an empty domain. For AC-7's clause `Inc` it returns `invalid_runtime_input`/`missing-member` naming `max_trace_length` when that is `None`, and naming `subject` when the subject is `None`; for the value clause `Square` with a subject it returns `invalid_runtime_input`/`unknown-member` naming `subject`, and with `max_trace_length` `None` it runs. With `NativeAdmission`, AC-7's clause `Inc` (package, subject with `initial` value 0, seed 7, `max_cases` 100, `max_trace_length` `Some(8)`) returns `Ok` settled `tested`, and its `by` 0 and `by` 7 refusal cases are refused by `admit_arguments` over the argument vector `[by]`. The same request with `explore.states` 1 returns `Ok` settled `tested`; its `by` 6 partition is `uncovered` with reason `undetermined` naming `{"limit": "max_states", "setting": "explore.states", "value": 1}` (FR-255-AC-13), no case is generated for it, and `jam`'s `operations` entry is `undetermined` naming the same limit. As a `model_based_test` plan step running `Inc` with `AnalyzeLimits.simulation` `max_states` 1, the step's record equals that direct run's. | Test (TC-3068) |

| FR-752-AC-12 | Over package `test/no-initial`, whose successful `NoInitial` model declares only `cells: Population<Cell>` of record value type `ix://test/no-initial/Cell` with `value: Int[0,1]`, the Cell declaration's model node key is `{package:"test/no-initial",path:["model","record_value_type","ix://test/no-initial/Cell"],ordinal:0}` by FR-322. With `universe_snapshot` row `cells` `complete: true` and object keys `[c1,c2]`, a state-clause model-based-test request with no initial snapshots, seed 7, `max_cases` 100 and `max_trace_length` 8 settles `tested`; every case's initial state belongs to the four-state product domain `{((c1,0),(c2,0)),((c1,0),(c2,1)),((c1,1),(c2,0)),((c1,1),(c2,1))}`, and reachability uses that same domain. The first seeded initial state is the uniform draw at FR-190 `step` 0, `choice` 0; repeated equal requests produce byte-equal records. No other object key or population is present. In a separate refusal fixture, the model declares `cells` followed by class-P root `unbounded_cells`, both of `Cell`, and the snapshot has `cells` complete with `[c1,c2]` and `unbounded_cells` `complete: false` with no objects. That request returns `Err(ModelBasedTestRefusal { member: "subject", code: "requires_bound", cause: "requires-bound", details: {domains: [{key: DomainKey::Population {member_type: {package:"test/no-initial",path:["model","record_value_type","ix://test/no-initial/Cell"],ordinal:0}, ordinal: 1}, kind: Population(None)}]} })` before any case runs. | Test (TC-3018) |

## Dependencies

- FR-120 (model transition system), FSpec FR-181 (execution), FR-753 plan
  steps; FSpec FR-190.
