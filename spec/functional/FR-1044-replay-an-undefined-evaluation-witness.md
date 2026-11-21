---
id: FR-1044
title: "Replay an undefined-evaluation witness"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-023
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-202
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-196
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-203
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-201
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-413
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-411
    type: depends_on
---
# FR-1044: Replay an undefined-evaluation witness

## Description

When a probabilistic claim evaluates undefined at a product state reached
with positive probability, EN-5 SHALL refute it with FR-202's `Undefined`
evidence, written as FSpec FR-413's `undefined-path`, and the replay core
SHALL settle the item `refuted` only after re-executing the path and
reproducing the same first undefined read itself. The bound plays no part:
a claim whose value is undefined on some behaviour of positive probability
has no probability to compare (FSpec FR-363, FR-411).

## Use case

`Dropped` over `Coin` asks that `c.v = 0` hold for three steps with
probability at least `0.01`. `drop` with `b = 0` deletes the coin, with
probability `1/2`, and at position 1 `c.v` has no value. EN-5 stops the
build there and returns the one-step undefined path. Replay re-executes
`drop` with `b = 0`, checks its probability `1/2` is positive, rebuilds the
`always` monitor, finds `c.v = 0` defined and true at position 0 and
undefined, `absent-key`, at position 1, and settles `refuted`. The same
evidence names position 0 in a forged copy, and replay settles
`inconclusive`.

## Inputs

- FR-196's `ProbBuild::Undefined(UndefinedPath)`: the initial-state index,
  binding and instance, the steps with each post-state `resolved`, the
  complete pre-read key, the read and its `UndefinedCause`.
- FR-202's `WitnessEvidence::Undefined { path, undefined }`, its
  `ProbabilisticCounterexample` and, over every scheduler, its witness
  scheduler.

## Outputs

- A reproduced-witness token (FR-202) or `ReplayParity`, and FSpec FR-413's
  `undefined-path` evidence in the item's `counterexample` (FR-741).

## Behavior

### Emission

- EN-5 SHALL build `Undefined` evidence from the first undefined product
  state in FR-181 canonical breadth-first order (FSpec FR-411). Its path is
  then a shortest one, so no complete product key repeats on it, and the
  choices it takes form a memoryless deterministic scheduler on its keys.
  Over every scheduler the counterexample's `scheduler` SHALL have one entry
  per choice state of the path, naming the action taken there.
- `undefined` SHALL be `UndefinedEvaluation{where, cause}` with `where`
  carrying the expression's locus, the read's event position, the
  initial-state index and binding, and the complete pre-read product key
  (FR-196-AC-13), and SHALL equal the item's result cause. The wire SHALL
  be FSpec FR-413's `{"type": "undefined-path", "path", "undefined"}`, with
  `where.product_state.key` the base64url `ProbabilityProductKey` octets.

### Replay

- Layer 6 SHALL expose a checked-subject entry for `Undefined` evidence
  beside FR-202's path-set entries, taking the `CheckedPackage`, its
  admitted `ModelSystem`, the claim occurrence and binding, the typed
  evidence, the caller's limits and `&Cancel`; the wire facade SHALL
  decode FSpec FR-413's spelling into the same evidence and run the same
  checks.
- Replay SHALL bind the claim, its profile, scheduler or workload, fairness
  set, `over` binding and `on each` instance from the package, as FR-202's
  checked-subject entries do, and SHALL re-execute every step: the action is
  enabled, each drawn value is in its support, the post-state and complete
  product key are rederived, and the step's probability is positive (the
  workload's weight and the draw's probability, or under every scheduler
  the draw's probability with the step taking the witness scheduler's
  choice, which some scheduler takes with probability 1). A complete key
  that repeats on the path SHALL settle `ReplayParity`, since the
  memoryless scheduler it requires is not defined.
