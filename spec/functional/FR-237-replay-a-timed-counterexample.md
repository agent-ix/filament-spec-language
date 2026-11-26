---
id: FR-237
title: "Replay a timed model or task-automaton counterexample"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-026
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-098
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-101
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-128
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-231
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-232
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-236
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-249
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-217
    type: references
  - target: ix://agent-ix/filament-specification/FR-331
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-364
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-120
    type: depends_on
---
# FR-237: Replay a timed model or task-automaton counterexample

## Description

`fsl_replay::replay_model_trace` (FR-128) SHALL replay a counterexample
over a timed subject in exact rational arithmetic (ADR-026 CT-3, TD-3): it
starts every clock at 0, checks each delay is admissible, checks each
transition is enabled after its delay, selects the successor by the
post-state key, applies resets, checks time invariants, checks the timed
lasso's closure and fairness, and evaluates the formula over the timed
lasso. A time-lock counterexample replays by its own arm. Replay needs no
zone engine.

For a task automaton, the existing `ModelTrace` replay entry SHALL rebuild
FR-249's task-derived timed subject and pass the existing FSpec FR-364 model
trace to the FSpec FR-364 replay contract. It SHALL return FR-072's existing
ModelTrace-arm result. FSL SHALL NOT add a `TaskAutomatonTrace` result arm or
a parallel trace wire. FSpec FR-364 owns task-step and continuation replay;
FR-331 owns incomplete `task_search` evidence. A frontier replay never
implies absence of an unseen miss.

A task trace has no wire spelling of its own. Its steps are FSpec FR-364
model-arm steps whose `transition` is an FR-331 `TransitionIdentity`
(`transition` or `task-transition`) or the `terminal-stutter` marker, told
apart by `type`. The FR-331 contract version covers
them, and FSL adds no member, discriminator or version. In Rust,
`TaskProductTransition` holds this task-derived alphabet: `Source`, `Task` and
`TerminalStutter`. `ModelTransition` holds the plain model alphabet. The
`ModelTrace` entry therefore has two typed forms over one source arm and one
result arm, in the same way as FR-217's protocol form:

- `replay_model_trace` over `ModelTransition`;
- `replay_task_product_trace` over `TaskProductTransition`.

The entry also has one document form, which reads the wire.

## Use case

An auditor with only the source, the subject's snapshots and a refutation
of a timing claim replays it. A counterexample whose delay violates a time
invariant, or whose lasso does not let time diverge, does not count as a
refutation.

## Inputs

- FR-128's request and a `WitnessEnvelope<TemporalCounterexample<T>>` over a
  timed subject (FR-236), with `T` either `ModelTransition` or
  `TaskProductTransition`; or, for the document form, a
  `WitnessEnvelope<CounterexampleDocument>` whose payload is the FSpec
  FR-364 counterexample JSON value.
- For a non-local time-lock: the request's exploration limits.
- For a task-derived model trace: FR-249's checked task product and FSpec
  FR-364's derived-step identities.

## Outputs

- An FR-072 replay result on the existing `ModelTrace` arm, or a typed
  `ReplayRefusal` with no partial result, as FR-128 states.

## Behavior

- Replay SHALL apply FR-128's recompile, fairness-set and
  admission rules unchanged.
- The executor SHALL start every clock of the subject at 0.
- For each step, the executor SHALL:
  - check the delay is admissible at the current timed state (FR-231);
    if not, refuse `invalid_runtime_input`/`invalid-value`, naming the step
    and the time invariant or urgency that forbids it;
  - apply the delay, then check the transition identity is enabled at the
    delayed state, refusing `invalid_runtime_input`/`invalid-value`
    naming the step and the guard when it is not;
  - select the successor whose post-state key equals the recorded one;
  - apply the resets and check the target's time invariants, refusing
    `invalid_runtime_input`/`invalid-value` when one fails.
