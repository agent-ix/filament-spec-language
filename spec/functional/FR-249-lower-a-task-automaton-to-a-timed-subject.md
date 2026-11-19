---
id: FR-249
title: "Lower a task automaton to a timed subject"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-026
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-230
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-238
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-239
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-246
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-331
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-364
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-415
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-418
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-419
    type: depends_on
---
# FR-249: Lower a task automaton to a timed subject

## Description

A timed model whose `release S.τ on M::o;` declarations bind source
transitions to tasks of a checked task set is a task automaton. S3 SHALL
compile it with the selected scheduler to an ordinary unit-rate timed product
and EN-6 SHALL check `always holds(not deadline_missed)` on that product.
The product SHALL preserve the source state and exact job identities while
representing the scheduler state and task-derived timed data. Its task
semantics, job identity, demand, release and mode-change rules, bounded
subtraction contract, and certificate/replay carriers are owned by FSpec
FR-419, FR-415, FR-418, FR-331 and FR-364. FSL owns lowering those checked
inputs into EN-6's timed subject, searching it, rebuilding it for replay, and
settling its outcome. EN-6 SHALL use ordinary unit-rate clocks and the
FSpec-defined bounded-subtraction/diagonal construction; it SHALL NOT add
stopwatch rates (ADR-026 RT-8).

The engine SHALL decide the exact fixed-work class admitted by the current
FR-418 reader, using its independently checked finite region/product
certificate. Products outside that exact gate SHALL remain admitted for
sound bounded exploration. The engine SHALL return a concrete refutation
only when FR-237 accepts the deadline-miss prefix and FSpec FR-364's checked
time-divergent continuation. If bounded exploration cannot establish either
a complete proof or such a refutation, it SHALL return `incomplete` with replayable
`task_search` evidence. These cases are admitted; undecidability means that a
run may remain unknown, not that the input is unsupported.

## Use case

An embedded engineer checks that every task released by a timed controller
meets its deadline. A proof covers all admitted task choices; a refutation
contains a realizable late job and a time-divergent continuation; a bounded
run that cannot decide supplies evidence that an independent reader can
replay.

## Inputs

- A checked timed model, task set, `release S.τ on M::o;` declarations,
  `mode S hi on M::o;` mode-switch declarations and scheduling policy. Release binding, arrival spacing, jitter, task/job
  identity, demand selection and budget-change semantics are those of FSpec
  FR-419. A fixed-work task has a singleton actual demand as defined by that
  requirement; a WCET by itself remains an upper bound and does not select
  that class.
- EN-6 exploration and certificate limits (FR-239).

## Outputs

- A task-derived timed subject and an EN-6 outcome: a checked exact
  `Holds`, a replayed `Violated`, `Undecided`, or `Stopped` outcome. A stopped
  bounded task search carries FSpec FR-331 `task_search` frontier evidence;
  it does not produce `Holds`.

## Behavior

- `release S.τ on M::o;` is the model-level declaration of FSpec's shared
  grammar, not a member of an operation. The checker SHALL resolve `S` to a
  task set of the unit, `τ` to a task of `S`, and `M::o` to an operation of a
  model, refusing `missing_declaration`/`missing-name` or
  `ambiguous_declaration`/`ambiguous-name` as applicable. It SHALL refuse a
  second declaration of the same task on the same operation as
  `conflicting_declaration`/`duplicate-declaration` at the second
  declaration. It SHALL record each checked release in
  `CheckedGraph::task_sets()` for lowering.
