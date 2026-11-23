---
id: SR-3617001
title: "Code review — QSL-2962 PR #3617 call_site on the compile request"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; git diff origin/main...f0f9fc71d346391379b5fa28c1419a2b55e21fe4 (12 files: fsl-command/src/command.rs, command/{analyze,monitor,output,plan,transfer,wire}.rs, fsl-command/tests/it/{cli_contract/compile.rs,cli_contract/mod.rs,selection_text.rs}, fsl-replay/src/{call_site.rs,lib.rs}); FR-027 call_site rule and FR-027-AC-15..21, TC-3210, FR-121-AC-35/36, FR-286, FR-865-AC-19, FR-285"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-027
    type: references
  - target: ix://agent-ix/filament-spec-language/TC-3210
    type: references
---

## Summary

Ticket: QSL-2962. Code review of PR #3617, with the rust-review lane folded in. Head f0f9fc71d346391379b5fa28c1419a2b55e21fe4, base origin/main a8397972b7. Read-only. No cargo was run, because other cargo jobs were active on titan (one job at a time). The worktree also has an uncommitted, unpushed edit that re-tags the `pick` test as TC-3210/FR-027-AC-19. This review covers the committed head only.

Verified clean:

1. **Locate over the command's own checked package.** The PR adds `fsl_replay::locate_in_checked(&CheckedUnit, package_name, &selection)`. `complete_sources` calls it on the `checked` that the command's own `check_sources` produced, right after S4 packaging (`package_with_temporal_selections`), so there is no second compile pass and no limits other than the request's own (command.rs:3539-3546). `call_site()` in fsl-replay now delegates to the same function after its own compile, so behaviour is unchanged.
2. **A refusal writes nothing.** The locate runs inside `complete()`. `write_compile_handoff` and `output_file::write_new_file` run only after `complete()` returns (command.rs:3036-3059). The model-unit path writes `output` earlier (command.rs:3013-3015), but it refuses a present `call_site` before compiling (command.rs:2987). There is no watch variant of compile. The `--output` override goes through the same late write.
3. **Validation.** `request_call_site` runs immediately after `integer_division`, before extensions, handoff, output and intake. That covers `null`, a non-object, an unknown member, a missing member, a non-string member, a `kind` other than `imported-function`, an empty library, an identifier that does not parse, and a multi-element array. Each refuses `ClauseRequest { member: "call_site" }`. The model-unit refusal has to happen after the program is read, and it does.
4. **Cause, stage and exit.** `RunCause::CallSite` has the refusal's code. Its category is the code's own (refusal, exit 20), and a `Fault` maps to InternalFailure (exit 30). The outcome document has stage I1 and holds exactly the one `OutcomeDiagnostic::from_call_site_refusal` diagnostic. output.rs maps it to Spine Intake with Details::None.
5. **Message.** The package is named by the program source's `identity` label (`package_name(sources[0]...)`). The tests pin `package parcel`.
6. **Internal constructors.** The analyze, monitor, plan and transfer `wire::CompileRequest` literals set `call_site: None`. Their own request types keep `deny_unknown_fields`, so `call_site` is refused on those surfaces. The export surface is the exception (FND-002).
7. **Tests.** AC-15, 16, 17, 18, 20 and 21 each have an exact-document test through `compile_outcome`. AC-15 also asserts that neither `out.json` nor the `handoff` directory exists. AC-19 is known open: the `pick` test asserts the exact document the renamed AC will pin, with refusal, exit 20 and stage I1, no `selection`, and `locus` and `details` null. It is tagged FR-027 only until #3618 merges. The FR-865-AC-19 text test now goes through `compile_outcome` and renders the `  hint:` line. Stacking two `#[trace]` attributes has precedent (fsl-analyze/tests/it/hybrid_requests.rs:831-832).

## Verdict

**CONDITIONAL.** There is no high finding. FND-001 to FND-003 are medium:
- FND-001: the `Fault` mapping is untested, although tests can be built.
- FND-002: an export request now silently accepts `call_site`.
- FND-003: a one-element FR-480 array is accepted.

