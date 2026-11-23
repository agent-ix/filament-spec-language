---
id: SR-599008
title: "Code review — QSL-2910 PR #3559 caller limits at analyze, monitor and replay"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; git diff origin/main...91a4787ba (fsl-command/src/command/{limits,analyze,monitor,replay}.rs, fsl-command/src/command.rs, fsl-foundation/src/setting.rs, fsl-replay/src/{limits,request,spine}.rs, fsl-semantics/src/check/refusal.rs, tests)"
review_set: subset
---

## Summary

Ticket: QSL-2910 (QSL-2912 pointer). Code review with the Rust lane (rust-review, rust-style; repository AGENTS.md/CLAUDE.md applied) of PR #3559 at head 91a4787ba. The PR adds `CommandLimits` as one caller-limit object per operation, threads it into `analyze_with_limits`, `analyze_with_providers_and_limits`, `monitor` and a new `replay_with_limits`, adds five `monitor.*` settings to the `Setting` table, removes `RunLimits.replay_input_bytes`, and makes `CallerLimits::from_operands` refuse a setting no part holds. The coder states the PR is partial against FR-255-AC-24; that claim is accurate (see the companion gap analysis), but the tests are tagged and named as if the criterion were fully met.

## Verdict

**FAIL as a completion of FR-255-AC-24; mergeable as a partial step once FND-001 and FND-002 are resolved.** The threading that exists is correct: defaults are unchanged (`SpineLimits::default().imports` equals the previous `V2ReadLimits::bounded(I2_NATIVE_JOIN_STORAGE_DEFAULT)`, monitor accounting default equals `call_accounting(&Accounting::default())`, analyze `limits.replay` is unchanged when unset), each set setting overrides only its own field, the exhaustive `Setting` matches in `spine.rs` and `refusal.rs` were extended, `StageLimits::with_named` refuses `monitor.*`, and no compatibility shim was left behind. Focused unit tests pass.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `each_operation_exposes_exactly_its_enumerated_settings` asserts analyze and monitor sets that differ from FR-255's enumerated lists, yet is named "exactly" and tagged FR-255-AC-24 | fsl-command/src/command/limits.rs:529-563 |
| FND-002 | medium | Entry points never check `CommandLimits::operation()`; a limits object built for another operation is silently half-applied | fsl-command/src/command/analyze.rs:829; fsl-command/src/command/monitor.rs:1046; fsl-command/src/command/replay.rs:55 |
| FND-003 | medium | `monitor.preparation_*` are accepted by `command::monitor` but no FSL code reads `preparation()`; the bound reaches nothing inside the entry point | fsl-command/src/command/limits.rs:91-117,325-334; fsl-command/src/command/monitor.rs:1093-1096 |
| FND-004 | medium | `CallerLimits::from_operands` refuses the five recognized `monitor.*` settings with `UsageCause::UnknownSetting`; the all-settings test was narrowed by a name-prefix string filter and no test pins the new refusal | fsl-replay/src/limits.rs:523-527,538-541 |
| FND-005 | medium | New public `replay_with_limits` has no test; no entry-level test shows any setting other than `replay.input_bytes` reaching an executed bound | fsl-command/src/command/replay.rs:55-72 |
| FND-006 | low | `StageLimits::decode` doc still lists only `replay.input_bytes` and `format.output_bytes` as refused names | fsl-replay/src/request.rs:132-133 |

FND-001: FR-255 "Lifecycle caller-limit coverage" requires analyze to accept `admission.*`, `library.*`, `s1.diagnostics`, all `model_check.*`, `expansion.max_candidates`, `simulation.max_steps` and the engine families, and monitor to accept `admission.*`, `library.*` and, with routed items, the engine and emit settings. The test's expected sets omit them, so it encodes the partial set as the specified one and the computed matrix will report FR-255-AC-24 backed. Rename the test to state the implemented subset, drop the FR-255-AC-24 tag (or keep a TC-721-only tag on the parts actually met, such as the replay set, the unavailable refusal and the `replay.input_bytes` routing), and record the remaining settings as `Remaining work:` on QSL-2910.

FND-002: `apply_to_analyze` reads only spine, observation, accounting, replay and emit, so `analyze_with_limits(.., &CommandLimits::replay(), ..)` with `admission.*`, `library.*` or `model_check.*` set accepts them and changes no bound. `replay_with_limits` with an analyze object writes `emit.output_bytes` into `stage_limits`; with a monitor object it writes `monitor.*`, which the replay request decoder then refuses as an unknown `stage_limits` member. That breaks "accepted setting changes its bound / unavailable setting refused before work" for any caller that mixes them. Make the mismatch unrepresentable (one type per operation, or a constructor-typed wrapper) or refuse it at entry.