- `mode S hi on M::o;` is FSpec's model-level mode-switch declaration
  (shared grammar `mode-switch-decl`, FSpec FR-419 "Criticality mode"). The
  checker SHALL resolve `S` and `M::o` as it resolves a `release`, refusing
  `missing_declaration`/`missing-name` or `ambiguous_declaration`/
  `ambiguous-name` as applicable. It SHALL refuse a second `mode S hi` on the
  same operation as `conflicting_declaration`/`duplicate-declaration` at the
  second declaration. It SHALL record each checked switch in source order in
  `CheckedGraph::task_sets()` beside the releases
  (`CheckedTaskSets::mode_switches`). A mode switch does not make a claim
  release-backed (FR-246). S3 SHALL carry the checked switches of `S` on the
  product's model into the lowered subject. The product's criticality mode,
  allowance phases, budget exhaustion and `budget-change` steps are FSpec
  FR-419's, and FSL has one product semantics: the search and the replay
  rebuild the same product. FSL SHALL NOT infer the mode from a source field,
  a job's service or an operation name.
- S1 SHALL parse `mode S hi on M::o;` at declaration level as one
  `ModeSwitchDeclaration` production of the shared grammar, beside
  `ReleaseDeclaration`, and S2 SHALL build one `ModeSwitchForm { task_set,
  operation, span }` for it, spelled and unresolved like `ReleaseForm`.
  `mode` begins this declaration only at declaration level; inside a `hybrid`
  body it keeps its hybrid meaning. `hi` is a keyword spelling only here and
  in `task`. `mode S lo on M::o;`, and a declaration missing `hi`, `on`, the
  qualified operation name or the closing `;`, fail at S1 with
  `invalid_syntax`/`unexpected-token` at the first unexpected token and build
  no form. The formatter (FR-003) prints the declaration as
  `mode S hi on M::o;` on one line, and formatting is idempotent.
- When S3 lowers checked release declarations and a task set into the EN-6
  task-derived subject, S3 SHALL consume the FSpec FR-419 product contract and
  preserve source state, FSpec task/job identities, and derived-step
  references in the subject and counterexample. FSL SHALL NOT define competing
  task identity, arrival, demand, service, budget-change, subtraction,
  certificate, or replay contracts.
- EN-6 SHALL implement the FSpec FR-415 bounded-subtraction/diagonal
  transition semantics using unit-rate clocks, then apply FSpec FR-418's
  finite-product certificate gate. The FSL-specific obligations are to build
  the subject accepted by that reader, pass it through the existing EN-6
  search and provide the same subject to FR-237 for replay. FSL SHALL NOT add
  stopwatch rates or infer support from the ordinary aLU model-subject path.
- The exact fixed-work class is the class admitted by the current FSpec
  FR-418 reader. An accepted complete certificate with no deadline miss
  settles `Holds`; a concrete miss settles `Violated` only after FR-237
  accepts FSpec FR-364's deadline-miss prefix and time-divergent continuation.
  Queue-cap overflow alone SHALL NOT settle a miss.
- Inputs outside the current FR-418 exact gate, including variable/interval
  demand and mixed-criticality budget changes, SHALL remain admitted and use
  sound bounded exploration. The run SHALL settle `Holds` only when the
  checked certificate covers the complete admitted task-product state space;
  it SHALL settle `Violated` only after replay verifies the concrete miss and
  continuation. Otherwise the provider SHALL return `Stopped` with FSpec
  FR-331 `task_search` evidence, which FR-235 settles as incomplete. FSL SHALL
  use FR-331's and FR-364's readers and wire contracts rather than defining
  task-frontier or derived-continuation schemas of its own.
- The checker SHALL continue to admit a `release` declaration on a model
  without a `time dense` source. When S3 is asked to lower that model, the
  task automaton lowering SHALL return the existing typed timed-model
  refusal before invoking EN-6.
  For every admitted dense-time task product, the lowering SHALL preserve
  FR-231's scope: only
  time-divergent behaviours can refute the timed invariant, and FR-417
  time-lock outcomes remain separate.

### Requesting the product check