- Every comparison and sum SHALL be exact.
- **Task-derived replay.** For a task automaton, the executor SHALL rebuild
  the product through FR-249 and invoke the FSpec FR-364 replay contract on
  the existing model-trace payload. The replay settles reproduced only when
  that contract accepts the task-derived steps and the required continuation.
  FSL SHALL not define another derived-step identity or divergence witness.
  The rebuilt product includes the package's checked mode-switch
  declarations. Replay derives the criticality mode and allowance phases from
  them and from the replayed steps (FSpec FR-364), and never reads them from
  the counterexample.
- **Reading the wire.** `fsl_replay::read_temporal_counterexample::<T>` SHALL
  be FSL's one strict reader of an FSpec FR-364 model-arm counterexample.
  `T` is the step alphabet, `ModelTransition` or `TaskProductTransition`. The
  reader SHALL read exactly the FR-331 schema members of
  `TemporalCounterexample`, `ModelStep` and `TaskTransitionIdentity`. The
  reader SHALL refuse `malformed_wire`
  with a cause and the RFC 6901 pointer of the offending member, relative to
  the counterexample, for each of these:
  - an unknown member (`unknown-member`), a missing member
    (`missing-member`) or a member of the wrong kind (`wrong-value-kind`);
  - an unknown `type` or `action` (`unknown-variant`);
  - `budget_delta` present on a step whose `action` is not `budget-change`,
    or absent on one whose `action` is (`budget-delta-presence`);
  - a task step without `delay` (`missing-member` at `/delay`);
  - FSpec FR-364-AC-22's `mode` members;
  - FSpec FR-364-AC-17's continuation rules.

  Inside an FR-331 result, every pointer is prefixed
  `/counterexamples/<n>/counterexample`. Reading is lossless: FR-286's
  writer gives back the same JSON value from what the reader read. A step
  whose `type` lies outside `T`'s alphabet is not a shape fault. The reader
  returns it as an alphabet refusal, `invalid_runtime_input`/`invalid-value`
  at that step's `/transition/type` (FSpec FR-364 "Step alphabet"). The
  reader checks the whole payload's shape first. It returns an alphabet
  refusal only for a payload with no shape fault, so a shape fault at a later
  step wins over an alphabet fault at an earlier one.
- **Document form.** `fsl_replay::replay_model_trace_document` SHALL apply
  FR-128's recompile and item resolution first. It SHALL then take the
  alphabet from the resolved item and never from the payload:
  `TaskProductTransition` exactly when the item is a release-backed
  schedulability item over an FR-249 task product, and `ModelTransition`
  otherwise. It SHALL read the payload with that alphabet and run the
  matching typed form. Analyze SHALL replay each EN-6 task counterexample
  through this form, from the JSON it wrote, and not through a product
  self-replay. Every reported refutation has then passed the reader that an
  auditor uses.
- **Typed forms.** A typed form whose alphabet differs from the resolved
  item's SHALL refuse `ModelTraceRefusal::TraceAlphabet`
  (`invalid_runtime_input`/`invalid-value`, naming the required and the
  supplied alphabet) before any step replays. A `ModelTransition` payload for
  a task item holds only the source projection of a task trace. Every step of
  that projection is enabled in the source model, so replaying it on that
  model would drop every scheduler check.
- **Task-step equality.** On the task form, each step SHALL equal the step
  that the rebuilt product derives, by FSpec FR-364's "Synchronized source
  moves" and "Comparison with the re-derived step". The first difference
  refuses `invalid_runtime_input`/`invalid-value` at that member's pointer,
  naming the supplied and the recomputed value. A tamper is never reported
  only as a transition that is not enabled when one enabled move has the
  step's `action`.
- **Lasso.** Replay SHALL check the loop's closure (FR-236) and that its
  total delay is positive, refusing `invalid_runtime_input`/`invalid-value`
  otherwise. It SHALL check fairness over time (FR-232) and refuse an
  unfair lasso as FR-128 does.