FND-004 is low (fmt hunks). The core wiring, the no-write ordering and the AC-15..21 tests are correct.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-027's `CallSiteRefusal::Fault` -> internal-failure, exit 30 has no test, though one can be built at both levels: (a) fsl-replay, `locate_in_checked(&checked, "", &selection)` returns `Fault` (code `runtime_invariant`); (b) fsl-command, a hand-built `RunError { cause: RunCause::CallSite(Box::new(CallSiteRefusal::Fault(InternalFault::new(..)))), .. }` as selection_text.rs already builds RunErrors, asserting category internal-failure, exit 30, stage I1. Only an end-to-end path is unreachable (an empty identity label is refused at intake) | fsl-command/src/command.rs:1408-1413; fsl-replay/src/call_site.rs:737-753 |
| FND-002 | medium | An export request (`native-run/1` with `export`) is re-read as `wire::CompileRequest` (export.rs:293-297). Before this PR `deny_unknown_fields` refused `call_site` there. Now it is admitted and silently ignored: `compile_and_map` never calls `request_call_site`, and the explicit compile-only-member refusal covers only `output` and `handoff` | fsl-command/src/command/export.rs:219-227; fsl-command/src/command/export.rs:293-297; fsl-command/src/command/wire.rs:79-83 |
| FND-003 | medium | The FR-480 array check is `program_sources > 1`, but `compile_program` keeps no object-versus-array distinction. A one-element program array (non-empty, so a valid FR-480 array) with `call_site` is admitted and located, although FR-027 says an FR-480 array of units refuses `invalid-request` naming `call_site`. `validate_handoff_request` has the same count precedent, so this may be intended, but the spec does not say so | fsl-command/src/command.rs:113-115; fsl-command/src/command/wire.rs:197-222 |
| FND-004 | low | There are hunks outside the change. compile.rs:447-448 re-wraps a pre-existing line, and selection_text.rs:211-217, 231-237 and 248-254 re-wrap three unrelated `#[trace]` attributes. `cargo fmt --check` (pinned 1.98, rustfmt 1.9.0) also wants the PR's own `PICK` const on one line (compile.rs:475-476) and the three re-wrapped attributes collapsed. main is not fmt-clean either | fsl-command/tests/it/cli_contract/compile.rs:447-448; fsl-command/tests/it/selection_text.rs:211-254; fsl-command/tests/it/cli_contract/compile.rs:475-476 |

FND-001 fix: add a replay test `locate_in_checked` over any checked unit with package name `""` -> `CallSiteRefusal::Fault`, and a fsl-command test of the `RunError` mapping (category, `exit_code()` 30, document `last_stage` I1, one diagnostic of code `runtime_invariant`).

FND-002 fix: add `call_site` to the export compile-only-member refusal at export.rs:219, or call `request_call_site` in `compile_and_map`, and add a test.

FND-003 fix: keep `is_array` on `CompileProgram` and refuse when it is set, or have the spec owner state that a one-element array counts as a single program.

FND-004 fix: drop the unrelated re-wraps and run the pinned `cargo fmt` on the PR's own lines.

## Coverage

- FR-027 `call_site` statement: examined. Clean apart from FND-001 to FND-003.
- FR-027-AC-15: examined. Exact diagnostic, nothing written, exit 20, stage I1. Clean.
- FR-027-AC-16: examined. Clean.
- FR-027-AC-17: examined. Clean. The test supplies only `test/units` (plus `test/wrap`), not AC-15's `test/other`, which does not affect the refusal.
- FR-027-AC-18: examined. Clean.
- FR-027-AC-19: known open (`first` is reserved; rename in #3618). The test asserts the exact renamed document. Not counted.
- FR-027-AC-20: examined. Bytes equal and the output file is equal. Clean.
- FR-027-AC-21: examined. Every listed case is covered, and the model unit and the array are both tested. FND-003 is the one-element array.
- FR-285 and FR-286 `Missing selections` (`unimported-library`): examined. Clean.
- FR-865-AC-19: examined. The test goes through `compile_outcome` and the text has the hint line. Clean.
- FR-121-AC-35/36: examined. The replay `call_site` delegation is unchanged in behaviour. Tags are in SR-3617002.
- Tests: not run (other cargo jobs active on titan).

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | `a_one_unit_array_takes_a_call_site_like_the_single_program` is tagged `FR-027` only. FR-027-AC-21 (merged in #3620) now names that clause: "the same request with `call_site` valid and `program` an FR-480 array of exactly one unit behaves exactly like the single-object form". The test should carry `TC-3210`, `FR-027-AC-21` | fsl-command/tests/it/cli_contract/compile.rs:818-820 |

## Dispositions

Round 1, head 4973631532a165ed9224c593050d5963df9ccbe1, fix commit 4973631532 on merge-base bc77821e18. Checked by reading the code and running `quire matrix`; no cargo run.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | fsl-replay `a_package_name_that_is_no_library_name_is_a_fault` (imported_function_site.rs:453) shows `locate_in_checked` returns Ok for "fixture" and `CallSiteRefusal::Fault` for "". fsl-command `a_call_site_fault_is_an_internal_failure_at_stage_i1` (selection_text.rs:263) asserts category InternalFailure, exit 30, a schema-valid document, `last_stage` I1 and one diagnostic |
| FND-002 | fixed | export.rs:219 refuses `call_site` with `output` and `handoff`. `a_compile_only_member_beside_an_export_refuses_naming_it` (export.rs:344, TC-3213/FR-1015-AC-18) asserts invalid-request naming each member, exit 20, `last_stage` null, and no files written |
| FND-003 | fixed | Spec #3620 now states that an array of exactly one unit behaves as the single-object form (FR-027 statement, AC-21), so the code's `> 1` check is correct. `a_one_unit_array_takes_a_call_site_like_the_single_program` pins equal bytes and the same refusal (tag: FND-005) |
| FND-004 | still-open | The non-own hunks are reverted: compile.rs:447 and the three selection_text.rs attributes now match main. The PR's own lines are still not clean under the pinned `cargo fmt --check` (1.98): the `PICK` const (compile.rs:475), and lines new in this round at command.rs:3539-3541, export.rs:361 and imported_function_site.rs:320/471/487. Fix: run `cargo fmt` on those lines only. Low; main is not fmt-clean either |
