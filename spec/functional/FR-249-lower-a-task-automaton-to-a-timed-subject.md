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

- A checked timed model, task set, `release S.τ on M::o;` declarations and
  scheduling policy. Release binding, arrival spacing, jitter, task/job
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
  release. For such a claim the candidate set SHALL be `["en-7", "en-6"]`
  in that order: EN-7 keeps deciding the closed-form question over the task
  set alone (FR-247), and an analyze item naming the claim with `backend`
  `en-6` (FR-620) runs this lowering and EN-6's search over the product of
  the releasing model and `S` under `P`. A claim over a task set with no
  checked release keeps FR-246's candidate set `["en-7"]`. Both engines then
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
  demand. FR-237 SHALL replay the prefix and the time-divergent
  continuation against the rebuilt product before the item settles
  `refuted`; a prefix whose miss step's `deadline` or `demand` disagrees
  with the recomputed job refuses replay.

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
| FR-249-AC-4 | The lowered subject and replay preserve the FSpec FR-419/FR-364 mode-change transition identity and captured job data. If the current FR-418 reader does not accept the mixed-criticality product as exact, a bounded run cannot settle `Holds` and settles incomplete with FR-331 `task_search` evidence when its limit is reached. | Test |
| FR-249-AC-5 | For the periodic-source overload task, whose model repeats the release transition every `T=1` (`W=2`, `D=1`), under each admitted `fixed_priority` and `edf` policy, a refutation requires a concrete missed-job prefix followed by FSpec FR-364's replay-checked `queue_growth` continuation with positive elapsed time. Queue-cap overflow or a finite queue loop without the queue-growth identity induction does not refute. A task-search stop with an unexplored frontier replays as incomplete under FR-331. | Test |
| FR-249-AC-6 | Two task releases bound to one operation are recorded in source order; a duplicate release of one task on that operation refuses `conflicting_declaration`/`duplicate-declaration`; unresolved task set, task or operation names refuse `missing_declaration`/`missing-name`; an ambiguous inherited operation refuses `ambiguous_declaration`/`ambiguous-name`; a `release` on a model without `time dense` is admitted by checking and refused only when lowering is requested. | Test |
| FR-249-AC-7 | Task set `Pair` in `ms`: `a` periodic 4, deadline 3, `bcet` = `wcet` = 1, priority 1; `b` sporadic 6, deadline 5, `bcet` = `wcet` = 2, priority 2; each released by its own always-enabled operation of a `time dense` model. `schedulable PairFp on Pair under fixed_priority;` has candidates `["en-7", "en-6"]`, and its item with `backend` `en-6` proves with an accepted certificate. The lowered subject's task-timing rows, and the certificate's `task_product.queue_bounds`, are in order `(Pair, a, periodic, interarrival 4, relative_deadline 3, max_jitter 0, work 1..1, max_jobs 2)` and `(Pair, b, sporadic, 6, 5, 0, 2..2, 2)`. The same set with no `release` keeps candidates `["en-7"]`. | Test |
| FR-249-AC-8 | Task set `B` in `ms` (Buttazzo's fixed-priority response-time example), `(C, T = D)` of `t1 (1, 4)`, `t2 (2, 6)`, `t3 (3, 10)`, `bcet` = `wcet`, priorities 1 to 3, each task released by its own always-enabled operation. EN-7 returns `Schedulable` under `fixed_priority` with fixpoints `(1, 3, 10)`, `t3` iterating `3, 6, 7, 9, 10, 10`, and under `edf` with utilization `53/60`; EN-6 proves both claims. With `t3`'s `bcet` = `wcet` = 4, EN-7 returns `Miss` for `t3`, job 0, iterates `4, 7, 10, 11` under `fixed_priority` and `Schedulable` (utilization `59/60`) under `edf`; EN-6 refutes the first and proves the second. For all four claims the EN-6 item's `proved`/`refuted` label equals the EN-7 item's. | Test |
| FR-249-AC-9 | The refuted EN-6 item of `B` with `t3` work 4 under `fixed_priority` carries a model-arm counterexample whose first `deadline-miss` step names `t3`, job 0, `demand` 4, and an absolute `deadline` 10 after `t3`'s activation; no `t1` or `t2` miss precedes it, and `t3`'s completed service at that step is below 4. In the least-delay concretisation (FR-241), every task activates at 0 and the miss is at time 10 with completed service 3. FR-237 replays the prefix and its time-divergent continuation (`reproduced` true); the same prefix with the miss's `deadline` changed to 11, or its `demand` to 3, refuses replay. | Test |
| FR-249-AC-10 | `taskset Bad { task p periodic 0 ms deadline 4 ms wcet 1 ms priority 1 lo; }` with `release Bad.p on M::o;` refuses `invalid_timed_model`/`invalid-task-set` naming `p` at `0 ms`, with no `missing-name` refusal for the release; `periodic -4 ms` refuses the same at its period quantity; an analyze request naming `schedulable BadFp on Bad under fixed_priority;` with `backend` `en-6` is refused as a whole at check and runs no engine. | Test |

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
- G. C. Buttazzo, *Hard Real-Time Computing Systems* (Springer), chapter 4,
  response-time analysis of fixed-priority periodic tasks; the source of the
  task set `B` used to compare EN-6 with EN-7.
- M. Joseph and P. Pandya, "Finding response times in a real-time system"
  (The Computer Journal 29(5), 1986): the response-time recurrence and the
  synchronous release as its worst case.