- **Admitted endpoint.** For a finite `Formula`, `Deadlock` or
  `UndefinedEvaluation` counterexample, replay SHALL also explore the last
  replayed timed state's forward closure afresh, unreduced, under the
  request's limits, by the same exploration as the non-local time-lock arm,
  looking for a quiescent state or a cycle with positive total delay. An
  exploration that finds one SHALL let the refutation settle; one that
  completes with none SHALL settle `inconclusive`, `ReplayParity`, since the
  violation lies only on time-locked behaviour (ADR-026 TD-1, TV-1); one
  stopped by a limit SHALL settle V-7. This arm does not apply to a
  deadline-miss counterexample of a task-derived subject (FR-249): its
  continuation is the carried FSpec FR-364 witness (a finite loop, a
  terminal-stutter loop or `queue_growth`), and one that carries none is
  refused `malformed_wire` at `/loop` before any exploration.
- **Evaluation.** For `kind: Formula`, replay SHALL evaluate the formula by
  FR-231 over the timed lasso, unrolling the loop until the unrolled time
  span covers the formula's time reach. `false` SHALL settle
  `reproduced-with-evaluated-witness`; `true` or no value, `inconclusive`.
- **Undefined evaluation.** For `kind: UndefinedEvaluation`, replay SHALL
  evaluate the claim's letters along the replayed timed prefix by FR-231
  (ADR-018 UE-5). The first undefined evaluation at `where`, with an equal
  cause, SHALL settle `reproduced-with-evaluated-witness`; any other
  result SHALL settle `inconclusive`, `ReplayParity`.
- **Arm selection.** For `kind: TimeLock`, after the prefix and its
  `final_delay`, replay SHALL classify the last timed state by ADR-026
  TD-1: it is a local time-lock when no positive delay is admissible and no
  transition identity is enabled there, and replay SHALL then settle
  `reproduced-with-evaluated-witness`. Otherwise, whether a positive delay
  is admissible or a transition identity is enabled, replay SHALL take the
  non-local arm.
