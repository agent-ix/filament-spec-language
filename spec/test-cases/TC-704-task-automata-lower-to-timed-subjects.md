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

Scope: FR-249-AC-1 to FR-249-AC-6, FR-249-AC-11 to FR-249-AC-16,
FR-237-AC-7 to FR-237-AC-12, FSpec FR-364-AC-21, FSpec FR-364-AC-22 (its
`queue_growth` cycle case), FSpec FR-364-AC-23, FSpec FR-364-AC-24,
FR-239-AC-6,
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
12. Check FR-249-AC-13's `mode` declarations over `C` and `MC` (FSpec
    FR-419-AC-14): the recorded switch, the unknown task set, the unknown
    operation, the repeated declaration and `mode MC lo`.
13. Analyze `MCFp` with `backend` `en-6` (FR-249-AC-14), replay its
    counterexample through `ModelTrace`, then replay each FSpec FR-364-AC-21
    tamper, a `queue_growth` witness whose cycle holds the `go_hi` entry
    (FSpec FR-364-AC-22), the record against the package without the `mode`
    declaration and the record with a `mode` member added (FR-237-AC-9).
    Rerun with the source operations declared in reverse order.
14. Analyze `MCEarly` and `MCNone` (FR-249-AC-15).
15. Parse, build and format FR-249-AC-16's sources.
16. Write the FR-249-AC-9 refutation's counterexample to JSON, read it back
    with `read_temporal_counterexample`, rewrite it, and replay the document
    through `replay_model_trace_document`. Then read and replay each
    FR-237-AC-10 shape mutation, alone and embedded as `counterexamples[0]`
    of its FR-331 result.
17. Replay FR-237-AC-11's alphabet mismatches: the source-spelled activation,
    the source projection, the inserted task step over `Rpc`, the two typed
    forms given the other alphabet, and the `Tick` model's synchronized
    activations with each FSpec FR-364-AC-23 mutation.
18. Replay each FR-237-AC-12 and FSpec FR-364-AC-24 member tamper through
    the document form.

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
- Step 10: `incomplete`, `limit-reached{limit: max_task_product_work}`, with
  `task_search` nodes that include the miss state and a frontier row of
  reason `work-limit`, no `counterexamples`
  entry, never `refuted`.
- Step 11: refused `malformed_wire` at `/loop` before any exploration; the
  admitted-endpoint arm is not run.
- Step 12: the switch is recorded in source order beside the releases;
  `missing-name` naming `Nope`, then `C::nope`; `duplicate-declaration` at the
  second; `invalid_syntax`/`unexpected-token` at `lo`.
- Step 13: refuted with the miss of `h` job 0 at 10 (demand 3, deadline 10,
  service 2), the exhaustion `preempt` at 2 with null `source_transition`,
  exactly two `budget-change` steps at 12 directly after `go_hi` (`h` job 0
  then job 1, `budget_delta` 1, delay 0, `source_transition` `go_hi`), a loop
  of elapsed 10 with no `budget-change`, and `reproduced`. Each tamper
  refuses `invalid_runtime_input`/`invalid-value` at its FSpec FR-364-AC-21
  pointer, the cycle at `/queue_growth/cycle/<i>`, the package without
  `mode` at `/prefix/10/post_state` (and, with that key rewritten to `LO`, at
  the first `budget-change` step's `/transition`), the `mode`
  member `malformed_wire` at its pointer. The reverse-order run reports the
  same miss and the same two `budget-change` steps.
- Step 14: `MCEarly` settles `incomplete` with `task_search`, with no
  `deadline-miss` edge and one `budget-change` of `h` job 0 after each `go_hi`
  edge; `MCNone` has no `budget-change` and refutes at `h` job 0's miss at 10
  with a `queue_growth` growth block of one `h` job (`allowance` 2,
  `remaining` 1, `preempted`), `iteration_shift` 1, `elapsed` 10.
- Step 15: one `ModeSwitchDeclaration` and `ModeSwitchForm`, the hybrid
  modes and the field `hi` unaffected; each malformed source refuses at its
  stated token; the formatted text is `mode MC hi on C::go_hi;`, formatting
  is idempotent, and both sources compile to equal packages.
- Step 16: the rewritten JSON equals the written JSON, and the document
  replays `reproduced` with the result EN-6 recorded. The analyze run that
  produced the refutation replayed it through `replay_model_trace_document`,
  and a run whose written counterexample fails that reader does not settle
  `refuted`. Each mutation refuses
  `malformed_wire` with its FR-237-AC-10 cause and pointer, and, when
  embedded, with the pointer prefixed `/counterexamples/0/counterexample`.
  No mutation replays a step.
- Step 17: each mismatch refuses `invalid_runtime_input`/`invalid-value` at
  its `/transition/type` pointer, or, for a typed form given the other
  alphabet, with `ModelTraceRefusal::TraceAlphabet` naming both alphabets.
  None is `reproduced`. The `Tick` move replays as `activate a#0` then
  `activate b#0`, and each mutation refuses at its FSpec FR-364-AC-23 pointer.
- Step 18: each tamper refuses `invalid_runtime_input`/`invalid-value` at its
  named member, naming the supplied and the recomputed value, and never
  only as a step that is not enabled.
