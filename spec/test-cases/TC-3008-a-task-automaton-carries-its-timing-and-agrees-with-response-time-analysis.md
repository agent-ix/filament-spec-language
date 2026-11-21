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
and deadlines, that EN-7 checks all four work/policy cases of the retimed
three-task set `BReduced`, that EN-6 agrees on its work-4 fixed-priority
refutation with reproduced replay, that an unschedulable set
yields a replayed counterexample whose first deadline miss is exact, and
that a non-positive period is refused at check. Scope: FR-249-AC-7 to
FR-249-AC-10.

## Test Procedure

Fixtures: a `time dense` model with one always-enabled operation per task;
task set `Pair` (`a` periodic 4 ms, deadline 3 ms, work 1, priority 1; `b`
sporadic 6 ms, deadline 5 ms, work 2, priority 2); task set `B` with
`(C, T = D)` of `t1 (1, 4)`, `t2 (2, 6)`, `t3 (3, 10)` ms and the variant
with `t3` work 4; each task bound by `release S.τ on M::o;`. Every task has
`bcet` equal to `wcet`. `BReduced` keeps these three LO tasks, their work,
priorities 1 to 3, zero jitter and blocking, and separate always-enabled
release operations, but retimes periods and deadlines to `(6, 6, 8)` ms.

This retiming minimizes the maximum period in the integer-ms domain with
unchanged work and all four EN-7 policy/work outcomes preserved: if every period
were at most 7, work-4 EDF utilization would be at least `7/7`; equality
requires all three periods to be 7, where fixed priority also proves, losing
the required refutation. Maximum period 8 is therefore minimal. The
hyperperiod is 24 ms versus the original 60 ms. This is retiming, not uniform
or componentwise period downscaling; no smaller region quotient is assumed.

1. Analyze `schedulable PairFp on Pair under fixed_priority;` unnamed and
   with `backend` `en-6`; read both candidate sets, its certificate's
   `task_product.queue_bounds` and the lowered subject's task-timing rows.
   Repeat with the `release` declarations removed, naming `en-6`.
2. Analyze `schedulable BFp on B under fixed_priority;` and
   `schedulable BEdf on B under edf;` on EN-7; repeat for the work-4 variant.
   Original `B` has no EN-6 baseline measurement in this test procedure.
   Analyze the corresponding four `BReduced` work/policy cases on EN-7.
   Analyze only the `BReduced` work-4 fixed-priority claim on EN-6 with
   ordinary defaults, and read its reproduced counterexample replay.
3. Read the refuted EN-6 item of the original `B` work-4 variant under
   `fixed_priority`: its counterexample prefix, `trace_position`, `loop`,
   `queue_growth` and replay through the FR-237 `ModelTrace` entry. Replay
   each variant: the miss step's `deadline` 11; its `demand` 3; `deadline`
   11 on every `t3` job 0 step; `deadline` 81 on the loop's `activate` of
   `t3` job 7; the last loop step removed; `trace_position` on the following
   `activate`; a `deadline-miss` of `t3` job 1 inserted into the loop at
   elapsed 20; the loop replaced by one `terminal-stutter` step of delay 1;
   `loop` emptied. Rerun with the
   `fire_*` operations declared in reverse order.
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
  original `B` remains an EN-7 acceptance fixture. EN-6 retains its configured
  default limits; no completed EN-6 baseline or first exhausted bound is
  claimed for it. For original work 4 EN-7 returns `Miss` for
  `t3`, job 0, iterates `4, 7, 10, 11`, and EDF `Schedulable` at `59/60`.
  On `BReduced`, work 3 has fixed-priority fixpoints `(1, 3, 6)`, `t3`
  iterates `3, 6, 6`, and EDF utilization `7/8`; both EN-7 claims prove.
  Work 4 has fixed-priority `Miss` for `t3`, job 0, iterates `4, 7, 10`
  exceeding deadline 8, and EDF `Schedulable` at utilization 1. EN-6 refutes
  work-4 fixed priority, equals that EN-7 label, and its replay is reproduced.
  The other three `BReduced` EN-6 agreements are not asserted: the default
  checker work budget is 100000000 and there is no default wall-time deadline.
  No EN-6 verdict or typed limit stop is claimed for those three cases.
  The original work-4 fixed-priority refutation in step 3 remains a valid
  early concrete refutation.
- Step 3: `prefix[trace_position]` is the earliest miss of the product:
  `deadline-miss` of `t3`, job 0, `demand` 4, `deadline` 10, at time 10
  with completed service 3, every task activated at 0, and no other
  `deadline-miss` step in the prefix or loop. The prefix's last step is
  `activate` of `t3` job 1 at time 10; `loop` is non-empty with delays
  summing to exactly 60; `queue_growth` is absent. The same miss and loop
  elapsed time are reported with the source operations declared in reverse
  order. Replay reproduces. The variants refuse
  `invalid_runtime_input`/`invalid-value` at, in order,
  `/prefix/<miss>/transition/deadline`, `/prefix/<miss>/transition/demand`,
  the `activate` step's `/transition/deadline`,
  `/loop/<j>/transition/deadline`, `/loop/<last>/post_state`,
  `/trace_position`, the inserted miss step's `/loop/<j>/transition` and
  `/loop/0/transition`; the
  emptied loop is refused `malformed_wire` at `/loop`. No variant settles
  `refuted`.
- Step 4: each check refuses `invalid_timed_model`/`invalid-task-set`
  naming `p` at its period quantity, with no `missing-name` for the
  release; the analyze request is refused as a whole at check and no engine
  runs.