FND-003: `MonitorPreparationLimits`, `PreparationBound`, `PreparationRefusal` and `CommandLimits::preparation()` have no production consumer in FSL; `command::monitor` never reads them, and the request-path `run_bytes` passes `u64::MAX`. Enforcement exists only if the driver separately calls `preparation()` for its own `monitor_root` plan preparation. That may be the intended split (driver FR-003-AC-13/14), but the `monitor` doc and the `MonitorPreparationLimits` doc ("FSL owns the mapping") do not say the entry point does not apply them. Document on `monitor()` that preparation bounds are carried for the caller's plan preparation and are not consumed by this entry, or move them out of the entry-point parameter.

FND-004: the five `monitor.*` names are FR-255 table settings, so "unknown setting" is the wrong cause for `fsl-probability-check` and `xtask qualification` users; FR-255 calls these operation-unavailable. The test exclusion `setting.name().starts_with("monitor.")` branches on name text rather than a typed property and leaves the new refusal untested. Use a distinct cause (or document the new behavior) and add a test that `monitor.retained_positions=1` is refused by `from_operands`.

FND-005: `replay_with_limits` (format refusal, reader bound, `stage_limits` override) is exercised by no test; `apply_to_replay` is tested only in isolation. TC-721 step 11 is object-level, so this is not an AC gap, but the new public function and the analyze/monitor threading have no behavior test beyond the pre-existing `replay.input_bytes` cases. Add one replay entry test (for example `s1.tokens=1` refusing, `replay.input_bytes` reaching the reader) and one analyze or monitor case where a set stage setting is reached.

FND-006: `with_named` was updated to refuse `monitor.*`; the `decode` doc that describes the same refusal set was not.

## Gates

- `cargo test -p fsl-command --lib command::limits` (CARGO_INCREMENTAL=0, jobs 2, worktree target): see Coverage.
- `cargo fmt --check` is not clean on origin/main in unrelated files; the PR's own hunks are formatted. Clippy and the workspace suite were not run (filtered review build only, per brief).

## Coverage

Examined: FR-255-AC-24; FR-255 "Lifecycle caller-limit coverage" (analyze, monitor, replay lists); TC-721 step 11; driver FR-003-AC-12..AC-14 (filament-driver origin/main); every hunk of the diff; callers of `from_operands` (tools/fsl-probability-check, xtask, tests) and of the changed entry points; `CallerLimits::for_request`; `StageLimits::decode`/`with_named`. Driver origin/main `fsl-cli/src/cli.rs` still sets `RunLimits.replay_input_bytes` and calls the old `monitor` signature, so the driver must move with QSL-2912 (expected, no shim wanted).

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | `an_entry_point_refuses_the_limits_of_another_operation` builds a monitor `CommandLimits` it never passes to an entry point, then drops it | tests/it/replay_request_file.rs:398-445 |

FND-007: the `monitor` binding is dead setup (`drop(monitor)` only silences the unused variable); no case hands monitor limits to `analyze_with_limits` or `replay_with_limits`. Either use it for one of the cases or remove it.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | 858b5963f renamed the test to `each_operation_exposes_the_settings_accepted_today` and retagged all six limit tests `#[trace("TC-721")]`; no test claims FR-255-AC-24. |
| FND-002 | fixed | 7e35b77dc adds `CommandLimits::require`/`OperationMismatch`, checked first in `analyze_with_providers_and_limits` (so also `analyze_with_limits`), `monitor` and `replay_with_limits`, before any read; a test drives all three entries with an absent request path. |
| FND-003 | fixed | `monitor()` and `CommandLimits::preparation()` now document that FSL carries the preparation bounds and enforces neither; the driver enforces them and reports with `MonitorPreparationLimits::refusal`. |
| FND-004 | fixed | New `UsageCause::UnavailableSetting` for a table setting no limits field holds; the test uses an explicit `MONITOR_SETTINGS` list and asserts the cause and exact text for each. No other in-repo exhaustive match on `UsageCause` exists. |
| FND-005 | fixed | `replay_with_limits_applies_the_callers_settings` covers the default replay, `replay.input_bytes` bounding the reader (exact text), `accounting.work_units=0` settling inconclusive naming its setting, and `s1.tokens=1` refused through the `stage_limits` override path. |
| FND-006 | fixed | `StageLimits::decode` doc now names the five `monitor.*` settings. |