- Replay SHALL rebuild the claim's control along the path with the
  qualified operations the certificate checker uses: `fsl-eval`'s bounded
  monitor (FR-201's bounded subset, `eventually[0,h]` and `always[0,h]`),
  the infinite-trace state-predicate reading of the unbounded forms, the
  measure and fraction control of FR-1043, the reward of each step for an
  expected reward, and the predicate and weight of each position for a
  long-run fraction; over a timed subject it SHALL use FR-921's own timed
  enumerator. At each position before the read every read of the claim
  SHALL be defined, and the control SHALL still be undecided (for an
  unbounded form, no target or failure reached), because a decided claim
  reads no later value.
- At the read position replay SHALL evaluate the claim's reads in ADR-018
  UE-5's order and require the first undefined one to be at
  `where.locus`, with `where.position`, the complete pre-read key
  `where.product_state` and cause `cause`. Agreement SHALL issue the
  reproduced token, and the item SHALL settle `refuted`,
  `decisive-counterexample`, cause `UndefinedEvaluation{where, cause}`,
  category violation. Any other outcome (a defined value, an earlier
  undefined read, a different locus, position, key or cause, a decided
  control before the read, or an item cause that differs from
  `undefined`) SHALL settle `inconclusive`, `ReplayParity`. A step that is
  not enabled, a drawn value outside its support or a step that departs
  from the scheduler SHALL refuse `invalid_runtime_input`/`invalid-value`
  (FR-202).
- The threshold, the bound's side and the path's mass beyond being positive
  SHALL play no part, and no sum is formed.

### Bounds

- Replay SHALL bound the encoded request by `replay.input_bytes` before
  provision access (FR-202-AC-7). Its work is the path length times the
  enabled transitions and reads at each position, and it SHALL run under
  the caller's expansion, work, storage and arithmetic limits and
  `&Cancel`; a stop or cancellation SHALL settle `incomplete` with no token
  and no refuted record.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-1044-AC-1 | FR-196-AC-5's `Coin` claim `probability >= T [ always[0,3] holds(c.v = 0) ]` under `Even`, for `T` 0.5 and 0.01, settles `refuted`, `decisive-counterexample`, cause `UndefinedEvaluation{where: 1, cause: absent-key}`, through `analyze`, with a `counterexample` whose evidence is `{"type": "undefined-path", "path": [drop with b = 0], "undefined": …}`, `where` naming position 1, the locus of `c.v`, initial state 0, the binding and the pre-read key, and an item `replay` that is present and reproduced; replaying it later over the emitted package reproduces it (FR-203-AC-5, FR-202-AC-5). | Test (TC-641) |
| FR-1044-AC-2 | AC-1's evidence with `position` 0, with the path `drop` with `b = 1` (where `c.v` is defined), with another catalogued cause, or offered for an item whose cause names another locus settles `inconclusive`, `ReplayParity`; for `probability >= 0.5 [ always[0,3] holds(c.v = 1) ]`, which the monitor rejects at position 0 before any later read, a forged undefined path to position 1 settles `inconclusive`, `ReplayParity`, and the item settles `refuted` by its value 0 with path-set evidence. | Test (TC-3153) |
| FR-1044-AC-3 | `Coin` with a further operation `keep` that never deletes, under every scheduler: the claim of AC-1 settles `refuted` with the undefined path `drop` with `b = 0` and a scheduler entry `drop` at the initial key, although a scheduler that always takes `keep` keeps `c.v` defined; the same path with a scheduler entry `keep` at that key refuses `invalid_runtime_input`/`invalid-value`. | Test (TC-3153) |
| FR-1044-AC-4 | Under the infinite-trace profile `probability >= 0.5 [ eventually holds(c.v = 1) ]` over `Coin` settles `refuted` with the undefined path `drop` with `b = 0` and `where.position` 1; the claim's unbounded control has null monitor and accumulator in the pre-read key. | Test (TC-3153) |
| FR-1044-AC-5 | AC-1's replay with pre-cancellation, cancellation at its final poll, `replay.input_bytes` one below the request's size, or each expansion, work or storage limit exhausted returns `incomplete` or the request refusal with no token and no refuted record. | Test (TC-3153) |

## Dependencies

- [FR-196](FR-196-build-the-probabilistic-product.md) builds the undefined
  path; [FR-202](FR-202-replay-a-probabilistic-witness.md) owns the replay
  entries and settlement token; [FR-203](FR-203-bound-an-exact-run-and-settle-its-verdict.md)
  the settlement row; FSpec FR-413 the wire.

## References

- Ticket: Linear QSL-2158.