- The schedulability item of a task automaton is the `schedulable N on S
  under P;` claim (FR-246) over a task set `S` that has at least one checked
  release; its requirement record is release-backed (FR-246). FR-075 makes
  the matching set of a release-backed item `[en-6, en-7]` and FR-962 narrows
  it to `[en-7]` when the item names no backend: EN-7 keeps deciding the
  closed-form question over the task set alone (FR-247) by default, and an
  analyze item naming the claim with `backend` `en-6` (FR-620) runs this
  lowering and EN-6's search over the product of the releasing model and `S`
  under `P`. A claim over a task set with no checked release is not
  release-backed, its matching set is `[en-7]`, and naming `en-6` for it gives
  an empty candidate set. Both engines then
  answer one question for one task set, so their verdicts can be compared
  directly.

### Task timing in the lowered subject

- The lowered subject SHALL carry one task-timing row per task of `S`, in
  the task set's source order, whose members are FSpec FR-418's
  `task_product.queue_bounds` members: `taskset` and `task` (names);
  `arrival` (`periodic` or `sporadic`, the FR-246 checked arrival kind);
  `interarrival` (the checked period `T`, or minimum inter-arrival time);
  `relative_deadline` (the checked `D`); `max_jitter` (the checked `J`,
  zero when omitted); `work_lower` and `work_upper` (the checked `BCET` and
  applicable WCET, equal for a fixed-work task); and `max_jobs`
  (`ceil(D / T) + 1`, FSpec FR-419). Every time member is the exact FR-246
  value in the task set's unit, typed as an exact rational. An accepted
  zone certificate for the product carries exactly these rows, so a reader
  can confirm that the product checked the declared periods and deadlines.
- Every `activate` step of a task-derived counterexample or replayed prefix
  SHALL carry, in its FSpec FR-364 `TaskTransitionIdentity`, the absolute
  `deadline` equal to that activation's time plus the task's `D`; two
  successive activations of one periodic task SHALL be exactly `T` apart.

### Agreement with response-time analysis

- For a fixed-work task set with `D <= T`, no jitter or blocking, each task
  released by its own source operation that is always enabled, the product
  contains the synchronous release, which is the critical instant of
  classic response-time analysis (Liu and Layland; Joseph and Pandya). EN-6's
  `Holds`/`Violated` verdict on such a product SHALL therefore equal EN-7's
  `Schedulable`/`Miss` verdict for the same claim under both
  `fixed_priority` and `edf`. A disagreement is a defect in one of the two
  engines, never an accepted outcome.

### The unschedulable counterexample

- A refuted product item SHALL carry FSpec FR-364's model-arm
  `TemporalCounterexample`. Its prefix lists source transitions and typed
  `TaskTransitionIdentity` steps with their exact rational delays (FR-237),
  and its first `deadline-miss` step is the first missed deadline of the
  behaviour: it names the task, the job occurrence, the job's exact
  `demand` and its absolute `deadline`, and no earlier step is a
  `deadline-miss`. At that step the job's completed service is below its
  demand. The reported miss SHALL be the earliest deadline miss of the
  product, not whichever miss the search reaches first: among all
  behaviours that miss a deadline, the prefix SHALL reach its first
  `deadline-miss` step at the least elapsed time, ties broken by the fewest
  steps and then by the bytewise order of the miss step's (`task`, `job`)
  identity; the prefix's steps before it are in step order with FR-241's
  least admissible delays. So the reported task, job and absolute deadline
  are a function of the claim and the product, independent of search order.
  FR-237 SHALL replay the prefix and the time-divergent
  continuation against the rebuilt product before the item settles
  `refuted`; a prefix whose miss step's `deadline` or `demand` disagrees
  with the recomputed job refuses replay.
