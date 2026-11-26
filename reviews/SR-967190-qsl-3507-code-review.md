---
id: SR-967190
title: "Code and Rust review of QSL-3507 PR 4062"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 4062 at cd923f76fb65d5bcb13669be4f5968fb46eab096; fsl-analyze/src/protocol_analysis/conformance.rs, fsl-analyze/src/analyze/document/package_run.rs, fsl-eval/src/simulation/model.rs, fsl-analyze/src/protocol_analysis/finite_replay.rs, fsl-analyze/src/analyze.rs, fsl-analyze/tests/it/protocol_conformance.rs, fsl-analyze/tests/it/analyze_package_replay_conformance.rs, fsl-analyze/tests/it/model_instances.rs"
review_set: subset
---

## Summary

Ticket: QSL-3507. Reviewed PR 4062 against its stacked base. The empty-set causes and their order are right by reading; the object-set and limit rules FR-896 states are only partly implemented. No test was executed by this review.

## Verdict

**FAIL** - three high findings. Not mergeable.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Global conformance never reads subject.protocol: the bound over object is ignored and another protocol's binding is not declined | fsl-analyze/src/protocol_analysis/conformance.rs:571-690 |
| FND-002 | high | Run limits are applied per key instead of shared, and keys are still decided after a refutation or a run-ending stop | fsl-analyze/src/protocol_analysis/conformance.rs:632-670 |
| FND-003 | high | A protocol whose over is not an object reference now settles inconclusive no-instance instead of being checked | fsl-eval/src/simulation/model.rs:2576-2588 |
| FND-004 | medium | Checked-package route decides only the empty cases; any admitted key settles unsupported, so the ticket's package control is not met | fsl-analyze/src/protocol_analysis/conformance.rs:469-477 |
| FND-005 | medium | Package behaviour route skips the source route's missing-implementation refusal | fsl-analyze/src/analyze/document/package_run.rs:3694-3726 |
| FND-006 | medium | Two new string-edge findings: the item verdict is chosen by comparing output label strings | fsl-analyze/src/protocol_analysis/conformance.rs:671-687 |
| FND-007 | medium | Third copy of the FR-621 instance-selection rule, without the proved-record rule; instance listing and refusal mapping are also duplicated | fsl-analyze/src/protocol_analysis/conformance.rs:671-688 |
| FND-008 | medium | Paired-route tests are incomplete: no package admitted-key control, no route test for no-initial-state, no package or order-sensitive refutation | fsl-analyze/tests/it/analyze_package_replay_conformance.rs:302-318 |
| FND-009 | low | PR adds unformatted hunks while the body says the changed hunks are formatted | fsl-analyze/tests/it/model_instances.rs:845-855 |
| FND-010 | low | both_routes_with_snapshots asserts no exit code for the source run | fsl-analyze/tests/it/analyze_package_replay_conformance.rs:66-93 |
| FND-011 | low | New public types sit between use statements and ClaimInstances::first has no caller | fsl-eval/src/simulation/model.rs:125-160 |

## Finding detail

**FND-001.** global_behavior_item takes form but reads only max_depth, bounds and limits from it, and document.rs:2150-2184 builds the item's protocol binding only for deadlock and analysis kinds, so form.protocol is None here. The loop at 632-670 checks every admitted key. With subject.protocol binding over=p on the AC-7 fixture, AC-8 requires proved; head would report refuted for q (the base, checking only the first key, would have reported proved, so this is a new wrong verdict). A binding naming another protocol is not declined (AC-9). Violates FR-896 'The objects checked over every behaviour' bullets 1-2 and the arch ruling that subject.protocol is judged over the object when bound. Established by reading; not executed (probes p-ac8-bound-p, p-ac8-bound-q, p-ac9-other). Unconfirmed by execution. Confirm with: cd <scratchpad>/probe-src (head export plus sr-4062/zz_probe.patch) && CARGO_PROFILE_TEST_DEBUG=0 ~/dev/worktrees/locked-build.sh /mnt/build/qsl-3507-review-target cargo test --locked -p fsl-analyze --test it -- zz_probe --nocapture --test-threads=1

