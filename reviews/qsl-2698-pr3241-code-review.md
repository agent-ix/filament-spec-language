---
id: SR-20261121024111
title: "code review of PR3241 analyze replay reads under replay.input_bytes"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-analyze/src/analyze.rs, fsl-analyze/src/analyze/{clause_item,derived_item,invariant_item,protocol_item,state_graph_item}.rs, fsl-analyze/tests/it/{analyze_cli_support,analyze_package_selection}.rs, fsl-command/src/command.rs, fsl-command/src/command/analyze.rs, fsl-replay/src/{outcome,proof_result}.rs, fsl-replay/src/proof_result/plugin.rs, fsl-replay/src/model_check/settle.rs; PR3241 head 209531aa988a4b55fe7e16ede4fd208953707071"
review_set: subset
---

## Summary

Ticket: QSL-2698. Code-review with the rust-review lane folded in; diff only, no build (context from `git show` at the head). `RunLimits.replay_input_bytes` reaches `AnalyzeLimits.replay` through `read_run_members` on both the source and the package route, and every `ReplayPackage::read` in analyze (clause source/package, invariant, derived, protocol, state-graph evidence) now goes through `clause_item::read_package`, which refuses `ReplayRequestRefusal::BoundExceeded` before the read. Every `replay_*` call in `fsl-analyze/src/analyze/` now takes the run's `limits.replay`. The refusal settles through `InconclusiveCause::from_replay_refusal` only on the item whose replay read refused; other items are untouched. An emit failure keeps `analyze`/`package-emission`. `InconclusiveCause` has no serde derive and is written only through FSL's `ItemCause` (outcome schema `item` has `additionalProperties: true`); the plugin path is inbound only and refuses `ReplayRefusedAtBound` as the FSL-only `replay-refused`, so nothing schema-invalid reaches a provider document. TC-2958 asserts kind, code, cause and setting for both replaying items on both routes, `Bounded` `proved`, category `inconclusive` and exit not 30.

Remaining defects: non-size read failures stay internal faults where FR-940 says `replay_refused`; the witness envelope reconstruction and the protocol-conformance replay still use `ReplayLimits::default()`; AC-22's test-only-emitter sentence has no test; and three hunks are not rustfmt-clean, which fails the `cargo fmt --check` gate.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `read_package` maps every non-size `ReplayPackage::read` failure to the internal fault `analyze`/`package-bytes` or `package-emission`, but FR-940 says a refusal of that read is a replay refusal of the item alone: `inconclusive` `replay_refused` with `resource_exhausted` for an allocation it cannot make and `invalid_package` otherwise. | fsl-analyze/src/analyze/clause_item.rs:460-461 |
| FND-002 | medium | The witness envelope reconstruction still passes `ReplayLimits::default()`, though FR-255 says `replay.input_bytes` bounds the replay request and envelope bytes; a raised setting is not honoured for an envelope over 1 MiB (it faults `analyze`/`envelope`), and the envelope check ignores a lowered one. | fsl-analyze/src/analyze/clause_item.rs:596, fsl-analyze/src/analyze/clause_item.rs:690 |
| FND-003 | medium | The protocol-conformance replays inside analyze (`recording_result`, `every_behavior_result`) still bound the counterexample with `ReplayLimits::default()`, so the run's `replay.input_bytes` does not reach them, and a bound refusal there is `invalid` and not `replay_refused`. | fsl-analyze/src/protocol_analysis/conformance.rs:141, fsl-analyze/src/protocol_analysis/conformance.rs:843 |
| FND-004 | medium | FR-940-AC-22's last sentence (a test-only emitter that fails at the replay's emission settles `AtMostTwo` `failed` with `analyze`/`package-emission`) has no test: TC-2958 does not exercise it and no test in fsl-analyze or fsl-command names `package-emission`. | fsl-analyze/tests/it/analyze_package_selection.rs:1262-1328 |
| FND-005 | low | Three changed hunks are not rustfmt-clean (the `ReplayRefusedAtBound` arm, the reflowed `fsl_replay` import list, and the `replay_model_graph_from_package` call), so `cargo fmt --all -- --check` in the Makefile gate fails. | fsl-replay/src/proof_result/plugin.rs:401, fsl-analyze/src/analyze/state_graph_item.rs:28-31, fsl-analyze/src/analyze/state_graph_item.rs:1536-1541 |

## Verdict

Not mergeable as is: there are no highs, and nothing downgrades a verdict for an item that did not need the replay. The bound cannot be bypassed for the package read (the replay request decode is also checked under the run's limits), and no schema-invalid output is possible. Fixes: FND-001 map non-size `ReplayPackage::read` refusals to `ReplayRefusal::Request` (or the reader's own refusal) so they settle `replay_refused` with `invalid_package`/`resource_exhausted`. FND-002 pass `inputs.limits.replay` / `self.limits.replay` into `WitnessEnvelope::reconstruct`. FND-003 thread `AnalyzeLimits.replay` into the two conformance replays and settle a `BoundExceeded` there as `replay_refused`. FND-004 add the emitter-failure case, or the lead defers that sentence to a ticket. FND-005 run `cargo fmt` on the three files.

Driver scope (not a defect here): FR-287 names `--limit` only for the parse/check/compile stage settings and for `run`'s observation settings, not for `analyze` or `replay.input_bytes`. FR-255 says the setting is "raised from the library and the CLI", so a driver `--limit replay.input_bytes=N` on `analyze` is remaining driver work, and FR-287 does not name it yet.