- **Non-local time-lock.** On the non-local arm, replay SHALL explore that state's forward closure afresh,
  unreduced, under the request's limits, with the subject's timed
  exploration (FR-101 `explore` for a `Tick` source, the zone successor relation of FR-238 from the point zone for a `Dense`
  source, computed with the checker's DBM code in `fsl-replay` (FR-245),
  never EN-6's), looking for
  a quiescent state or a cycle with positive total delay. An exploration
  that completes with none SHALL settle `reproduced-with-evaluated-witness`;
  one that finds one SHALL settle `inconclusive`, `ReplayParity`; one a
  limit stops SHALL settle the item V-7.
- Replay SHALL read no path, environment variable, clock or search
  location, and SHALL give the same result for the same request and
  envelope.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-237-AC-1 | FR-236-AC-1's counterexample replays to `reproduced-with-evaluated-witness`, checking the delay 3 against `x <= 3` and `x <= T` and the guards `x >= 3` and `x >= 1`. `Settles` with `eventually[0 ms, 3 ms)` over `Rpc` with `T = 4 ms` is refuted by `send` at 0 then `reply` after delay 3, and that counterexample reproduces. | Test (TC-692) |
| FR-237-AC-2 | Refusals settle no result: the `timeout` delay changed to `5/2` (guard `x >= 3` fails); the delay changed to `7/2` (`x <= 3` forbids it); a lasso whose loop has total delay 0; a lasso whose loop does not return to its entry clock values. | Test (TC-692) |
| FR-237-AC-3 | The strict-guard variant's local time-lock counterexample reproduces; the same payload replayed against the variant with `reply` guarded by `x >= 3 ms` settles `inconclusive`, `ReplayParity`, since `reply` is enabled at the last state, so replay takes the non-local arm and its exploration reaches the quiescent `Replied` state. | Test (TC-692) |
| FR-237-AC-4 | The `Stall` model's non-local time-lock counterexample (stem empty, `final_delay` `1/2`) reproduces by fresh exploration finding no quiescent state and no cycle with positive delay; the same payload against a variant with `ping` resetting `x` settles `inconclusive`, `ReplayParity`; with an exploration limit of one state it settles V-7. The `Stall` payload with `final_delay` 1, whose last state `x = 1` admits no positive delay while `ping` is enabled, takes the non-local arm and reproduces. Replaying one envelope twice gives equal results. | Test (TC-692) |
| FR-237-AC-5 | FR-239-AC-5's counterexample replays to `reproduced-with-evaluated-witness`; with its last step removed it settles `inconclusive`, `ReplayParity`. | Test (TC-710) |
| FR-237-AC-6 | Over a `Lock` model (one object with `phase: {A, B}` and a clock `x` never reset; `go` moves `A` to `B` with no guard; `time invariant when phase = B { x <= 1 ms }`; `ping`, enabled in `B`, changes no data field), the counterexample `go` after delay 0 to `always holds(c.phase = A)` settles `inconclusive`, `ReplayParity`, because the replayed state `(B, x = 0)` has no time-divergent continuation; over the variant in which `ping` resets `x`, the same counterexample settles `reproduced-with-evaluated-witness`; with an exploration limit of one state it settles V-7. | Test (TC-692) |
| FR-237-AC-7 | The existing `ModelTrace` replay entry rebuilds an FR-249 task product and passes its model-trace payload to the FSpec FR-364 reader; an accepted task-derived miss yields the existing FR-072 ModelTrace result, while a rejected payload does not reproduce. No `TaskAutomatonTrace` result arm or parallel FSL trace wire is emitted. | Test (TC-704) |
| FR-237-AC-8 | The FR-249-AC-9 counterexample with `loop` emptied is refused `malformed_wire` at `/loop` before any exploration, and is never settled by the admitted-endpoint arm, although its last state reaches a positive-delay cycle. | Test (TC-704) |
| FR-237-AC-9 | The FR-249-AC-14 counterexample replays through `ModelTrace` to `reproduced`. Re-admitted against the same package with the `mode` declaration removed, it refuses `invalid_runtime_input`/`invalid-value` at the `go_hi` step's `/post_state` (`/prefix/10/post_state`), because the recomputed post-state key is in `LO`; with that step's key also rewritten to the `LO` key, it refuses at the first `budget-change` step's `/transition`. With `"mode": "hi"` added to that step, it refuses `malformed_wire` at `/prefix/<i>/mode`, and added to its `transition`, at `/prefix/<i>/transition/mode`. | Test (TC-704) |
| FR-237-AC-10 | The FR-249-AC-9 refutation's counterexample (FSpec FR-364-AC-23's fixture `B`, with `<a>` the index of its `activate t1#0` step), written to JSON by FR-286's writer, reads back with `read_temporal_counterexample::<TaskProductTransition>`, and re-writing gives an equal JSON value. Through `replay_model_trace_document` it is `reproduced`. The analyze run that produced the refutation replays it through `replay_model_trace_document`: a run whose written counterexample fails that reader does not settle `refuted`. Each mutation refuses `malformed_wire` before any step replays, with the cause and pointer shown: a `phase` member on the miss step's `transition` (`unknown-member`, `/prefix/<miss>/transition/phase`); `action` `"overflow"` (`unknown-variant`, `/prefix/<miss>/transition/action`); `budget_delta` on `activate t1#0` (`budget-delta-presence`, `/prefix/<a>/transition/budget_delta`); `delay` removed from `activate t1#0` (`missing-member`, `/prefix/<a>/delay`); its `job` as the string `"0"` (`wrong-value-kind`, `/prefix/<a>/transition/job`). Embedded as `counterexamples[0]` of its FR-331 result, each pointer is prefixed `/counterexamples/0/counterexample`. | Test (TC-704) |
| FR-237-AC-11 | Each of these refuses `invalid_runtime_input`/`invalid-value`, and none is `reproduced`. Through the document form, the FR-249-AC-9 counterexample with its `activate t1#0` step, at index `<a>`, written as the source identity `M::Call::fire_t1` refuses at `/prefix/<a>/transition/type` (FSpec FR-364-AC-23). Its source projection (every task step dropped, and each activating move written as its `transition` step) refuses at its first step's `/prefix/0/transition/type`, although `replay_model_trace` would find each of its steps enabled in the source model alone. The same projection passed as a typed `TemporalCounterexample<ModelTransition>` to `replay_model_trace` for the task item refuses `ModelTraceRefusal::TraceAlphabet` before any step replays. FR-237-AC-1's `Rpc` counterexample with a `task-transition` step inserted refuses at that step's `/transition/type`. Passed to `replay_task_product_trace`, it refuses `TraceAlphabet`. | Test (TC-704) |
| FR-237-AC-12 | Through the document form, each FSpec FR-364-AC-24 tamper of the FR-249-AC-9 counterexample refuses `invalid_runtime_input`/`invalid-value` at its named member, naming the supplied and the recomputed value. FSpec FR-364-AC-23's fixture `Tick` is a `time dense` model `M` whose object type `Call` has `fired: Bool`, initially false, and one operation `tick` guarded `not self.fired` with post `self.fired = true`; `taskset Two { task a sporadic 10 ms deadline 2 ms bcet 3 ms wcet 3 ms priority 1 lo; task b sporadic 10 ms deadline 10 ms bcet 1 ms wcet 1 ms priority 2 lo; }`, with `release Two.b on M::Call::tick;` declared before `release Two.a on M::Call::tick;`; and `schedulable TwoFp on Two under fixed_priority;` with `backend` `en-6`. The item is refuted. Its counterexample's prefix is `activate a#0` (0) and `activate b#0` (1) at 0, both with the `tick` move's post-state key, then `dispatch a#0` at 0 (2), then `deadline-miss a#0` at elapsed 2 (3, the `trace_position`) with `demand` 3 and `deadline` 2. The prefix continues to `complete b#0` at elapsed 4, and `loop` is one `terminal-stutter` step with positive delay. Through the document form it is `reproduced`. Each AC-23 mutation of that move refuses at its stated pointer. Declaring the two releases in the other order writes the same steps. | Test (TC-704) |
| FR-237-AC-13 | For a release-backed schedulability item analyzed from an admitted checked package read from its bytes (FR-850, FR-940), replay derives `TaskProductTransition` from the resolved item as the document form does in AC-11; it does not accept the source-only `ModelTransition` projection. Passing that projection to the byte-route replay refuses `invalid_runtime_input`/`invalid-value` with `ModelTraceRefusal::TraceAlphabet` naming the required and supplied alphabets, before any step replays, and the counterexample is not reproduced. The matching `TaskProductTransition` is replayed under the same task-product alphabet. | Test (TC-704) |

## Dependencies

- ADR-026 §7 CT-3, §4 TD-3; ADR-018 CX-3 and SM-8; ADR-022 GX-3.
- [FR-128](FR-128-replay-a-model-counterexample.md),
  [FR-098](FR-098-execute-a-replay-request.md),
  [FR-101](FR-101-explore-finite-models-with-canonical-order-and-the-fspec-sampler.md),
  [FR-231](FR-231-read-a-timed-subject-s-behaviours-as-timed-traces.md),
  [FR-232](FR-232-derive-the-time-lock-freedom-item-and-read-deadlocks-over-time.md),
  [FR-236](FR-236-carry-exact-rational-delays-in-a-timed-counterexample.md).

## References

- FSpec half: FSpec FR-416 and FR-417, with FSpec FR-181 (Linear
  STD-139), own the replay rules of a timed counterexample and of the
  time-lock trap on the wire (ADR-026 OV-6, OV-9).
