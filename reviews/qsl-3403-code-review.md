---
id: SR-2026112520400001
title: "Code review — filament-spec-language QSL-3403 replay carries emit.output_bytes"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-command/src/command/limits.rs, fsl-foundation/src/setting.rs, fsl-replay/src/limits.rs, fsl-replay/src/proof_result/plugin_tests.rs, fsl-replay/src/request.rs, fsl-replay/tests/it/stage_limit_named_root_only.rs, tests/it/producer_unit_identity.rs, tests/it/replay_request_file.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-255", type: references }
---

## Summary

Ticket: QSL-3403. PR agent-ix/filament-spec-language#3969 at head `512f1536a56fa260548122ced17857baa330f648`, diff `origin/main...HEAD` (two commits), rust-review lane folded in. The four newer `origin/main` commits (QSL-3269, QSL-3392, QSL-3360, QSL-3371) touch no file of this PR and no replay-limit code; `git merge-tree` is clean.

Producer and reader sides agree. The deny list `is_replay_wire_stage_setting` (fsl-replay, which also feeds `StageLimits::with_named`, request decode and the plugin replay context filter) and the allow list `COMMON` + `REPLAY_ONLY` (fsl-command, `replay --limit`) both gain `emit.output_bytes` and both refuse `exact_prob.max_witness_paths`. The new test `the_replay_command_and_request_take_the_same_stage_settings` holds them equal over `Setting::ALL`. The accepted wire set (68 stage settings) equals FR-255's replay list exactly, less `replay.input_bytes`. Nothing in the repository or the sibling filament/quire repositories sends `exact_prob.max_witness_paths` on a replay request (the plugin context now filters it out), and replay never read it: only `graph_replay.rs` reads `caller.engines.model_check`. Adding `ExactProbMaxWitnessPaths` to `is_item_engine_setting` leaves `is_positive_count` unchanged (it matches that setting first).

Memory and result: `emit.output_bytes` bounds the canonical package only (`quire_canonical::Limits::new`); nothing pre-allocates by the bound, and the emitted bytes do not depend on it (lifecycle test `the_emit_output_ceiling_is_raised_by_the_settings_operation_and_by_a_request`). A raised entry can only turn a refusal into the same result. The default stays 16777216. It does lift an incidental backstop: a request that already raises `s1.*`/`s3.*` (as TC-736 does) now runs past E4 into the recompile cost that OOMed at 12 GiB (QSL-2825). That is opt-in and bounded by the other request settings the caller already controls; the plugin context sends the proving run's own emit bound, which the run already emitted under.

## Verdict

**CONDITIONAL** — one low finding. The change is correct on both sides; tags name criteria the code implements.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | TC-736's `#[ignore]` reason still says replay "reaches the fixed emit.output_bytes bound"; this PR removes that refusal, so the reason is now false. The remaining cause is the 12 GiB recompile memory (QSL-2825); QSL-3253's text is equally stale. | fsl-replay/src/execute/tests/composite.rs:878 |

## Coverage

- Rust lane: no new `unwrap`/panic in production code, no integer conversions, no locks or I/O added. Doc comments on `with_named` and `decode` updated to the shared check.
- `#[implements]`: `is_replay_wire_stage_setting` (FR-255-AC-24, FR-263-AC-3, FR-399-AC-5), `CommandLimits::exposes` and `Setting::is_item_engine_setting` (FR-255-AC-24) — each implements the named refusal/acceptance set; `use ix_trace_rs::implements;` with bare attributes; both crates already depend on ix-trace-rs v0.1.2.
- TC-736 (the 12 GiB test) was not run, per instruction.
