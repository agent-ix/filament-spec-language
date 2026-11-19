---
id: TC-704
title: "Task automata use replayable timed products for deadline checking"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-249
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-128
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-230
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-237
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-238
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-239
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-235
    type: verifies
---
# TC-704: Task automata use replayable timed products for deadline checking

## Description

Verify fixed-work task-product decisions, paused-service replay, sound bounded
search for variable demand and mixed-criticality changes, time-divergent miss
evidence, and release admission. This replaces the former
`Unsupported(StopwatchRequired)` expectation.

Scope: FR-249-AC-1 to FR-249-AC-6, FR-249-AC-11, FR-249-AC-12,
FR-237-AC-7, FR-237-AC-8, FR-239-AC-6,
FR-235-AC-6, FR-128-AC-9, and FR-230-AC-7.

## Test Procedure

Fixtures: the `sample`/`command` model over `Ctl`'s first two tasks, an exact
fixed-work variant, an interval-demand/completion-feedback/preemptive variant,
a two-job preemption scenario, a mixed-criticality task with distinct
`C(LO)` and `C(HI)`, and the periodic overload `T=1`, `W=2`, `D=1` under fixed_priority and EDF; plus the
`release` declarations described in FR-249.

1. Analyze the exact fixed-work model with its finite product certificate;
   mutate the certificate by removing an enabled edge. Analyze the exact
   work-4 deadline-miss variant and replay its counterexample.
2. Run the interval-demand model with a finite symbolic-state limit. Check
   that the outcome carries a task-search frontier. Remove an edge, change
   the effective bound, omit a frontier node, and attempt to settle the
   bounded prefix as `Holds`.
3. Replay the preemption scenario through the FSpec FR-364 contract. Check
   that the existing ModelTrace result arm is used; mutate one derived-step
   identity and require the reader to reject it.
4. Replay an LO-to-HI source transition through the FSpec FR-364 reader. Run
   the periodic overload under both fixed_priority and EDF, requiring the
   replayed queue_growth continuation; mutate its occurrence mapping and test
   an overflow-only purported miss.
5. Check FR-128-AC-9 using the existing `ReplaySource::ModelTrace` request
   and FR-072 result type.
6. Check the release declarations: recorded releases, duplicate release,
   unresolved task set/task/operation, and ambiguous inherited operation.
7. Check that a release without the timed facet is refused by the existing
   facet gate with FR-230-AC-7, not by task-product lowering.
8. In step 4's overload refutation, replay each `queue_growth` mutation of
   FR-249-AC-5 (`elapsed` 0, `iteration_shift` 0, `backlog_growth` 2,
   growth-block template `demand` 2 to 1, two `identity_map` entries on one
   destination) and the three-cycle finite `loop` in place of the witness.
9. Analyze `SOnce` over the `Once` model of FR-249-AC-11 with `backend`
   `en-6`, replay its counterexample, then replay it with the stutter delay
   0, and replay the original record against the unguarded `OnceAgain` (`sporadic 1 ms`)
   variant. Analyze the `OnceLocked` variant and its FR-417 time-lock item.
10. Rerun the FR-249-AC-9 claim of TC-3008 with
    `temporal.limits.max_task_product_work` set to the full run's expansion
    count at the miss node plus one (FR-249-AC-12).
11. Replay the FR-249-AC-9 counterexample with `loop` emptied through the
    `ModelTrace` entry (FR-237-AC-8).

Tag executable tests with the criterion IDs in scope, for example
`#[trace("TC-704", "FR-249-AC-3")]`; a schema-only task-search check is not
evidence that the runtime provider implements the search semantics.

## Expected Results

- Step 1: exact fixed-work proof only with accepted complete certificate;
  the altered certificate is rejected. The miss settles `Violated` only
  after replay confirms the job has positive remaining demand at its deadline
  and the continuation is time-divergent.
- Step 2: a bounded run settles `incomplete` and its task-search evidence
  replays. Missing/extra edges, an omitted reachable frontier node, altered
  limits/accounting, or a false complete-prefix assertion are rejected.
  Bounded exploration never settles `Holds`.
- Step 3: preempted service remains unchanged during the other job's delay;
  all clocks and the running service advance by that delay. Mutated service
  or identity refuses replay.
- Step 4: the mode transition increases only the allowed budget by
  `C(HI)-C(LO)` and preserves actual demand, service, priority, identity and
  original deadline. The valid queue-growth witness proves a positive-time
  continuation; its malformed mutations and an overflow-only miss are
  rejected.
- Step 5: each valid release is recorded in source order; duplicate and
  unresolved declarations refuse at their specified locus; a non-dense
  model's release checks successfully and the lowering request receives the
  existing typed timed-model refusal.
- Step 8: the overload refutation's `prefix` ends at the miss step and
  `loop` is empty. Each witness mutation refuses
  `invalid_runtime_input`/`invalid-value` at its `/queue_growth/...`
  pointer; the three-cycle finite loop refuses at `/loop/<last>/post_state`.
- Step 9: `SOnce` is refuted; `prefix[trace_position]` (index 2) is
  `deadline-miss` of `s` job 0 at time 2 with `demand` 3 and `deadline` 2,
  the prefix ends with `complete` of `s` job 0 at time 3, and `loop` is one
  `terminal-stutter` step with positive delay that reproduces; the
  zero-delay stutter refuses at `/loop/0/delay`, and the record over
  `OnceAgain` refuses at `/loop/0/transition`. `OnceLocked` proves on the
  closed certificate with the miss node `locked`, and its time-lock item
  refutes.
- Step 10: `incomplete`, `limit-reached{limit: work-limit}`, with
  `task_search` nodes that include the miss state, no `counterexamples`
  entry, never `refuted`.
- Step 11: refused `malformed_wire` at `/loop` before any exploration; the
  admitted-endpoint arm is not run.
