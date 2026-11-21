---
id: SR-2026112120444515239800
title: "code-review (with rust-review lane) of PR3378 closure-stutter witness and replay consumer"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR3378 f092056d7021d55ee93179645fb3f02eb806e315 over base dab8c60624b4427e2290efe7ebaa3827f5ae0842; fsl-replay/src/model_check/{counterexample,replay}.rs, fsl-replay/src/model_check/replay/timed.rs, fsl-replay/src/model_check/refinement_replay.rs, fsl-replay/src/outcome/counterexample.rs, fsl-replay/src/spine/request.rs, fsl-replay/src/lib.rs, fsl-analyze/src/model_check/liveness.rs, fsl-analyze/src/analyze/{clause_item,items}.rs, mechanical constructor fields in 50 further files"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-128
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-1030
    type: references
---

## Summary

Ticket: QSL-2247. Code review with the rust-review lane folded in, of the actual diff dab8c606..f092056d (63 files, +1656/-102). The normative text read is FR-128 Behavior "Closure stutter" and the writer and reader paragraphs, plus FR-128-AC-10..15 and FR-1030-AC-6/7, at the reviewed head. No Cargo, clippy or test was run, so every behaviour claim here comes from reading the source.

The core mechanism matches the contract. `TemporalCounterexample.closure_stutters` has `StepSite::{Prefix,Loop}`, `post_state` and `delay`. `closure_loop()` accepts only a sole `Loop(0)` with `entry == prefix.len()` and no terminal marker. `check_closure_stutters` runs before `replay_prefix` in `settle`, in the order alphabet, placement, kind (each at `<site>/transition/type`), then delay (at `<site>/delay`). `close_closure_loop` refuses a terminal state at `/loop/0/transition` and a wrong key at `/loop/0/post_state`. For fairness, the closure loop state takes the real `expand` enabledness with `taken: None`, which maps to `EdgeLabel::Stutter`. `fsl-eval/src/fairness/lasso.rs::check_loop` reads `step.enabled` whatever the label, so a weak or strong constraint enabled there is unfair, and no terminal shortcut is taken. `trace_of` gives a loop of length 1. The flag goes from `TemporalItem.stutter_closure` through `ModelTraceSubject`, `ReplayInputs`, the reference-literal argument and `PackageClauseSubject`. The timed, protocol, refinement, invariant and derived entries refuse at the alphabet. EN-1 keeps the entry key for an all-closure loop. The writer emits only the sole closure loop, and the reader reads the step for the model alphabet only. `ModelStepWire` still refuses the step. Upstream `ModelStep`, `ModelTransition` and `Lasso` are unchanged, and there is no fake operation. Integration: `lib.rs` keeps both `ClauseRunCounterexample`/`ModelClauseCounterexample` and `ClosureStutter`/`StepSite`. The new `tests/it/package_model_native.rs` literal is a terminal-stutter lasso (asserted at :325), so an empty closure vector is correct there.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The public task-derived entry replay_task_product_trace never reads closure_stutters, so a typed task counterexample carrying a closure step replays as if the step were absent instead of refusing at its /transition/type | fsl-replay/src/outcome/counterexample.rs:116 |
| FND-002 | low | evaluate_model_trace (FR-330 native/admit route) refuses a closure lasso as LoopOpen through close_loop rather than the ClosureStutter alphabet refusal, and the closure flag it newly passes to trace_of is unreachable | fsl-replay/src/model_check/replay.rs:2871 |
| FND-003 | low | TemporalCounterexample::measured_bytes counts no closure_stutters element or its StateKey heap, so the witness payload bound under-measures a payload with many closure steps | fsl-replay/src/model_check/counterexample.rs:436 |

FND-001: `replay_task_product_trace<T>` (outcome/counterexample.rs:116-215) builds `TaskProductTrace` from `counterexample.prefix` and `lasso` only. FR-128 says task-derived subjects "SHALL NOT gain the alternative", and FR-128-AC-11 requires the task-derived case to refuse at `/transition/type`. The JSON route is safe because `read_task_temporal_document` passes `closure_alphabet=false`, which refuses at the reader. But the typed public entry accepts any `TemporalCounterexample<TaskProductTransition>`, and since this PR every such value has a `closure_stutters` field. Fix: call the alphabet refusal there and map it to `TaskProductTraceReplayRefusal::TraceAlphabet { pointer: "<site>/transition/type" }`, then add the AC-11 task case.

FND-002: `close_loop` with `entry == steps` and no terminal marker returns `LoopOpen`, so the `payload.closure_loop().is_some()` argument at replay.rs:2903 never takes effect. The production native payloads (spine/clause/model_trace.rs:132, the native read) carry an empty vector, so this is public-API only. Either call `refuse_closure_alphabet` first or drop the dead argument.

FND-003: the new `Vec` adds its header through `size_of_val(self)`, but each element (a site, a `StateKey` byte vector and an optional `Rational`) is not counted. The reader admits any number of closure steps before replay refuses the first one. Input bytes bound the document, so the impact is small.

## Verdict

Changes requested on FND-001, a medium gap in a public entry with an easy fix. FND-002 and FND-003 are low. The closure mechanism, the refusal order, fairness parity, the holds/false/undefined settlement, the writer and reader, and the source/package/reference-literal flag threading are correct as read. No runtime evidence exists yet, and the root final gate is still outstanding.

## Scope and evidence

Rust lane: rust-style idioms are respected. There is no `unwrap` in production code, saturating arithmetic is used in `written_position` and `len`, the enums are small and `Copy` where possible, and there is no `unsafe`. Making `ClauseSubject::configured`, `PackageClauseSubject::configured` and `PackageClauseSubject::replay_counterexample` public widens the API for integration tests only, which is acceptable for prerelease. Removing `LivenessRefusal::ClosureStutterLoop` is safe: no reference remains (grep over *.rs). The changed type footprint grows `TemporalCounterexample` by one `Vec`. The inherited clippy errors (product, measure, clock_enabled) name no changed type, but this was checked against the source only and not reproduced.

- FR-128-AC-10: examined, code path settle/check_closure_stutters/close_closure_loop/trace_of is correct
- FR-128-AC-11: examined, FND-001
- FR-128-AC-12: examined, LoopState.taken None and EdgeLabel::Stutter, real enabledness
- FR-128-AC-13: examined, Verdicts/NoValue reached through check_formula
- FR-128-AC-14: examined, writer sole-loop, reader model-only, written_position re-index, ModelStepWire refusal
- FR-128-AC-15: examined, PackageClauseSubject now admits the flag for Liveness only and threads it
- FR-1030-AC-6: examined, liveness.rs lasso keeps entry key, refuted only through replay
- FR-1030-AC-7: examined, same mechanism with a one-step stem