**FND-002.** Each key calls every_behavior_result with a fresh copy of form.limits, so max_states, max_transitions and the meter are not shared across keys; FR-896 requires FR-126 'Bindings and ranking' (shared budgets end the run, no binding decided after a refuted one, a run cancelled before a refutation returns the cancellation). With a max_states that covers one key but not two, the item reports proved where the rule gives incomplete. With key 1 non-proved and key 2 cancelled, 671-687 selects key 1 and drops the cancellation. A later key's early return at 668 replaces an earlier refutation. Per-key limits are established by reading; the max_states outcome was not executed (probe 'limit max_states=N keys=1|2'). Unconfirmed by execution. Confirm with: cd <scratchpad>/probe-src (head export plus sr-4062/zz_probe.patch) && CARGO_PROFILE_TEST_DEBUG=0 ~/dev/worktrees/locked-build.sh /mnt/build/qsl-3507-review-target cargo test --locked -p fsl-analyze --test it -- zz_probe --nocapture --test-threads=1

**FND-003.** instances() returns an empty Ok for any non-Reference ValueType (model.rs:2545-2547), and claim_instances turns that into NoInstance. global_behavior_item calls it for every protocol, so `over (n: Boolean)` settles inconclusive/no-instance; FR-896 says such an input ranges over its type's values within one protocol system. The base failed the same input with established-invariant-broken, so it never worked, but head turns a visible fault into a legitimate-looking verdict, and the package route does the same instead of unsupported. Established by reading; not executed (probes p-scalar, pk-scalar). Unconfirmed by execution. Confirm with: cd <scratchpad>/probe-src (head export plus sr-4062/zz_probe.patch) && CARGO_PROFILE_TEST_DEBUG=0 ~/dev/worktrees/locked-build.sh /mnt/build/qsl-3507-review-target cargo test --locked -p fsl-analyze --test it -- zz_probe --nocapture --test-threads=1

**FND-004.** With at least one admitted key the package item settles unsupported unsupported_construct/not-yet-implemented while the source route returns proved or refuted. This is not a dropped feature: the base package route refused the whole request as a pending family (package_run.rs on base, `kind == GlobalConformance && recording.is_none()`), so head is narrower in effect. But QSL-3507 requires source and checked-package cases plus an admitted-key control with the reader/replay path agreeing, FR-850 requires package equal to source, and there is no package admitted-key or later-refuted-key behaviour. The gap needs to be recorded on the ticket for the owner (COMMON: never drop or defer a feature silently).

**FND-005.** The source route declines missing_declaration/missing-name when a role names an unselected model before any analysis (request.rs, missing_role). The new package arm goes straight to package_global_behavior_item with no such check, so the same request can settle no-instance or unsupported on the package route and declined on the source route. A memory resolution error there is also reported as established-invariant-broken (conformance.rs:425-430). Established by reading; not executed (probe pk-missing). Unconfirmed by execution. Confirm with: cd <scratchpad>/probe-src (head export plus sr-4062/zz_probe.patch) && CARGO_PROFILE_TEST_DEBUG=0 ~/dev/worktrees/locked-build.sh /mnt/build/qsl-3507-review-target cargo test --locked -p fsl-analyze --test it -- zz_probe --nocapture --test-threads=1

**FND-006.** fsl-string-edge over FSL's crate roots reports 1169 occurrences on the base and 1171 on head; the two new ones are conformance.rs:677 (label == "refuted") and :684 (label != "proved") in global_behavior_item. string-edge is part of the repository's ci checks. The PR body says the string-edge findings are existing. The typed TerminalValue is available on the record.