- The item SHALL carry exactly one FSpec FR-364 continuation after the miss.
  EN-6 SHALL take the loop entry at the earliest position at or after the
  miss from which a closed positive-delay loop, or a positive-delay
  terminal-stutter loop from a quiescent product-terminal state, exists. Ties
  go to the least loop elapsed time, then the fewest steps, with FR-241 least
  delays. EN-6 SHALL emit `queue_growth` only when no finite loop closes.
  Miss candidates are taken in earliest-miss order. One whose continuations
  are all time-locked is skipped (FR-417 reports the time-lock). If a limit
  stops the search while an earlier candidate is undecided, the item SHALL
  settle `incomplete` with FSpec FR-331 `task_search` evidence and SHALL
  report no later miss, so the reported miss stays a function of the claim,
  the product and the limits. A finite prefix with no continuation is never
  a refutation. The continuation is found from the FR-418 closed graph or by
  forward exploration under the same work budget, and is always written on
  the wire; replay never settles `refuted` from an exploration of its own.
  The product's miss latch is set at the reported miss, so no later step of
  the prefix, loop or `queue_growth` stem is a `deadline-miss`, and
  `trace_position` is the index of the reported miss step in `prefix`.

### A non-positive period

- A task whose period or minimum inter-arrival time is zero or negative is
  refused by S3 when the task set is checked, as FR-246 states:
  `invalid_timed_model`/`invalid-task-set` naming the task, at the period's
  quantity. A `release` naming that task resolves (its name is declared), so
  it adds no `missing-name` refusal. Because the unit does not check, an
  analyze request over it is refused as a whole at check (FR-624); no
  lowering runs and neither EN-6 nor EN-7 is invoked. The refusal is made at
  checking rather than at lowering so that both engines, and every other
  reader of the task set, see the same refusal for the same defect.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-249-AC-1 | The `sample`/`command` model over `Ctl`'s first two tasks under fixed priority returns `Holds` when its exact fixed-work product has an accepted finite certificate. With `a`'s exact work raised to 4 and the command deadline missed on a time-divergent behaviour, it returns `Violated` with a task-derived counterexample that FR-237 replays. | Test |
