---
id: TC-3008
title: "A task automaton carries its timing and agrees with response-time analysis"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-249
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-246
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-247
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-633
    type: verifies
---
# TC-3008: A task automaton carries its timing and agrees with response-time analysis

## Description

Verify that the lowered task-product subject carries the declared periods
and deadlines, that EN-6's verdict on a task automaton equals EN-7's
response-time verdict on the three-task set `B`, that an unschedulable set
yields a replayed counterexample whose first deadline miss is exact, and
that a non-positive period is refused at check. Scope: FR-249-AC-7 to
FR-249-AC-10.

## Test Procedure

Fixtures: a `time dense` model with one always-enabled operation per task;
task set `Pair` (`a` periodic 4 ms, deadline 3 ms, work 1, priority 1; `b`
sporadic 6 ms, deadline 5 ms, work 2, priority 2); task set `B` with
`(C, T = D)` of `t1 (1, 4)`, `t2 (2, 6)`, `t3 (3, 10)` ms and the variant
with `t3` work 4; each task bound by `release S.τ on M::o;`. Every task has
`bcet` equal to `wcet`.

1. Analyze `schedulable PairFp on Pair under fixed_priority;` unnamed and
   with `backend` `en-6`; read both candidate sets, its certificate's
   `task_product.queue_bounds` and the lowered subject's task-timing rows.
   Repeat with the `release` declarations removed, naming `en-6`.
2. Analyze `schedulable BFp on B under fixed_priority;` and
   `schedulable BEdf on B under edf;` once on EN-7 and once with `backend`
   `en-6`; repeat for the work-4 variant.
3. Read the refuted EN-6 item of the work-4 variant under
   `fixed_priority`: its counterexample prefix, first `deadline-miss` step
   and replay. Change the miss step's `deadline` to 11, then its `demand`
   to 3, and replay each. Rerun with the `fire_*` operations declared in
   reverse order.
4. Check `taskset Bad` whose task `p` has `periodic 0 ms`, then
   `periodic -4 ms`, each with `release Bad.p on M::o;`; analyze
   `schedulable BadFp on Bad under fixed_priority;` with `backend` `en-6`.

Tag tests `#[trace("TC-3008", "FR-249-AC-7")]` to
`#[trace("TC-3008", "FR-249-AC-10")]`.

## Expected Results

- Step 1: unnamed candidates `[en-7]`, named `[en-6]`; the EN-6 item
  proves; the rows,
  in order, are `(Pair, a, periodic, 4, 3, 0, 1, 1, 2)` and
  `(Pair, b, sporadic, 6, 5, 0, 2, 2, 2)` as `(taskset, task, arrival,
  interarrival, relative_deadline, max_jitter, work_lower, work_upper,
  max_jobs)`, equal in the subject and the certificate. Without releases the
  item naming `en-6` has an empty candidate set and settles `unsupported`.
- Step 2: EN-7 returns `Schedulable` with fixpoints `(1, 3, 10)` (`t3`
  iterating `3, 6, 7, 9, 10, 10`) and, under EDF, utilization `53/60`;
  EN-6 proves both. For the variant EN-7 returns `Miss` for `t3`, job 0,
  iterates `4, 7, 10, 11`, and EDF `Schedulable` at `59/60`; EN-6 refutes
  `fixed_priority` and proves `edf`. Each EN-6 label equals EN-7's.
- Step 3: the first `deadline-miss` step in step order is the earliest
  miss of the product: `t3`, job 0, `demand` 4, `deadline` 10, at time 10
  with completed service 3, every task activated at 0, and no earlier miss.
  The same miss is reported with the source operations declared in reverse
  order. Replay reproduces; both mutations refuse replay.
- Step 4: each check refuses `invalid_timed_model`/`invalid-task-set`
  naming `p` at its period quantity, with no `missing-name` for the
  release; the analyze request is refused as a whole at check and no engine
  runs.