**FND-007.** The stacked base already has the shared selector fsl-analyze/src/analyze/instance_records.rs `decisive` (FR-621-AC-9/23/24/25). The PR writes another over strings and takes report 0 when all are proved, skipping the weakest-certification and basis steps FR-896 requires through FR-621. No differing certifications were shown reachable here. Also duplicated: ReplaySubject::claim_instances repeats the binder lookup of ReplaySubject::instances (finite_replay.rs:720-752), and NoClaimInstances is mapped to item outcomes separately for source (conformance.rs:596-630) and package (469-488) with different fallbacks.

**FND-008.** The control in empty_protocol_over_settles_no_instance_in_both_routes (workspace 'package-no-instance-control') runs analyze on the source request only. No test drives no-initial-state or requires-bound through global conformance on either route (only the selector is tested in model_instances.rs). The later-refuted-key case is source only, does not assert the counterexample's over object, and no case has an undecided or limited earlier key before a refuted later one, or a shared run limit. The empty-set expected values themselves are hand-written from the spec.

**FND-009.** cargo fmt --all -- --check fails on base and head; comparing hunks, head adds one hunk in model_instances.rs (line 842: selector construction and RequiresBound pattern) and one in conformance.rs (line 428: PackageProtocolProgram::with_memory call), and removes one old import hunk in conformance.rs. The rest of the 1008 hunks are not from this PR.

**FND-010.** Commit e9703f61a5 removed `assert_eq!(source.exit, 0)` instead of asserting the expected exit for an inconclusive item; assert_equal only compares the two routes, so a wrong exit code shared by both passes.

**FND-011.** ClaimInstances and NoClaimInstances are declared in the middle of the module's use block; claim_instances ends with no blank line before the next doc comment; `ClaimInstances::first` is public with no caller in the workspace.

## Dispositions on `ab1075e479`

FND-001 fixed: global conformance now resolves `subject.protocol`, uses its bound object, and declines a binding to another protocol. FND-002 fixed: one run budget is consumed across keys, cancellation/limits terminate the run, and no later key replaces a refutation. FND-003 fixed: scalar `over` types enumerate finite values. FND-004 fixed: checked-package global conformance uses the package-native protocol program and certificate/replay path. FND-005 fixed: package route now applies the same missing-role refusal. FND-006 fixed: typed terminal values select the result. FND-007 fixed: shared `decisive` selection and a single instance listing are used. FND-008 fixed with paired hand-valued route tests for admitted keys, later refutation, no instance, no initial state, scalar values, missing roles, and shared limits. FND-009: edited Rust files were formatted with rustfmt; workspace-wide `cargo fmt --check` remains unsuitable because unrelated baseline hunks fail. FND-010 fixed: both routes assert the source exit code. FND-011 fixed: declarations moved below imports and `ClaimInstances::first` is used.

Focused verification is pending on the final head; see the PR update for the gate results.

## Gates

Gates, base f8bb04f501 against head cd923f76fb:

| Check | Base | Head | Introduced by this PR | How run |
| --- | --- | --- | --- | --- |
| cargo fmt --all -- --check | fails | fails | 2 hunks added, 1 removed (FND-009) | run on both |
| string-edge | 1169 | 1171 | 2 (FND-006) | installed ~/.local/bin/fsl-string-edge (not rebuilt from the pin) with xtask's crate roots and an empty allow list |
| xtask canonical-types | fails | fails, same output | none | prebuilt xtask from another worktree's target |
| arch-lint api-surface --fsl-only | passes | passes | none | prebuilt arch-lint from another worktree's target |
| arch-lint qualified-core | fails | fails, same findings | none | prebuilt arch-lint |
| route-lint, checked-input, clippy, cargo check, every test | not run | not run | unknown | build lanes stayed above the load limit of 18 for over an hour; run cancelled on the code lead's order |

The PR's "passed" results for tests, clippy, check, route-lint and checked-input are unverified.

## Traceability and answers

FSL spec text and criteria exist (FR-896 'The objects checked over every behaviour', AC-7 to AC-10; FR-990-AC-11; FR-125-AC-13), merged by QSL-3513. NotSimulated is checked before NoInitialState and NoInstance in claim_instances, as ruled. An empty admitted object set settles NoInstance on both routes, by reading.