| FR-249-AC-2 | With interval actual demand, a `when done(a)` source guard and preemptive fixed-priority scheduling, the model is admitted and bounded exploration retains the chosen demand across preemption and completion feedback. A limit reached before a complete certificate or replayable miss returns `incomplete` with task-search frontier evidence; it never returns `Unsupported(StopwatchRequired)` or `Holds` from a bounded prefix. | Test |
| FR-249-AC-3 | The lowered EN-6 subject preserves FSpec FR-419's task-product state and FR-364's derived-step identity across a preemption/resumption sequence; FR-237 replays the resulting existing `ModelTrace` arm against the rebuilt product. | Test |
| FR-249-AC-4 | The lowered subject and replay keep FSpec FR-419's criticality mode, allowance phases, budget-exhaustion `preempt` and `budget-change` steps (FR-249-AC-13 to AC-15). A mixed-criticality product with unequal budgets is outside the exact gate. A bounded run of it does not settle `Holds`, and when its limit is reached it settles incomplete with FR-331 `task_search` evidence. | Test |
| FR-249-AC-5 | For the periodic-source overload task, whose model repeats the release transition every `T=1` (`W=2`, `D=1`), under each admitted `fixed_priority` and `edf` policy, a refutation requires a concrete missed-job prefix followed by FSpec FR-364's replay-checked `queue_growth` continuation with positive elapsed time. Queue-cap overflow or a finite queue loop without the queue-growth identity induction does not refute. A task-search stop with an unexplored frontier replays as incomplete under FR-331. The refutation's `prefix` ends at the miss step, `loop` is empty, and `queue_growth` replays. The witness with `elapsed` 0, with `iteration_shift` 0, with `backlog_growth` 2, with a growth-block template `demand` changed from 2 to 1, or with two `identity_map` entries sharing one destination each refuses `invalid_runtime_input`/`invalid-value` at its `/queue_growth/...` pointer (`/queue_growth/elapsed`, `/queue_growth/iteration_shift`, `/queue_growth/backlog_growth`, `/queue_growth/before/growth_block/0/demand`, `/queue_growth/identity_map/<second>`). The same witness with a finite three-cycle `loop` in place of `queue_growth` does not close (the live queue differs) and refuses at `/loop/<last>/post_state`. | Test |
| FR-249-AC-6 | Two task releases bound to one operation are recorded in source order; a duplicate release of one task on that operation refuses `conflicting_declaration`/`duplicate-declaration`; unresolved task set, task or operation names refuse `missing_declaration`/`missing-name`; an ambiguous inherited operation refuses `ambiguous_declaration`/`ambiguous-name`; a `release` on a model without `time dense` is admitted by checking and refused only when lowering is requested. | Test |
| FR-249-AC-7 | Task set `Pair` in `ms`: `a` periodic 4, deadline 3, `bcet` = `wcet` = 1, priority 1; `b` sporadic 6, deadline 5, `bcet` = `wcet` = 2, priority 2; each released by its own always-enabled operation of a `time dense` model. `schedulable PairFp on Pair under fixed_priority;` is release-backed: unnamed its candidate set is `[en-7]`, and its item with `backend` `en-6` has candidates `[en-6]` and proves with an accepted certificate. The lowered subject's task-timing rows, and the certificate's `task_product.queue_bounds`, are in order `(Pair, a, periodic, interarrival 4, relative_deadline 3, max_jitter 0, work 1..1, max_jobs 2)` and `(Pair, b, sporadic, 6, 5, 0, 2..2, 2)`. The same set with no `release` is not release-backed and its item naming `en-6` has an empty candidate set. | Test |
| FR-249-AC-8 | Task set `B` in `ms`, `(C, T = D)` of `t1 (1, 4)`, `t2 (2, 6)`, `t3 (3, 10)`, `bcet` = `wcet`, priorities 1 to 3, each task released by its own always-enabled operation. EN-7 returns `Schedulable` under `fixed_priority` with fixpoints `(1, 3, 10)`, `t3` iterating `3, 6, 7, 9, 10, 10`, and under `edf` with utilization `53/60`; EN-6 proves both claims. With `t3`'s `bcet` = `wcet` = 4, EN-7 returns `Miss` for `t3`, job 0, iterates `4, 7, 10, 11` under `fixed_priority` and `Schedulable` (utilization `59/60`) under `edf`; EN-6 refutes the first and proves the second. For all four claims the EN-6 item's `proved`/`refuted` label equals the EN-7 item's. | Test |
| FR-249-AC-9 | The refuted EN-6 item of `B` with `t3` work 4 under `fixed_priority` carries a model-arm counterexample whose `prefix[trace_position]` is the product's earliest deadline miss: `t3`, job 0, `demand` 4, absolute `deadline` 10, at elapsed time 10 with completed service 3, every task having activated at time 0. No earlier step is a `deadline-miss`, and no later step of the prefix or loop is one. No behaviour misses earlier, since `t1` and `t2` meet every deadline and a `t3` miss needs 10 ms after its activation. The prefix's last step is `activate` of `t3` job 1 at elapsed 10. `loop` is non-empty, its exact delays sum to 60 (the schedule repeats with period 60, since utilization is `59/60` and the queue empties at 59), it holds no `deadline-miss`, and `queue_growth` is absent. Rerunning with the source operations declared in reverse order reports the same task, job, deadline, time and loop elapsed time. Through the FR-237 `ModelTrace` replay entry the counterexample is `reproduced`. These variants refuse `invalid_runtime_input`/`invalid-value` at the named pointer: miss `deadline` 11 (`/prefix/<miss>/transition/deadline`); miss `demand` 3 (`/prefix/<miss>/transition/demand`); `deadline` 11 on every `t3` job 0 step (its `activate` step's `/transition/deadline`); the loop's `activate` of `t3` job 7 with `deadline` 81 (`/loop/<j>/transition/deadline`); last loop step removed (`/loop/<last>/post_state`); `trace_position` moved to the following `activate` step (`/trace_position`); a `deadline-miss` step of `t3` job 1 inserted into the loop at elapsed 20, where the latch disables it (at that step, `/loop/<j>/transition`); the loop replaced by one `terminal-stutter` step of delay 1, whose entry holds live jobs (`/loop/0/transition`). With `loop` emptied the counterexample is refused `malformed_wire` at `/loop`. | Test |
| FR-249-AC-10 | `taskset Bad { task p periodic 0 ms deadline 4 ms wcet 1 ms priority 1 lo; }` with `release Bad.p on M::o;` refuses `invalid_timed_model`/`invalid-task-set` naming `p` at `0 ms`, with no `missing-name` refusal for the release; `periodic -4 ms` refuses the same at its period quantity; an analyze request naming `schedulable BadFp on Bad under fixed_priority;` with `backend` `en-6` is refused as a whole at check and runs no engine. | Test |
| FR-249-AC-11 | Model `Once` (`time dense`, `fired: Bool` initially false, operation `go` guarded `not self.fired` with post `self.fired = true`, no other operation, no time invariant), `taskset S { task s sporadic 10 ms deadline 2 ms bcet 3 ms wcet 3 ms priority 1 lo; }`, `release S.s on M::go;`, `schedulable SOnce on S under fixed_priority;` with `backend` `en-6`: refuted. `prefix[trace_position]` is `deadline-miss` of `s` job 0 at elapsed 2 with `demand` 3 and `deadline` 2 (`trace_position` 2, after `activate` and `dispatch` of `s` job 0 at 0). The prefix continues to `complete` of `s` job 0 at elapsed 3. `loop` is one `terminal-stutter` step with positive delay. Replay reproduces. With the stutter delay 0 it refuses at `/loop/0/delay`. The same record replayed against variant `OnceAgain`, whose `go` has no guard and whose `s` is `sporadic 1 ms` (so the bound source operation is enabled at the loop entry, elapsed 3, arrival clock 3 >= 1), refuses `invalid_runtime_input`/`invalid-value` at `/loop/0/transition`, because `go` is enabled at the loop entry. Variant `OnceLocked` adds clock `y`, never reset, and `time invariant when self.fired { y <= 3 ms }`. There every continuation from the miss is time-locked, so the item does not refute: it proves on the FR-418 closed certificate (the miss node is `locked`), and the FR-417 time-lock item over the same model refutes. | Test |
| FR-249-AC-12 | The AC-9 claim run with `temporal.limits.max_task_product_work` set to the expansion count at which the run expands the miss node, plus one, settles `incomplete`, `limit-reached{limit: work-limit}`, with FSpec FR-331 `task_search` evidence whose nodes include the miss state. It carries no `counterexamples` entry and is never `refuted`. The budget is read from a full run's accounting, so the fixture stays exact under search changes. | Test |
| FR-249-AC-13 | Over FSpec FR-419-AC-14's model `C` and task set `MC`: `mode MC hi on C::go_hi;` checks and is recorded in `mode_switches` in source order beside `MC`'s releases. `mode Nope hi on C::go_hi;` refuses `missing_declaration`/`missing-name` naming `Nope`. `mode MC hi on C::nope;` refuses the same naming `C::nope`. A repeated `mode MC hi on C::go_hi;` refuses `conflicting_declaration`/`duplicate-declaration` at the second. `mode MC lo on C::go_hi;` is `invalid_syntax`/`unexpected-token` at `lo`. A task set with a `mode` declaration and no `release` is not release-backed. | Test (TC-704, TC-3009) |
| FR-249-AC-14 | The EN-6 item of `schedulable MCFp on MC under fixed_priority;` (FSpec FR-419-AC-14 fixture) with `backend` `en-6` is refuted. `prefix[trace_position]` is the `deadline-miss` of `h` job 0 at elapsed 10 with `demand` 3, `deadline` 10 and completed service 2. Before it, `h` job 0 has a `preempt` at elapsed 2 with delay 0 and null `source_transition`, and no later `dispatch` or `resume` before elapsed 12. The counterexample holds exactly two `budget-change` steps, at elapsed 12, directly after the `go_hi` source step: `h` job 0, then `h` job 1, each with `budget_delta` 1, delay 0 and `source_transition` `go_hi`. No `budget-change` names a job activated after 12. Every `activate` step's `source_transition` equals its `release`. The loop is non-empty, its delays sum to 10, and it holds no `budget-change`. The result is `reproduced` through the FR-237 `ModelTrace` entry, and each FSpec FR-364-AC-21 tamper refuses at its pointer. Rerunning with the source operations declared in reverse order gives the same miss and the same two `budget-change` steps. | Test (TC-704) |
| FR-249-AC-15 | `MCEarly` (`MC` with `12 ms` replaced by `2 ms` in `go_hi`'s guard and the time invariant) does not refute and does not prove: it settles `incomplete` (the exact gate needs `BCET = C(LO) = C(HI)`) with FR-331 `task_search` evidence. Its edges hold no `deadline-miss`, and every `go_hi` edge is followed by exactly one `budget-change`, of `h` job 0 with `budget_delta` 1. `MCNone` (`MCEarly` without the `mode` declaration) holds no `budget-change` in any edge or counterexample. Its first miss is `h` job 0 at 10, and it refutes with a `queue_growth` continuation whose `growth_block` is one `h` job with `allowance` 2, `remaining` 1 and `status` `preempted`, with `iteration_shift` 1 and `elapsed` 10. A product that runs a phase-`lo` `h` job past `C(LO)` finds no miss on `MCNone` and fails this criterion. | Test (TC-704) |
| FR-249-AC-16 | `mode MC hi on C::go_hi;` parses to one `ModeSwitchDeclaration` and builds `ModeSwitchForm { task_set: MC, operation: C::go_hi }` with the span of the whole declaration, `;` included, in a unit that also holds a `hybrid` declaration with `mode` entries, which keep parsing as hybrid modes. A field named `hi` parses as an identifier. Each of `mode MC lo on C::go_hi;`, `mode MC on C::go_hi;`, `mode MC hi C::go_hi;`, `mode MC hi on;`, `mode MC.h hi on C::go_hi;` and `mode MC hi on C::go_hi` (no `;`) fails at S1 with `invalid_syntax`/`unexpected-token` at `lo`, `on`, `C`, `;`, `.` and the end of input respectively, and builds no form. Formatting `mode   MC hi` and, on the next line, `  on C::go_hi ;` gives `mode MC hi on C::go_hi;`, formatting that output again gives it unchanged, and both sources compile to equal packages. | Test (TC-704) |

## Dependencies

- ADR-026 §12 RT-8 and §8 EZ-3 to EZ-5.
- [FR-230](FR-230-check-time-declarations-clocks-and-clock-constraints.md),
  [FR-237](FR-237-replay-a-timed-counterexample.md),
  [FR-238](FR-238-represent-zones-as-difference-bound-matrices.md),
  [FR-239](FR-239-check-a-timed-claim-by-symbolic-zone-search.md),
  [FR-246](FR-246-check-task-sets-and-schedulability-claims.md).

## References

- **Scope boundary.** FSpec FR-419 owns task-set, task-automaton, demand,
  activation and mode-change semantics; FR-415 owns the bounded-subtraction
  and diagonal proof contract; FR-418 owns certificate schema and exact-class
  admission; FR-331 owns incomplete task-search evidence; FR-364 owns the
  derived-step and continuation witness schema/replay rules. This FSL
  requirement defines only FSL lowering, EN-6 integration and outcome
  settlement, and does not amend those FSpec semantics or wire contracts.
- E. Fersman, P. Krčál, P. Pettersson and W. Yi, “Task automata:
  Schedulability, decidability and undecidability” (Information and
  Computation 205(8), 2007), Theorems 2 and 5. Fixed computation times admit
  decidable schedulability analysis;
  interval execution with completion feedback and preemption does not admit
  that exact result. The latter is the reason bounded search may return
  unknown, not a refusal.
- M. Joseph and P. Pandya, "Finding response times in a real-time system"
  (The Computer Journal 29(5), 1986): the response-time recurrence and the
  synchronous release as its worst case.
